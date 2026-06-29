//! The immutable [`WebMercatorViewport`] camera and the [`get_bounds`] helper.

use crate::fit_bounds::{fit_bounds, FitBoundsOptions};
use crate::math::{self, falsy_or_zero, Mat4};
use crate::utils::{
    altitude_to_fovy, fovy_to_altitude, get_distance_scales, get_projection_matrix,
    get_view_matrix, lng_lat_to_world, pixels_to_world, world_to_lng_lat, world_to_pixels,
    zoom_to_scale, DistanceScales, Precision, ProjectionOptions, DEFAULT_ALTITUDE,
};

const DEGREES_TO_RADIANS: f64 = std::f64::consts::PI / 180.0;

/// Construction inputs for [`WebMercatorViewport`].
///
/// All fields except `width` and `height` take defaults when `None`. `width`
/// and `height` of 0 are forced to 1 to avoid division by zero.
#[derive(Debug, Clone, Default)]
pub struct WebMercatorViewportProps {
    /// Viewport width in pixels.
    pub width: f64,
    /// Viewport height in pixels.
    pub height: f64,
    /// Center latitude in degrees. Defaults to 0.
    pub latitude: Option<f64>,
    /// Center longitude in degrees. Defaults to 0.
    pub longitude: Option<f64>,
    /// Camera position offset in meters `[x, y, z]`. Defaults to none.
    pub position: Option<[f64; 3]>,
    /// Zoom level. Defaults to 0.
    pub zoom: Option<f64>,
    /// Camera pitch in degrees. Defaults to 0.
    pub pitch: Option<f64>,
    /// Map bearing in degrees. Defaults to 0.
    pub bearing: Option<f64>,
    /// Camera altitude in screen units. Derived from `fovy` or defaults to 1.5.
    pub altitude: Option<f64>,
    /// Field of view in degrees. Derived from `altitude` when unset.
    pub fovy: Option<f64>,
    /// Near z-buffer multiplier. Defaults to 0.02.
    pub near_z_multiplier: Option<f64>,
    /// Far z-buffer multiplier. Defaults to 1.01.
    pub far_z_multiplier: Option<f64>,
}

/// Options for [`WebMercatorViewport::project`].
#[derive(Debug, Clone, Copy)]
pub struct ProjectOptions {
    /// Return top-left screen coordinates. Defaults to true.
    pub top_left: bool,
}

impl Default for ProjectOptions {
    fn default() -> Self {
        Self { top_left: true }
    }
}

/// Options for [`WebMercatorViewport::unproject`].
#[derive(Debug, Clone, Copy)]
pub struct UnprojectOptions {
    /// Treat input as top-left screen coordinates. Defaults to true.
    pub top_left: bool,
    /// Elevation plane in meters to unproject onto when pixel depth is unknown.
    pub target_z: Option<f64>,
}

impl Default for UnprojectOptions {
    fn default() -> Self {
        Self {
            top_left: true,
            target_z: None,
        }
    }
}

/// A camera that bundles map state and exposes projection helpers.
///
/// Instances are immutable. Build a new viewport when any parameter changes.
/// The stored matrices and distance scales are computed once at construction.
///
/// The fields are public for reading. Construction goes through
/// [`WebMercatorViewport::new`], and the struct is `#[non_exhaustive]` so new
/// computed fields can be added without a breaking change.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct WebMercatorViewport {
    /// Center latitude in degrees, as supplied.
    pub latitude: f64,
    /// Center longitude in degrees, as supplied.
    pub longitude: f64,
    /// Zoom level, as supplied.
    pub zoom: f64,
    /// Camera pitch in degrees.
    pub pitch: f64,
    /// Map bearing in degrees.
    pub bearing: f64,
    /// Camera altitude, after the 0.75 floor.
    pub altitude: f64,
    /// Field of view in degrees.
    pub fovy: f64,
    /// Camera position offset in meters `[x, y, z]`.
    pub meter_offset: [f64; 3],
    /// World-space center `[x, y, z]`.
    pub center: [f64; 3],
    /// Viewport width in pixels.
    pub width: f64,
    /// Viewport height in pixels.
    pub height: f64,
    /// Scale at the current zoom.
    pub scale: f64,
    /// Distance scales around the center latitude.
    pub distance_scales: DistanceScales,
    /// View matrix.
    pub view_matrix: Mat4,
    /// Projection matrix.
    pub projection_matrix: Mat4,
    /// Combined view-projection matrix.
    pub view_projection_matrix: Mat4,
    /// Matrix from world coordinates to screen pixels.
    pub pixel_projection_matrix: Mat4,
    /// Matrix from screen pixels to world coordinates.
    pub pixel_unprojection_matrix: Mat4,
}

impl Default for WebMercatorViewport {
    fn default() -> Self {
        Self::new(&WebMercatorViewportProps {
            width: 1.0,
            height: 1.0,
            ..Default::default()
        })
    }
}

impl WebMercatorViewport {
    /// Builds a viewport from map and camera state.
    ///
    /// `fovy` and `altitude` resolve against each other. With both unset, the
    /// altitude defaults to 1.5 and the fovy follows from it. Altitude is then
    /// floored at 0.75 to keep the view matrix well behaved.
    ///
    /// # Panics
    ///
    /// Panics if the pixel projection matrix is singular and cannot be
    /// inverted.
    #[must_use]
    pub fn new(props: &WebMercatorViewportProps) -> Self {
        let mut width = props.width;
        let mut height = props.height;
        let mut altitude = props.altitude;
        let mut fovy = props.fovy;

        let latitude = props.latitude.unwrap_or(0.0);
        let longitude = props.longitude.unwrap_or(0.0);
        let zoom = props.zoom.unwrap_or(0.0);
        let pitch = props.pitch.unwrap_or(0.0);
        let bearing = props.bearing.unwrap_or(0.0);
        let position = props.position;
        let near_z_multiplier = props.near_z_multiplier.unwrap_or(0.02);
        let far_z_multiplier = props.far_z_multiplier.unwrap_or(1.01);

        // 0 is forced to 1 so isomorphic render paths do not divide by zero.
        if width == 0.0 || width.is_nan() {
            width = 1.0;
        }
        if height == 0.0 || height.is_nan() {
            height = 1.0;
        }

        match (fovy, altitude) {
            (None, None) => {
                let a = DEFAULT_ALTITUDE;
                altitude = Some(a);
                fovy = Some(altitude_to_fovy(a));
            }
            (None, Some(a)) => fovy = Some(altitude_to_fovy(a)),
            (Some(f), None) => altitude = Some(fovy_to_altitude(f)),
            (Some(_), Some(_)) => {}
        }

        let fovy = fovy.expect("fovy resolved");
        let scale = zoom_to_scale(zoom);
        let altitude = altitude.expect("altitude resolved").max(0.75);

        let distance_scales = get_distance_scales(latitude, longitude, Precision::Standard);

        let world = lng_lat_to_world(&[longitude, latitude]);
        let mut center = [world[0], world[1], 0.0];
        if let Some(position) = position {
            for i in 0..3 {
                center[i] += position[i] * distance_scales.units_per_meter[i];
            }
        }

        let projection_matrix = get_projection_matrix(&ProjectionOptions {
            width,
            height,
            scale: Some(scale),
            center: Some(center),
            pitch: Some(pitch),
            fovy: Some(fovy),
            near_z_multiplier: Some(near_z_multiplier),
            far_z_multiplier: Some(far_z_multiplier),
            ..Default::default()
        });

        let view_matrix = get_view_matrix(height, pitch, bearing, altitude, scale, Some(&center));

        let (view_projection_matrix, pixel_projection_matrix, pixel_unprojection_matrix) =
            init_matrices(width, height, &projection_matrix, &view_matrix);

        Self {
            latitude,
            longitude,
            zoom,
            pitch,
            bearing,
            altitude,
            fovy,
            meter_offset: position.unwrap_or([0.0, 0.0, 0.0]),
            center,
            width,
            height,
            scale,
            distance_scales,
            view_matrix,
            projection_matrix,
            view_projection_matrix,
            pixel_projection_matrix,
            pixel_unprojection_matrix,
        }
    }

    /// Reports whether two viewports match within tolerance.
    ///
    /// Width and height must match exactly. The view and projection matrices
    /// must match within a relative tolerance of `1e-6`. The name signals the
    /// tolerant compare and keeps it distinct from `==`, which the type does
    /// not implement.
    #[must_use]
    pub fn approx_eq(&self, other: &WebMercatorViewport) -> bool {
        self.width == other.width
            && self.height == other.height
            && math::mat4_equals(&other.projection_matrix, &self.projection_matrix)
            && math::mat4_equals(&other.view_matrix, &self.view_matrix)
    }

    /// Projects `[lng, lat]` or `[lng, lat, z]` to screen pixels.
    ///
    /// The output length follows the input length. With `top_left` false, the y
    /// coordinate flips about the viewport height.
    #[must_use]
    pub fn project(&self, lng_lat_z: &[f64], options: ProjectOptions) -> Vec<f64> {
        let world_position = self.project_position(lng_lat_z);
        let coord = world_to_pixels(&world_position, &self.pixel_projection_matrix);

        let x = coord[0];
        let y = coord[1];
        let y2 = if options.top_left { y } else { self.height - y };
        if lng_lat_z.len() == 2 {
            vec![x, y2]
        } else {
            vec![x, y2, coord[2]]
        }
    }

    /// Unprojects screen pixels to world coordinates, usually `[lng, lat]`.
    ///
    /// With a finite input z, the output is `[lng, lat, Z]`. Otherwise, with a
    /// finite `target_z`, the output is `[lng, lat, target_z]`. With neither,
    /// the output is `[lng, lat]`.
    #[must_use]
    pub fn unproject(&self, xyz: &[f64], options: UnprojectOptions) -> Vec<f64> {
        let x = xyz[0];
        let y = xyz[1];
        let z = xyz.get(2).copied();

        let y2 = if options.top_left { y } else { self.height - y };

        // `targetZ && targetZ * scale`: only nonzero finite target_z converts
        // meters to world units. Otherwise the falsy value flows through and
        // pixels_to_world re-coerces it to 0.
        let target_z_world = match options.target_z {
            Some(tz) if tz != 0.0 && !tz.is_nan() => tz * self.distance_scales.units_per_meter[2],
            Some(tz) => tz,
            None => 0.0,
        };

        let mut pixel = vec![x, y2];
        if let Some(z) = z {
            pixel.push(z);
        } else {
            pixel.push(f64::NAN);
        }
        let coord = pixels_to_world(&pixel, &self.pixel_unprojection_matrix, target_z_world);
        let [big_x, big_y, big_z] = self.unproject_position(&coord);

        if z.is_some_and(f64::is_finite) {
            return vec![big_x, big_y, big_z];
        }
        match options.target_z {
            Some(tz) if tz.is_finite() => vec![big_x, big_y, tz],
            _ => vec![big_x, big_y],
        }
    }

    /// Projects a geographic position to world coordinates with a z meter scale.
    #[must_use]
    pub fn project_position(&self, xyz: &[f64]) -> [f64; 3] {
        let xy = lng_lat_to_world(xyz);
        let z = falsy_or_zero(xyz.get(2).copied().unwrap_or(0.0))
            * self.distance_scales.units_per_meter[2];
        [xy[0], xy[1], z]
    }

    /// Unprojects world coordinates to a geographic position with a z meter
    /// scale.
    #[must_use]
    pub fn unproject_position(&self, xyz: &[f64]) -> [f64; 3] {
        let lng_lat = world_to_lng_lat(xyz);
        let z = falsy_or_zero(xyz.get(2).copied().unwrap_or(0.0))
            * self.distance_scales.meters_per_unit[2];
        [lng_lat[0], lng_lat[1], z]
    }

    /// Projects `[lng, lat]` onto the flat zoom-0 tile.
    #[must_use]
    pub fn project_flat(&self, lng_lat: &[f64]) -> [f64; 2] {
        lng_lat_to_world(lng_lat)
    }

    /// Unprojects a flat tile point back to `[lng, lat]`.
    #[must_use]
    pub fn unproject_flat(&self, xy: &[f64]) -> [f64; 2] {
        world_to_lng_lat(xy)
    }

    /// Returns the map center that places `lng_lat` at screen point `pos`.
    #[must_use]
    pub fn get_map_center_by_lng_lat_position(&self, lng_lat: &[f64], pos: &[f64]) -> [f64; 2] {
        let from_location = pixels_to_world(pos, &self.pixel_unprojection_matrix, 0.0);
        let to_location = lng_lat_to_world(lng_lat);
        let translate = [
            to_location[0] - from_location[0],
            to_location[1] - from_location[1],
        ];
        let new_center = [self.center[0] + translate[0], self.center[1] + translate[1]];
        world_to_lng_lat(&new_center)
    }

    /// Deprecated alias for [`Self::get_map_center_by_lng_lat_position`].
    #[deprecated(note = "use get_map_center_by_lng_lat_position")]
    #[must_use]
    pub fn get_location_at_point(&self, lng_lat: &[f64], pos: &[f64]) -> [f64; 2] {
        self.get_map_center_by_lng_lat_position(lng_lat, pos)
    }

    /// Returns a new viewport that frames `bounds`. Non-perspective only.
    #[must_use]
    pub fn fit_bounds(
        &self,
        bounds: [[f64; 2]; 2],
        options: &FitBoundsOptions,
    ) -> WebMercatorViewport {
        let mut opts = options.clone();
        opts.width = self.width;
        opts.height = self.height;
        opts.bounds = bounds;
        let result = fit_bounds(&opts);
        WebMercatorViewport::new(&WebMercatorViewportProps {
            width: self.width,
            height: self.height,
            longitude: Some(result.longitude),
            latitude: Some(result.latitude),
            zoom: Some(result.zoom),
            ..Default::default()
        })
    }

    /// Returns the viewport bounds as `[[west, south], [east, north]]`.
    #[must_use]
    pub fn get_bounds(&self, z: f64) -> [[f64; 2]; 2] {
        let corners = self.get_bounding_region(z);
        let west = corners.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
        let east = corners
            .iter()
            .map(|p| p[0])
            .fold(f64::NEG_INFINITY, f64::max);
        let south = corners.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min);
        let north = corners
            .iter()
            .map(|p| p[1])
            .fold(f64::NEG_INFINITY, f64::max);
        [[west, south], [east, north]]
    }

    /// Returns the four corner points of the visible region at elevation `z`.
    ///
    /// Each corner is `[lng, lat, z]`. The order is
    /// `[bottom_left, bottom_right, top_right, top_left]`.
    #[must_use]
    pub fn get_bounding_region(&self, z: f64) -> [[f64; 3]; 4] {
        get_bounds(self, z)
    }
}

/// Builds the view-projection, pixel-projection, and pixel-unprojection
/// matrices.
fn init_matrices(
    width: f64,
    height: f64,
    projection_matrix: &Mat4,
    view_matrix: &Mat4,
) -> (Mat4, Mat4, Mat4) {
    let vpm = math::multiply(&math::identity(), projection_matrix);
    let vpm = math::multiply(&vpm, view_matrix);

    let m = math::identity();
    let m = math::scale(&m, [width / 2.0, -height / 2.0, 1.0]);
    let m = math::translate(&m, [1.0, -1.0, 0.0]);
    let m = math::multiply(&m, &vpm);

    let m_inverse = math::invert(&m).expect("Pixel project matrix not invertible");
    (vpm, m, m_inverse)
}

/// Returns the quad where the view frustum meets the `z` elevation plane.
///
/// The order is `[bottom_left, bottom_right, top_right, top_left]`. Each corner
/// is `[lng, lat, z]`. When the top frustum plane runs near parallel to the
/// ground, the top corners come from the far plane instead.
#[must_use]
pub fn get_bounds(viewport: &WebMercatorViewport, z: f64) -> [[f64; 3]; 4] {
    let width = viewport.width;
    let height = viewport.height;
    let ops = UnprojectOptions {
        top_left: true,
        target_z: Some(z),
    };

    // A finite target_z makes unproject return a length-3 `[lng, lat, z]`.
    let bottom_left = corner(&viewport.unproject(&[0.0, height], ops));
    let bottom_right = corner(&viewport.unproject(&[width, height], ops));

    let half_fov = if viewport.fovy != 0.0 {
        0.5 * viewport.fovy * DEGREES_TO_RADIANS
    } else {
        (0.5 / viewport.altitude).atan()
    };
    let angle_to_ground = (90.0 - viewport.pitch) * DEGREES_TO_RADIANS;

    let (top_left, top_right) = if half_fov > angle_to_ground - 0.01 {
        (
            unproject_on_far_plane(viewport, 0.0, z),
            unproject_on_far_plane(viewport, width, z),
        )
    } else {
        (
            corner(&viewport.unproject(&[0.0, 0.0], ops)),
            corner(&viewport.unproject(&[width, 0.0], ops)),
        )
    };

    [bottom_left, bottom_right, top_right, top_left]
}

/// Reads the first three elements of a `[lng, lat, z]` corner.
fn corner(point: &[f64]) -> [f64; 3] {
    [point[0], point[1], point[2]]
}

/// Finds a point on the far clipping plane at screen x and elevation `target_z`.
fn unproject_on_far_plane(viewport: &WebMercatorViewport, x: f64, target_z: f64) -> [f64; 3] {
    let matrix = &viewport.pixel_unprojection_matrix;
    let coord0 = math::transform_vector(matrix, [x, 0.0, 1.0, 1.0]);
    let coord1 = math::transform_vector(matrix, [x, viewport.height, 1.0, 1.0]);

    let z = target_z * viewport.distance_scales.units_per_meter[2];
    let t = (z - coord0[2]) / (coord1[2] - coord0[2]);
    let coord = [
        math::lerp(coord0[0], coord1[0], t),
        math::lerp(coord0[1], coord1[1], t),
    ];

    let result = world_to_lng_lat(&coord);
    [result[0], result[1], target_z]
}

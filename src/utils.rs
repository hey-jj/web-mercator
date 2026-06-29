//! Web Mercator projection and camera math.
//!
//! Free functions that convert between geographic coordinates, world pixels on a
//! 512x512 zoom-0 tile, screen pixels, meters, zoom, scale, altitude, and field
//! of view. They also build the 4x4 view and projection matrices that match the
//! Mapbox GL camera model.
//!
//! Angles are degrees at the public boundary and radians inside. World
//! coordinates are pixels on the zoom-0 tile.

use crate::math::{self, falsy_or_zero, Mat4, ASSERT_MESSAGE};

const PI: f64 = std::f64::consts::PI;
const PI_4: f64 = PI / 4.0;
const DEGREES_TO_RADIANS: f64 = PI / 180.0;
const RADIANS_TO_DEGREES: f64 = 180.0 / PI;
const TILE_SIZE: f64 = 512.0;

/// Average earth circumference in meters. Mean of the 40075 km equatorial and
/// 40007 km meridional circumferences.
const EARTH_CIRCUMFERENCE: f64 = 40.03e6;

/// Latitude that makes the projected world square: `2 * atan(e^pi) - pi/2`.
pub const MAX_LATITUDE: f64 = 85.051129;

/// Default camera altitude in screen units, matching Mapbox GL.
pub(crate) const DEFAULT_ALTITUDE: f64 = 1.5;

/// Selects whether [`get_distance_scales`] includes the second-order Taylor
/// terms.
///
/// `Standard` returns first-order scales only. `High` adds the `*2` correction
/// terms that account for the `1/cos(lat)` curvature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Precision {
    /// First-order scales. The `*2` second-order fields are `None`.
    Standard,
    /// First- and second-order scales. The `*2` fields are populated.
    High,
}

/// Per-degree and per-meter world-unit scales around a latitude.
///
/// All vectors have length 3. The `*2` second-order Taylor terms are present
/// only when high precision is requested.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct DistanceScales {
    /// World units per meter, one value repeated for x, y, z.
    pub units_per_meter: [f64; 3],
    /// Meters per world unit.
    pub meters_per_unit: [f64; 3],
    /// Second-order term for units per meter. Present only in high precision.
    pub units_per_meter2: Option<[f64; 3]>,
    /// World units per degree for x, y, and the z meter scale.
    pub units_per_degree: [f64; 3],
    /// Degrees per world unit.
    pub degrees_per_unit: [f64; 3],
    /// Second-order term for units per degree. Present only in high precision.
    pub units_per_degree2: Option<[f64; 3]>,
}

/// Parameters for a Mapbox-compatible perspective projection.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct ProjectionParameters {
    /// Field of view in radians.
    pub fov: f64,
    /// Aspect ratio, width over height.
    pub aspect: f64,
    /// Distance at which the visual scale factor is 1.
    pub focal_distance: f64,
    /// Near clipping plane.
    pub near: f64,
    /// Far clipping plane.
    pub far: f64,
}

/// Converts a logarithmic zoom to a linear scale: `2^zoom`.
#[must_use]
pub fn zoom_to_scale(zoom: f64) -> f64 {
    2.0_f64.powf(zoom)
}

/// Converts a linear scale to a logarithmic zoom: `log2(scale)`.
#[must_use]
pub fn scale_to_zoom(scale: f64) -> f64 {
    scale.log2()
}

/// Projects `[lng, lat]` in degrees onto `[x, y]` on the 512x512 zoom-0 tile.
///
/// This is the nonlinear part of the Web Mercator projection. The remaining
/// projection uses 4x4 matrices that also handle perspective.
///
/// # Panics
///
/// Panics if `lng_lat` has fewer than two elements. Panics if `lng` is not
/// finite. Panics with `"invalid latitude"` if `lat` is not finite or lies
/// outside `[-90, 90]`.
#[must_use]
pub fn lng_lat_to_world(lng_lat: &[f64]) -> [f64; 2] {
    let lng = lng_lat[0];
    let lat = lng_lat[1];
    assert!(lng.is_finite(), "{ASSERT_MESSAGE}");
    assert!(
        lat.is_finite() && (-90.0..=90.0).contains(&lat),
        "invalid latitude"
    );

    let lambda2 = lng * DEGREES_TO_RADIANS;
    let phi2 = lat * DEGREES_TO_RADIANS;
    let x = (TILE_SIZE * (lambda2 + PI)) / (2.0 * PI);
    let y = (TILE_SIZE * (PI + (PI_4 + phi2 * 0.5).tan().ln())) / (2.0 * PI);
    [x, y]
}

/// Unprojects a world point `[x, y]` back to `[lng, lat]` in degrees.
///
/// # Panics
///
/// Panics if `xy` has fewer than two elements.
#[must_use]
pub fn world_to_lng_lat(xy: &[f64]) -> [f64; 2] {
    let x = xy[0];
    let y = xy[1];
    let lambda2 = (x / TILE_SIZE) * (2.0 * PI) - PI;
    let phi2 = 2.0 * (((y / TILE_SIZE) * (2.0 * PI) - PI).exp().atan() - PI_4);
    [lambda2 * RADIANS_TO_DEGREES, phi2 * RADIANS_TO_DEGREES]
}

/// Returns the zoom level that yields one world unit per meter at `latitude`.
///
/// # Panics
///
/// Panics if `latitude` is not finite.
#[must_use]
pub fn get_meter_zoom(latitude: f64) -> f64 {
    assert!(latitude.is_finite(), "{ASSERT_MESSAGE}");
    let lat_cosine = (latitude * DEGREES_TO_RADIANS).cos();
    scale_to_zoom(EARTH_CIRCUMFERENCE * lat_cosine) - 9.0
}

/// Returns the world units per meter at `latitude`.
///
/// Cheaper alternative to [`get_distance_scales`] when only the meter scale is
/// needed.
#[must_use]
pub fn units_per_meter(latitude: f64) -> f64 {
    let lat_cosine = (latitude * DEGREES_TO_RADIANS).cos();
    TILE_SIZE / EARTH_CIRCUMFERENCE / lat_cosine
}

/// Computes per-degree and per-meter world-unit scales around a latitude.
///
/// The output depends only on `latitude`. `longitude` is read solely for the
/// finiteness check. Pass [`Precision::High`] to include the second-order
/// Taylor terms that correct the `1/cos(lat)` nonlinearity.
///
/// # Panics
///
/// Panics if either `latitude` or `longitude` is not finite.
#[must_use]
pub fn get_distance_scales(latitude: f64, longitude: f64, precision: Precision) -> DistanceScales {
    assert!(
        latitude.is_finite() && longitude.is_finite(),
        "{ASSERT_MESSAGE}"
    );

    let world_size = TILE_SIZE;
    let lat_cosine = (latitude * DEGREES_TO_RADIANS).cos();

    let units_per_degree_x = world_size / 360.0;
    let units_per_degree_y = units_per_degree_x / lat_cosine;
    let alt_units_per_meter = world_size / EARTH_CIRCUMFERENCE / lat_cosine;

    let mut result = DistanceScales {
        units_per_meter: [alt_units_per_meter; 3],
        meters_per_unit: [1.0 / alt_units_per_meter; 3],
        units_per_meter2: None,
        units_per_degree: [units_per_degree_x, units_per_degree_y, alt_units_per_meter],
        degrees_per_unit: [
            1.0 / units_per_degree_x,
            1.0 / units_per_degree_y,
            1.0 / alt_units_per_meter,
        ],
        units_per_degree2: None,
    };

    if precision == Precision::High {
        let lat_cosine2 = (DEGREES_TO_RADIANS * (latitude * DEGREES_TO_RADIANS).tan()) / lat_cosine;
        let units_per_degree_y2 = (units_per_degree_x * lat_cosine2) / 2.0;
        let alt_units_per_degree2 = (world_size / EARTH_CIRCUMFERENCE) * lat_cosine2;
        let alt_units_per_meter2 =
            (alt_units_per_degree2 / units_per_degree_y) * alt_units_per_meter;

        result.units_per_degree2 = Some([0.0, units_per_degree_y2, alt_units_per_degree2]);
        result.units_per_meter2 = Some([alt_units_per_meter2, 0.0, alt_units_per_meter2]);
    }

    result
}

/// Offsets a geographic position by a meter offset.
///
/// `lng_lat_z` is `[lng, lat]` or `[lng, lat, z]`. `xyz` is the meter offset
/// `[east, north, up]`. The result keeps a z component when either input z or
/// offset z is finite, otherwise it is length 2.
///
/// # Panics
///
/// Panics if `lng_lat_z` has fewer than two elements or `xyz` has fewer than
/// two elements. Panics through [`lng_lat_to_world`] if the latitude is
/// invalid.
#[must_use]
pub fn add_meters_to_lng_lat(lng_lat_z: &[f64], xyz: &[f64]) -> Vec<f64> {
    let longitude = lng_lat_z[0];
    let latitude = lng_lat_z[1];
    let z0 = lng_lat_z.get(2).copied();
    let x = xyz[0];
    let y = xyz[1];
    let z = xyz.get(2).copied();

    let scales = get_distance_scales(latitude, longitude, Precision::High);
    let units_per_meter = scales.units_per_meter;
    let units_per_meter2 = scales.units_per_meter2.expect("high precision requested");

    let mut worldspace = lng_lat_to_world(lng_lat_z);
    worldspace[0] += x * (units_per_meter[0] + units_per_meter2[0] * y);
    worldspace[1] += y * (units_per_meter[1] + units_per_meter2[1] * y);

    let new_lng_lat = world_to_lng_lat(&worldspace);
    let new_z = falsy_or_zero(z0.unwrap_or(0.0)) + falsy_or_zero(z.unwrap_or(0.0));

    if is_finite_opt(z0) || is_finite_opt(z) {
        vec![new_lng_lat[0], new_lng_lat[1], new_z]
    } else {
        vec![new_lng_lat[0], new_lng_lat[1]]
    }
}

/// Builds a Mapbox-compatible view matrix.
///
/// Operations apply to the identity in the listed order. Read them in reverse,
/// since vectors multiply in from the right during transformation. `center` is
/// optional. When absent, the final translate is skipped.
#[must_use]
pub fn get_view_matrix(
    height: f64,
    pitch: f64,
    bearing: f64,
    altitude: f64,
    scale: f64,
    center: Option<&[f64]>,
) -> Mat4 {
    let mut vm = math::identity();
    vm = math::translate(&vm, [0.0, 0.0, -altitude]);
    vm = math::rotate_x(&vm, -pitch * DEGREES_TO_RADIANS);
    vm = math::rotate_z(&vm, bearing * DEGREES_TO_RADIANS);

    let relative_scale = scale / height;
    vm = math::scale(&vm, [relative_scale, relative_scale, relative_scale]);

    if let Some(c) = center {
        vm = math::translate(&vm, [-c[0], -c[1], -c[2]]);
    }
    vm
}

/// Inputs for the projection-parameter and projection-matrix functions.
///
/// Optional fields take Mapbox defaults when `None`. `near_z_multiplier` and
/// `far_z_multiplier` default to `1.0` here.
#[derive(Debug, Clone, Default)]
pub struct ProjectionOptions {
    /// Viewport width in pixels.
    pub width: f64,
    /// Viewport height in pixels.
    pub height: f64,
    /// Scale at the current zoom.
    pub scale: Option<f64>,
    /// Target offset, a vec3 in world space.
    pub center: Option<[f64; 3]>,
    /// Focal-point offset, a vec2 in screen space.
    pub offset: Option<[f64; 2]>,
    /// Field of view in degrees.
    pub fovy: Option<f64>,
    /// Camera altitude. Overrides `fovy` via `altitude_to_fovy` when set.
    pub altitude: Option<f64>,
    /// Camera pitch in degrees. Defaults to 0.
    pub pitch: Option<f64>,
    /// Near z-buffer multiplier. Defaults to 1.
    pub near_z_multiplier: Option<f64>,
    /// Far z-buffer multiplier. Defaults to 1.
    pub far_z_multiplier: Option<f64>,
}

/// Computes projection parameters from camera options.
///
/// `near` is the raw near multiplier. `far` is the furthest renderable distance,
/// clamped to ten times the camera-to-sea-level distance to match the Mapbox
/// horizon limit.
#[must_use]
pub fn get_projection_parameters(options: &ProjectionOptions) -> ProjectionParameters {
    let width = options.width;
    let height = options.height;
    let pitch = options.pitch.unwrap_or(0.0);
    let near_z_multiplier = options.near_z_multiplier.unwrap_or(1.0);
    let far_z_multiplier = options.far_z_multiplier.unwrap_or(1.0);

    let mut fovy = options
        .fovy
        .unwrap_or_else(|| altitude_to_fovy(DEFAULT_ALTITUDE));
    if let Some(altitude) = options.altitude {
        fovy = altitude_to_fovy(altitude);
    }

    let fov_radians = fovy * DEGREES_TO_RADIANS;
    let pitch_radians = pitch * DEGREES_TO_RADIANS;

    let focal_distance = fovy_to_altitude(fovy);
    let mut camera_to_sea_level_distance = focal_distance;

    if let Some(center) = options.center {
        // The term reads `center[2] * scale` directly. With `scale` absent it
        // is NaN and flows through the result. No panic.
        let scale = options.scale.unwrap_or(f64::NAN);
        camera_to_sea_level_distance += (center[2] * scale) / pitch_radians.cos() / height;
    }

    let offset_y = options.offset.map_or(0.0, |o| o[1]);
    let fov_above_center = fov_radians * (0.5 + offset_y / height);

    let top_half_surface_distance = (fov_above_center.sin() * camera_to_sea_level_distance)
        / math::clamp(PI / 2.0 - pitch_radians - fov_above_center, 0.01, PI - 0.01).sin();

    let furthest_distance =
        pitch_radians.sin() * top_half_surface_distance + camera_to_sea_level_distance;
    let horizon_distance = camera_to_sea_level_distance * 10.0;

    let far_z = (furthest_distance * far_z_multiplier).min(horizon_distance);

    ProjectionParameters {
        fov: fov_radians,
        aspect: width / height,
        focal_distance,
        near: near_z_multiplier,
        far: far_z,
    }
}

/// Builds a Mapbox-compatible perspective projection matrix.
#[must_use]
pub fn get_projection_matrix(options: &ProjectionOptions) -> Mat4 {
    let params = get_projection_parameters(options);
    math::perspective(params.fov, params.aspect, params.near, params.far)
}

/// Converts an altitude to the field of view in degrees whose focal distance
/// equals that altitude.
#[must_use]
pub fn altitude_to_fovy(altitude: f64) -> f64 {
    2.0 * (0.5 / altitude).atan() * RADIANS_TO_DEGREES
}

/// Converts a field of view in degrees to the altitude whose focal distance
/// equals it.
#[must_use]
pub fn fovy_to_altitude(fovy: f64) -> f64 {
    0.5 / (0.5 * fovy * DEGREES_TO_RADIANS).tan()
}

/// Projects a flat world coordinate to screen pixels.
///
/// `xyz` is `[x, y]` or `[x, y, z]`. A missing z is treated as 0. Returns the
/// homogeneous-divided 4-vector `[x, y, depth, 1]`.
///
/// # Panics
///
/// Panics if `xyz` has fewer than two elements. Panics if any of x, y, or z is
/// not finite.
#[must_use]
pub fn world_to_pixels(xyz: &[f64], pixel_projection_matrix: &Mat4) -> [f64; 4] {
    let x = xyz[0];
    let y = xyz[1];
    let z = xyz.get(2).copied().unwrap_or(0.0);
    assert!(
        x.is_finite() && y.is_finite() && z.is_finite(),
        "{ASSERT_MESSAGE}"
    );
    math::transform_vector(pixel_projection_matrix, [x, y, z, 1.0])
}

/// Unprojects screen pixels to flat world coordinates.
///
/// When `xyz` carries a finite z, the result is the 4-vector at that depth. When
/// z is absent or not finite, the function unprojects at depth 0 and depth 1,
/// then interpolates to the plane where world z equals `target_z`, returning the
/// length-2 point `[x, y]`.
///
/// # Panics
///
/// Panics if `xyz` has fewer than two elements. Panics with
/// `"invalid pixel coordinate"` if x or y is not finite.
#[must_use]
pub fn pixels_to_world(xyz: &[f64], pixel_unprojection_matrix: &Mat4, target_z: f64) -> Vec<f64> {
    let x = xyz[0];
    let y = xyz[1];
    let z = xyz.get(2).copied();
    assert!(x.is_finite() && y.is_finite(), "invalid pixel coordinate");

    if let Some(z) = z {
        if z.is_finite() {
            let coord = math::transform_vector(pixel_unprojection_matrix, [x, y, z, 1.0]);
            return coord.to_vec();
        }
    }

    let coord0 = math::transform_vector(pixel_unprojection_matrix, [x, y, 0.0, 1.0]);
    let coord1 = math::transform_vector(pixel_unprojection_matrix, [x, y, 1.0, 1.0]);

    let z0 = coord0[2];
    let z1 = coord1[2];

    let t = if z0 == z1 {
        0.0
    } else {
        (falsy_or_zero(target_z) - z0) / (z1 - z0)
    };
    vec![
        math::lerp(coord0[0], coord1[0], t),
        math::lerp(coord0[1], coord1[1], t),
    ]
}

/// Mirrors `Number.isFinite` on an optional input. Absent is not finite.
fn is_finite_opt(value: Option<f64>) -> bool {
    value.is_some_and(f64::is_finite)
}

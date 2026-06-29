//! Clamp a viewport so the map fills it and angles wrap into legal ranges.

use crate::math::mod_floor;
use crate::utils::{scale_to_zoom, world_to_lng_lat};

const TILE_SIZE: f64 = 512.0;

/// A viewport state. `pitch` and `bearing` default to 0 when unset.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportProps {
    /// Viewport width in pixels.
    pub width: f64,
    /// Viewport height in pixels.
    pub height: f64,
    /// Center longitude in degrees.
    pub longitude: f64,
    /// Center latitude in degrees.
    pub latitude: f64,
    /// Zoom level.
    pub zoom: f64,
    /// Camera pitch in degrees. `None` means 0.
    pub pitch: Option<f64>,
    /// Map bearing in degrees. `None` means 0.
    pub bearing: Option<f64>,
}

/// Applies the mathematical constraints to a viewport.
///
/// Longitude and bearing wrap into `[-180, 180]` only when outside it. Zoom is
/// raised to the minimum that fills the height, and latitude is clamped so no
/// blank space shows above or below the map. The returned `pitch` and `bearing`
/// are always concrete, defaulting to 0.
#[must_use]
pub fn normalize_viewport_props(props: &ViewportProps) -> ViewportProps {
    let width = props.width;
    let height = props.height;
    let pitch = props.pitch.unwrap_or(0.0);
    let mut longitude = props.longitude;
    let mut latitude = props.latitude;
    let mut zoom = props.zoom;
    let mut bearing = props.bearing.unwrap_or(0.0);

    if !(-180.0..=180.0).contains(&longitude) {
        longitude = mod_floor(longitude + 180.0, 360.0) - 180.0;
    }
    if !(-180.0..=180.0).contains(&bearing) {
        bearing = mod_floor(bearing + 180.0, 360.0) - 180.0;
    }

    let min_zoom = scale_to_zoom(height / TILE_SIZE);
    if zoom <= min_zoom {
        zoom = min_zoom;
        latitude = 0.0;
    } else {
        let half_height_pixels = height / 2.0 / 2.0_f64.powf(zoom);
        let min_latitude = world_to_lng_lat(&[0.0, half_height_pixels])[1];
        if latitude < min_latitude {
            latitude = min_latitude;
        } else {
            let max_latitude = world_to_lng_lat(&[0.0, TILE_SIZE - half_height_pixels])[1];
            if latitude > max_latitude {
                latitude = max_latitude;
            }
        }
    }

    ViewportProps {
        width,
        height,
        longitude,
        latitude,
        zoom,
        pitch: Some(pitch),
        bearing: Some(bearing),
    }
}

//! Web Mercator projection and a perspective map viewport that match the
//! Mapbox GL and deck.gl camera math.
//!
//! The crate has two layers. Free functions convert between geographic
//! coordinates, world pixels on a 512x512 zoom-0 tile, screen pixels, meters,
//! zoom, scale, altitude, and field of view, and build the 4x4 view and
//! projection matrices. The [`WebMercatorViewport`] type bundles a full camera
//! state and exposes `project`, `unproject`, `fit_bounds`, and `get_bounds`.
//!
//! Three standalone camera utilities round it out: [`fit_bounds`] frames a
//! bounding box, [`fly_to_viewport`] and [`get_fly_to_duration`] interpolate a
//! smooth pan and zoom, and [`normalize_viewport_props`] clamps a viewport to
//! the legal latitude, longitude, and zoom range.
//!
//! Angles are degrees at the public boundary and radians inside. World
//! coordinates are pixels on the zoom-0 tile. All math runs in `f64`, and
//! matrices are column-major length-16 arrays.
//!
//! # Example
//!
//! ```
//! use web_mercator_viewport::{WebMercatorViewport, WebMercatorViewportProps};
//!
//! let viewport = WebMercatorViewport::new(&WebMercatorViewportProps {
//!     width: 800.0,
//!     height: 600.0,
//!     longitude: Some(-122.43),
//!     latitude: Some(37.75),
//!     zoom: Some(11.5),
//!     pitch: Some(30.0),
//!     ..Default::default()
//! });
//!
//! let screen = viewport.project(&[-122.43, 37.75], Default::default());
//! assert!((screen[0] - 400.0).abs() < 1e-6);
//! assert!((screen[1] - 300.0).abs() < 1e-6);
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod fit_bounds;
mod fly_to;
mod math;
mod normalize;
mod utils;
mod viewport;

pub use fit_bounds::{fit_bounds, FitBoundsOptions, FitBoundsResult, Padding, PaddingOption};
pub use fly_to::{fly_to_viewport, get_fly_to_duration, FlyToOptions, FlyToResult};
pub use normalize::{normalize_viewport_props, ViewportProps};
pub use utils::{
    add_meters_to_lng_lat, altitude_to_fovy, fovy_to_altitude, get_distance_scales, get_meter_zoom,
    get_projection_matrix, get_projection_parameters, get_view_matrix, lng_lat_to_world,
    pixels_to_world, scale_to_zoom, units_per_meter, world_to_lng_lat, world_to_pixels,
    zoom_to_scale, DistanceScales, Precision, ProjectionOptions, ProjectionParameters,
    MAX_LATITUDE,
};
pub use viewport::{
    get_bounds, ProjectOptions, UnprojectOptions, WebMercatorViewport, WebMercatorViewportProps,
};

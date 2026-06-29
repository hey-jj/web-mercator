//! Smooth pan and zoom interpolation from van Wijk and Nuij (2003).
//!
//! Implements "Smooth and Efficient Zooming and Panning". Equation numbers in
//! the comments refer to that paper, matching the Mapbox GL `flyTo` behavior.

use crate::math::lerp;
use crate::normalize::ViewportProps;
use crate::utils::{lng_lat_to_world, scale_to_zoom, world_to_lng_lat, zoom_to_scale};

const EPSILON: f64 = 0.01;
const DEFAULT_CURVE: f64 = 1.414;
const DEFAULT_SPEED: f64 = 1.2;

/// Tuning options for a fly-to transition.
///
/// `curve` defaults to 1.414 and `speed` to 1.2. `screen_speed` and
/// `max_duration` apply only when set.
#[derive(Debug, Clone, Copy, Default)]
pub struct FlyToOptions {
    /// Path curvature, the rho parameter. Defaults to 1.414.
    pub curve: Option<f64>,
    /// Movement speed. Defaults to 1.2.
    pub speed: Option<f64>,
    /// Screen-space speed. Takes precedence over `speed` when set.
    pub screen_speed: Option<f64>,
    /// Cap on duration. A longer transition returns 0 from [`get_fly_to_duration`].
    pub max_duration: Option<f64>,
}

/// Interpolated viewport at a transition parameter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlyToResult {
    /// Center longitude in degrees.
    pub longitude: f64,
    /// Center latitude in degrees.
    pub latitude: f64,
    /// Zoom level.
    pub zoom: f64,
}

struct TransitionParams {
    start_zoom: f64,
    start_center_xy: [f64; 2],
    u_delta: [f64; 2],
    w0: f64,
    u1: f64,
    s_total: f64,
    rho: f64,
    rho2: f64,
    r0: f64,
}

/// Interpolates a viewport along the smooth fly-to path at `t` in `[0, 1]`.
///
/// When the pan distance is tiny, the function falls back to a linear
/// interpolation of longitude, latitude, and zoom.
#[must_use]
pub fn fly_to_viewport(
    start_props: &ViewportProps,
    end_props: &ViewportProps,
    t: f64,
    options: &FlyToOptions,
) -> FlyToResult {
    let p = transition_params(start_props, end_props, options);

    if p.u1 < EPSILON {
        return FlyToResult {
            longitude: lerp(start_props.longitude, end_props.longitude, t),
            latitude: lerp(start_props.latitude, end_props.latitude, t),
            zoom: lerp(start_props.zoom, end_props.zoom, t),
        };
    }

    let s = t * p.s_total;

    let w = p.r0.cosh() / (p.r0 + p.rho * s).cosh();
    let u = (p.w0 * ((p.r0.cosh() * (p.r0 + p.rho * s).tanh() - p.r0.sinh()) / p.rho2)) / p.u1;

    let scale_increment = 1.0 / w;
    let new_zoom = p.start_zoom + scale_to_zoom(scale_increment);

    let new_center_world = [
        p.u_delta[0] * u + p.start_center_xy[0],
        p.u_delta[1] * u + p.start_center_xy[1],
    ];
    let new_center = world_to_lng_lat(&new_center_world);

    FlyToResult {
        longitude: new_center[0],
        latitude: new_center[1],
        zoom: new_zoom,
    }
}

/// Returns the transition duration in milliseconds.
///
/// `screen_speed` takes precedence over `speed`. When `max_duration` is set and
/// exceeded, the function returns 0 to signal "skip the animation".
#[must_use]
pub fn get_fly_to_duration(
    start_props: &ViewportProps,
    end_props: &ViewportProps,
    options: &FlyToOptions,
) -> f64 {
    let speed = options.speed.unwrap_or(DEFAULT_SPEED);
    let p = transition_params(start_props, end_props, options);
    let length = 1000.0 * p.s_total;

    let duration = match options.screen_speed {
        Some(screen_speed) if screen_speed.is_finite() => length / (screen_speed / p.rho),
        _ => length / speed,
    };

    match options.max_duration {
        Some(max_duration) if max_duration.is_finite() && duration > max_duration => 0.0,
        _ => duration,
    }
}

/// Computes the parameters that stay fixed for a start and end pair.
fn transition_params(
    start_props: &ViewportProps,
    end_props: &ViewportProps,
    options: &FlyToOptions,
) -> TransitionParams {
    let rho = options.curve.unwrap_or(DEFAULT_CURVE);
    let start_zoom = start_props.zoom;
    let start_center = [start_props.longitude, start_props.latitude];
    let start_scale = zoom_to_scale(start_zoom);
    let end_zoom = end_props.zoom;
    let end_center = [end_props.longitude, end_props.latitude];
    let scale = zoom_to_scale(end_zoom - start_zoom);

    let start_center_xy = lng_lat_to_world(&start_center);
    let end_center_xy = lng_lat_to_world(&end_center);
    let u_delta = [
        end_center_xy[0] - start_center_xy[0],
        end_center_xy[1] - start_center_xy[1],
    ];

    let w0 = start_props.width.max(start_props.height);
    let w1 = w0 / scale;
    let u1 = (u_delta[0] * u_delta[0] + u_delta[1] * u_delta[1]).sqrt() * start_scale;
    let u1_guarded = u1.max(EPSILON);

    let rho2 = rho * rho;
    let b0 = (w1 * w1 - w0 * w0 + rho2 * rho2 * u1_guarded * u1_guarded)
        / (2.0 * w0 * rho2 * u1_guarded);
    let b1 = (w1 * w1 - w0 * w0 - rho2 * rho2 * u1_guarded * u1_guarded)
        / (2.0 * w1 * rho2 * u1_guarded);
    let r0 = ((b0 * b0 + 1.0).sqrt() - b0).ln();
    let r1 = ((b1 * b1 + 1.0).sqrt() - b1).ln();
    let s_total = (r1 - r0) / rho;

    TransitionParams {
        start_zoom,
        start_center_xy,
        u_delta,
        w0,
        u1,
        s_total,
        rho,
        rho2,
        r0,
    }
}

//! Shared helpers for the conformance tests.
//!
//! Two float-comparison modes back the golden checks: an element-wise tolerance
//! compare and a low-precision rounding compare.

#![allow(dead_code)]

use web_mercator::WebMercatorViewportProps;

/// Rounds a number to a low precision for golden comparison.
///
/// Values with magnitude above 1 round to `precision` significant digits, like
/// `Number.toPrecision`. Smaller values round to `precision` decimal places,
/// like `Number.toFixed`.
#[must_use]
pub fn to_low_precision(x: f64, precision: i32) -> f64 {
    if !x.is_finite() {
        return x;
    }
    if x.abs() > 1.0 {
        to_precision(x, precision)
    } else {
        to_fixed(x, precision)
    }
}

/// Rounds with the default precision of 7, matching the suite default.
#[must_use]
pub fn lp(x: f64) -> f64 {
    to_low_precision(x, 7)
}

/// Replicates `Number.prototype.toPrecision` followed by `Number(...)`.
fn to_precision(x: f64, precision: i32) -> f64 {
    if x == 0.0 {
        return 0.0;
    }
    let digits = x.abs().log10().floor() as i32 + 1;
    let factor = 10f64.powi(precision - digits);
    (x * factor).round() / factor
}

/// Replicates `Number.prototype.toFixed` followed by `Number(...)`.
fn to_fixed(x: f64, precision: i32) -> f64 {
    let factor = 10f64.powi(precision);
    (x * factor).round() / factor
}

/// Compares two slices after low-precision rounding at precision 7.
#[must_use]
pub fn lp_eq(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| lp(*x) == lp(*y))
}

/// Element-wise relative-tolerance compare.
///
/// Each pair must satisfy `|a - b| <= eps * max(1, |a|, |b|)`. This is the
/// scaled tolerance the golden fixtures were generated against.
#[must_use]
pub fn approx_eq(a: &[f64], b: &[f64], eps: f64) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(x, y)| {
            let limit = eps * 1.0_f64.max(x.abs()).max(y.abs());
            (x - y).abs() <= limit
        })
}

/// Asserts two slices match within the relative tolerance `eps`.
pub fn assert_approx(a: &[f64], b: &[f64], eps: f64, label: &str) {
    assert!(approx_eq(a, b, eps), "{label}: {a:?} vs {b:?} (eps {eps})");
}

/// Asserts two slices match after low-precision rounding.
pub fn assert_lp(a: &[f64], b: &[f64], label: &str) {
    assert!(lp_eq(a, b), "{label}: {a:?} vs {b:?}");
}

/// The four sample viewports iterated by most tests.
#[must_use]
pub fn sample_viewports() -> Vec<(&'static str, WebMercatorViewportProps)> {
    vec![
        (
            "flat",
            WebMercatorViewportProps {
                width: 800.0,
                height: 600.0,
                longitude: Some(-122.43),
                latitude: Some(37.75),
                zoom: Some(11.5),
                bearing: Some(0.0),
                ..Default::default()
            },
        ),
        (
            "pitched",
            WebMercatorViewportProps {
                width: 800.0,
                height: 600.0,
                longitude: Some(-122.43),
                latitude: Some(37.75),
                zoom: Some(11.5),
                pitch: Some(30.0),
                bearing: Some(0.0),
                ..Default::default()
            },
        ),
        (
            "rotated",
            WebMercatorViewportProps {
                width: 1267.0,
                height: 400.0,
                longitude: Some(-122.4194),
                latitude: Some(37.7749),
                zoom: Some(11.0),
                altitude: Some(1.5),
                bearing: Some(180.0),
                pitch: Some(60.0),
                ..Default::default()
            },
        ),
        (
            "highLatitude",
            WebMercatorViewportProps {
                width: 500.0,
                height: 500.0,
                longitude: Some(42.42694),
                latitude: Some(75.751537),
                zoom: Some(15.5),
                altitude: Some(1.5),
                bearing: Some(-40.0),
                pitch: Some(20.0),
                ..Default::default()
            },
        ),
    ]
}

/// Great-circle destination on a sphere, matching `@turf/destination`.
///
/// `origin` is `[lng, lat]` in degrees, `distance` is in kilometers, `bearing`
/// is in degrees. Turf uses the WGS84 mean radius of 6371.0088 km. Returns
/// `[lng, lat]` in degrees.
#[must_use]
pub fn turf_destination(origin: [f64; 2], distance_km: f64, bearing_deg: f64) -> [f64; 2] {
    let radius_km = 6371.0088;
    let lng1 = origin[0].to_radians();
    let lat1 = origin[1].to_radians();
    let bearing = bearing_deg.to_radians();
    let radians = distance_km / radius_km;

    let lat2 = (lat1.sin() * radians.cos() + lat1.cos() * radians.sin() * bearing.cos()).asin();
    let lng2 = lng1
        + (bearing.sin() * radians.sin() * lat1.cos())
            .atan2(radians.cos() - lat1.sin() * lat2.sin());

    [lng2.to_degrees(), lat2.to_degrees()]
}

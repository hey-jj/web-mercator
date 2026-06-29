//! Contract tests for the free projection functions.
//!
//! These pin behavior the golden tables skip: the assertion messages, the
//! value-dependent return arity, the no-assert inverse path, and how
//! non-finite inputs flow through the scale functions. Expected values come
//! from the same formulas the functions implement, evaluated by hand or in a
//! scratch script and pasted here as constants.

mod common;

use web_mercator_viewport::{
    add_meters_to_lng_lat, get_distance_scales, get_meter_zoom, lng_lat_to_world, pixels_to_world,
    scale_to_zoom, units_per_meter, world_to_lng_lat, world_to_pixels, zoom_to_scale,
    WebMercatorViewport,
};

// --- scale and zoom: no validation, non-finite flows through ---

#[test]
fn scale_zoom_non_finite_flow_through() {
    assert_eq!(scale_to_zoom(0.0), f64::NEG_INFINITY, "log2(0)");
    assert!(scale_to_zoom(-4.0).is_nan(), "log2 of negative is NaN");
    assert_eq!(zoom_to_scale(f64::INFINITY), f64::INFINITY, "2^inf");
    assert_eq!(scale_to_zoom(f64::INFINITY), f64::INFINITY, "log2(inf)");
    assert!(zoom_to_scale(f64::NAN).is_nan(), "2^NaN is NaN");
}

// --- lng_lat_to_world: assertions and pole behavior ---

#[test]
#[should_panic(expected = "assertion failed")]
fn lng_lat_to_world_non_finite_lng_panics() {
    let _ = lng_lat_to_world(&[f64::INFINITY, 38.0]);
}

#[test]
#[should_panic(expected = "invalid latitude")]
fn lng_lat_to_world_lat_above_90_panics() {
    let _ = lng_lat_to_world(&[0.0, 90.001]);
}

#[test]
#[should_panic(expected = "invalid latitude")]
fn lng_lat_to_world_lat_nan_panics() {
    let _ = lng_lat_to_world(&[0.0, f64::NAN]);
}

#[test]
fn lng_lat_to_world_at_poles() {
    // lat = 90 does not throw. tan(pi/2) is a large finite double, so y is a
    // large finite value rather than infinity.
    let north = lng_lat_to_world(&[0.0, 90.0]);
    assert_eq!(north[0], 256.0);
    assert!((north[1] - 3298.0733173527224).abs() < 1e-9, "y at lat 90");

    // lat = -90: tan(0) = 0, ln(0) = -inf, so y is -inf.
    let south = lng_lat_to_world(&[0.0, -90.0]);
    assert_eq!(south[0], 256.0);
    assert!(south[1].is_infinite() && south[1] < 0.0, "y at lat -90");
}

#[test]
fn lng_lat_to_world_boundary_latitudes_do_not_panic() {
    // Exactly +/-90 are inside the asserted range and must not panic.
    let _ = lng_lat_to_world(&[0.0, 90.0]);
    let _ = lng_lat_to_world(&[0.0, -90.0]);
}

// --- world_to_lng_lat: no assertions, arbitrary input passes through ---

#[test]
fn world_to_lng_lat_has_no_assertions() {
    // Far-out-of-range world coordinates produce finite output with no panic.
    let r = world_to_lng_lat(&[-1000.0, 99999.0]);
    assert!((r[0] - (-883.125)).abs() < 1e-9, "lng wraps freely");
    assert!((r[1] - 90.0).abs() < 1e-9, "lat saturates near 90");
}

// --- get_meter_zoom and units_per_meter goldens ---

#[test]
fn get_meter_zoom_golden() {
    assert!((get_meter_zoom(0.0) - 16.25457827993699).abs() < 1e-9);
    assert!((get_meter_zoom(37.5) - 15.920613735406608).abs() < 1e-9);
    assert!((get_meter_zoom(75.0) - 14.30459396646049).abs() < 1e-9);
}

#[test]
#[should_panic(expected = "assertion failed")]
fn get_meter_zoom_non_finite_panics() {
    let _ = get_meter_zoom(f64::NAN);
}

#[test]
fn units_per_meter_golden() {
    assert!((units_per_meter(0.0) - 0.000_012_790_407_194_604_047).abs() < 1e-18);
    assert!((units_per_meter(37.75) - 0.000_016_176_268_942_111_666).abs() < 1e-18);
    assert!((units_per_meter(75.0) - 0.000_049_418_338_552_086_234).abs() < 1e-18);
}

// --- get_distance_scales: longitude only validated, second-order keys gated ---

#[test]
#[should_panic(expected = "assertion failed")]
fn get_distance_scales_non_finite_longitude_panics() {
    // longitude is read solely for the finiteness check.
    let _ = get_distance_scales(37.0, f64::INFINITY, false);
}

#[test]
fn get_distance_scales_low_precision_omits_second_order() {
    let s = get_distance_scales(37.75, -122.0, false);
    assert!(
        s.units_per_meter2.is_none(),
        "no unitsPerMeter2 in low precision"
    );
    assert!(
        s.units_per_degree2.is_none(),
        "no unitsPerDegree2 in low precision"
    );
}

#[test]
fn get_distance_scales_high_precision_has_second_order() {
    let s = get_distance_scales(37.75, -122.0, true);
    let upm2 = s.units_per_meter2.expect("unitsPerMeter2 present");
    let upd2 = s.units_per_degree2.expect("unitsPerDegree2 present");
    // The fixed zero slots: unitsPerMeter2[1] and unitsPerDegree2[0].
    assert_eq!(upm2[1], 0.0, "unitsPerMeter2[1] is 0");
    assert_eq!(upd2[0], 0.0, "unitsPerDegree2[0] is 0");
}

#[test]
fn get_distance_scales_at_equator_golden() {
    let s = get_distance_scales(0.0, 0.0, true);
    assert!((s.units_per_meter[0] - 0.000_012_790_407_194_604_047).abs() < 1e-18);
    assert!((s.meters_per_unit[0] - 78183.59375).abs() < 1e-6);
    assert!((s.units_per_degree[0] - 1.422_222_222_222_222_3).abs() < 1e-12);
    assert!((s.degrees_per_unit[0] - 0.703125).abs() < 1e-12);
    // At the equator tan(0) = 0, so every second-order term is exactly 0.
    assert_eq!(s.units_per_degree2.unwrap(), [0.0, 0.0, 0.0]);
    assert_eq!(s.units_per_meter2.unwrap(), [0.0, 0.0, 0.0]);
}

// --- add_meters_to_lng_lat: value-dependent return arity ---

#[test]
fn add_meters_length_two_when_no_z() {
    // Two-element position, two-element offset: neither z is finite, length 2.
    let r = add_meters_to_lng_lat(&[-122.0, 38.0], &[100.0, 200.0]);
    assert_eq!(r.len(), 2, "no z anywhere gives length 2");
    assert!((r[0] - (-121.998_858_711_560_73)).abs() < 1e-9);
    assert!((r[1] - 38.001_798_628_954_4).abs() < 1e-9);
}

#[test]
fn add_meters_length_three_when_offset_z_finite() {
    // Offset carries z, so the result keeps a z component.
    let r = add_meters_to_lng_lat(&[-122.0, 38.0], &[100.0, 200.0, 5.0]);
    assert_eq!(r.len(), 3, "finite offset z gives length 3");
    assert_eq!(r[2], 5.0, "newZ = (0 || 0) + 5");
}

#[test]
fn add_meters_length_three_when_input_z_is_zero() {
    // Input z of 0 is finite, so length 3 even though 0 is falsy.
    let r = add_meters_to_lng_lat(&[-122.0, 38.0, 0.0], &[100.0, 200.0]);
    assert_eq!(r.len(), 3, "finite input z of 0 gives length 3");
    assert_eq!(r[2], 0.0, "newZ = (0 || 0) + (0 || 0) = 0");
}

// --- world_to_pixels: finite assert, z defaults to 0, length-4 return ---

#[test]
fn world_to_pixels_defaults_missing_z_and_returns_length_four() {
    let viewport = WebMercatorViewport::default();
    let m = &viewport.pixel_projection_matrix;
    let with_z = world_to_pixels(&[256.0, 256.0, 0.0], m);
    let no_z = world_to_pixels(&[256.0, 256.0], m);
    assert_eq!(no_z.len(), 4, "result keeps the homogeneous w");
    // Missing z is treated as 0, so both inputs project to the same point.
    for i in 0..4 {
        assert!((with_z[i] - no_z[i]).abs() < 1e-12, "z defaults to 0");
    }
}

#[test]
#[should_panic(expected = "assertion failed")]
fn world_to_pixels_non_finite_panics() {
    let viewport = WebMercatorViewport::default();
    let _ = world_to_pixels(&[f64::INFINITY, 3.0], &viewport.pixel_projection_matrix);
}

// --- pixels_to_world: invalid-pixel assert and finite-z branch ---

#[test]
#[should_panic(expected = "invalid pixel coordinate")]
fn pixels_to_world_non_finite_pixel_panics() {
    let viewport = WebMercatorViewport::default();
    let _ = pixels_to_world(&[f64::NAN, 3.0], &viewport.pixel_unprojection_matrix, 0.0);
}

#[test]
fn pixels_to_world_finite_z_returns_length_four() {
    let viewport = WebMercatorViewport::default();
    let r = pixels_to_world(&[10.0, 20.0, 0.5], &viewport.pixel_unprojection_matrix, 0.0);
    assert_eq!(r.len(), 4, "finite z keeps the full 4-vector");
}

#[test]
fn pixels_to_world_no_z_returns_length_two() {
    // No third element drops into the ray-plane branch, which returns [x, y].
    let viewport = WebMercatorViewport::default();
    let r = pixels_to_world(&[10.0, 20.0], &viewport.pixel_unprojection_matrix, 0.0);
    assert_eq!(r.len(), 2, "ray-plane branch returns length 2");
}

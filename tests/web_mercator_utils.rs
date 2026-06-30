//! Free-function projection and scale math.

mod common;

use common::{approx_eq, lp, sample_viewports, turf_destination};
use web_mercator::{
    add_meters_to_lng_lat, get_distance_scales, get_meter_zoom, get_projection_parameters,
    lng_lat_to_world, zoom_to_scale, Precision, ProjectionOptions,
};

const DISTANCE_TOLERANCE: f64 = 0.0005;
const DISTANCE_TOLERANCE_PIXELS: f64 = 2.0;
const DISTANCE_SCALE_TEST_ZOOM: f64 = 12.0;

#[test]
fn lng_lat_to_world_golden() {
    let out = lng_lat_to_world(&[-122.0, 38.0]);
    assert!((out[0] - 82.4888888888889).abs() < 1e-9);
    assert!((out[1] - 314.50692551385134).abs() < 1e-9);
}

#[test]
#[should_panic(expected = "invalid latitude")]
fn lng_lat_to_world_invalid_latitude() {
    // Arguments swapped, so latitude is -122 and out of range.
    let _ = lng_lat_to_world(&[38.0, -122.0]);
}

#[test]
fn distance_scales_round_trip() {
    for (_, props) in sample_viewports() {
        let scales = get_distance_scales(
            props.latitude.unwrap(),
            props.longitude.unwrap(),
            Precision::Standard,
        );
        for i in 0..3 {
            let mu = scales.meters_per_unit[i] * scales.units_per_meter[i];
            let du = scales.degrees_per_unit[i] * scales.units_per_degree[i];
            assert_eq!(lp(mu), 1.0, "metersPerUnit * unitsPerMeter");
            assert_eq!(lp(du), 1.0, "degreesPerUnit * unitsPerDegree");
        }
    }
}

/// Per-component error in pixels and as a ratio, matching the `getDiff` helper.
fn get_diff(value: [f64; 3], base: [f64; 3], scale: f64) -> ([f64; 3], [f64; 3]) {
    let mut error_pixels = [0.0; 3];
    let mut error = [0.0; 3];
    for i in 0..3 {
        error_pixels[i] = ((value[i] - base[i]) * scale).abs();
        error[i] = (value[i] - base[i]).abs() / value[i].abs().min(base[i].abs());
    }
    (error_pixels, error)
}

#[test]
fn distance_scales_units_per_degree() {
    let scale = 2f64.powf(DISTANCE_SCALE_TEST_ZOOM);
    let z = 1000.0;

    for (_, props) in sample_viewports() {
        let longitude = props.longitude.unwrap();
        let latitude = props.latitude.unwrap();
        let s = get_distance_scales(latitude, longitude, Precision::High);
        let upd = s.units_per_degree;
        let upd2 = s.units_per_degree2.unwrap();

        for delta in [0.001, 0.01, 0.05, 0.1, 0.3] {
            let coords_adjusted = [
                delta * (upd[0] + upd2[0] * delta),
                delta * (upd[1] + upd2[1] * delta),
                z * (upd[2] + upd2[2] * delta),
            ];

            let pt = [longitude + delta, latitude + delta];
            let base = lng_lat_to_world(&[longitude, latitude]);
            let moved = lng_lat_to_world(&pt);
            let real_coords = [
                moved[0] - base[0],
                moved[1] - base[1],
                z * get_distance_scales(pt[1], pt[0], Precision::Standard).units_per_meter[2],
            ];

            let (pixels, ratio) = get_diff(coords_adjusted, real_coords, scale);
            assert!(
                ratio.iter().all(|e| *e < DISTANCE_TOLERANCE),
                "ratio tolerance, delta {delta}"
            );
            assert!(
                pixels.iter().all(|p| *p < DISTANCE_TOLERANCE_PIXELS),
                "pixel tolerance, delta {delta}"
            );
        }
    }
}

#[test]
fn distance_scales_units_per_meter() {
    let scale = 2f64.powf(DISTANCE_SCALE_TEST_ZOOM);
    let z = 1000.0;

    for (_, props) in sample_viewports() {
        let longitude = props.longitude.unwrap();
        let latitude = props.latitude.unwrap();
        let s = get_distance_scales(latitude, longitude, Precision::High);
        let upm = s.units_per_meter;
        let upm2 = s.units_per_meter2.unwrap();

        for delta in [10.0, 100.0, 1000.0, 5000.0, 10000.0, 30000.0] {
            let coords_adjusted = [
                delta * (upm[0] + upm2[0] * delta),
                delta * (upm[1] + upm2[1] * delta),
                z * (upm[2] + upm2[2] * delta),
            ];

            let pt = turf_destination([longitude, latitude], (delta / 1000.0) * 2f64.sqrt(), 45.0);
            let base = lng_lat_to_world(&[longitude, latitude]);
            let moved = lng_lat_to_world(&pt);
            let real_coords = [
                moved[0] - base[0],
                moved[1] - base[1],
                z * get_distance_scales(pt[1], pt[0], Precision::Standard).units_per_meter[2],
            ];

            let (pixels, ratio) = get_diff(coords_adjusted, real_coords, scale);
            assert!(
                ratio.iter().all(|e| *e < DISTANCE_TOLERANCE),
                "ratio tolerance, delta {delta}"
            );
            assert!(
                pixels.iter().all(|p| *p < DISTANCE_TOLERANCE_PIXELS),
                "pixel tolerance, delta {delta}"
            );
        }
    }
}

#[test]
fn add_meters_matches_turf() {
    let eps = 1e-7;
    for (_, props) in sample_viewports() {
        let longitude = props.longitude.unwrap();
        let latitude = props.latitude.unwrap();

        for delta in [10.0, 100.0, 1000.0, 5000.0] {
            let origin = [longitude, latitude];
            let dest = turf_destination(origin, (delta / 1000.0) * 2f64.sqrt(), 45.0);
            let expected = [dest[0], dest[1], delta];

            let result = add_meters_to_lng_lat(&origin, &[delta, delta, delta]);
            assert_eq!(result.len(), 3);
            assert!(
                approx_eq(&result, &expected, eps),
                "addMetersToLngLat delta {delta}: {result:?} vs {expected:?}"
            );
        }
    }
}

#[test]
fn meter_zoom_yields_one_pixel_per_meter() {
    for latitude in [0.0, 37.5, 75.0] {
        let zoom = get_meter_zoom(latitude);
        let scale = zoom_to_scale(zoom);
        let units = get_distance_scales(latitude, 0.0, Precision::Standard).units_per_meter;
        for u in units {
            assert_eq!(lp(u * scale), 1.0, "1 pixel per meter at {latitude}");
        }
    }
}

#[test]
fn projection_parameters_are_valid() {
    let mut cases = sample_viewports();
    cases.push((
        "extremePitched",
        web_mercator::WebMercatorViewportProps {
            width: 800.0,
            height: 600.0,
            longitude: Some(-122.43),
            latitude: Some(37.75),
            zoom: Some(11.5),
            pitch: Some(80.0),
            bearing: Some(0.0),
            ..Default::default()
        },
    ));

    for (title, props) in cases {
        let params = get_projection_parameters(&ProjectionOptions {
            width: props.width,
            height: props.height,
            pitch: props.pitch,
            fovy: props.fovy,
            altitude: props.altitude,
            ..Default::default()
        });
        assert!(params.fov.is_finite(), "{title}: fov");
        assert!(params.aspect.is_finite(), "{title}: aspect");
        assert!(params.focal_distance.is_finite(), "{title}: focalDistance");
        assert!(
            params.near.is_finite() && params.near > 0.0,
            "{title}: near"
        );
        assert!(
            params.far.is_finite() && params.far > params.near,
            "{title}: far"
        );
    }
}

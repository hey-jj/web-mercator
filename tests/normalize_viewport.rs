//! normalizeViewportProps golden values.

mod common;

use common::approx_eq;
use web_mercator_viewport::{normalize_viewport_props, ViewportProps};

struct Case {
    input: ViewportProps,
    expected: ViewportProps,
}

fn props(
    width: f64,
    height: f64,
    longitude: f64,
    latitude: f64,
    zoom: f64,
    pitch: f64,
    bearing: f64,
) -> ViewportProps {
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

fn cases() -> Vec<Case> {
    vec![
        Case {
            input: props(800.0, 600.0, -200.0, 10.0, 0.0, 60.0, 200.0),
            expected: props(800.0, 600.0, 160.0, 0.0, 0.22881869049588088, 60.0, -160.0),
        },
        Case {
            input: props(1000.0, 1000.0, 80.0, 0.0, 1.0, 0.0, 0.0),
            expected: props(1000.0, 1000.0, 80.0, 0.0, 1.0, 0.0, 0.0),
        },
        Case {
            input: props(1000.0, 1000.0, 80.0, -50.0, 1.0, 0.0, 0.0),
            expected: props(1000.0, 1000.0, 80.0, -4.214943141390651, 1.0, 0.0, 0.0),
        },
        Case {
            input: props(1000.0, 1000.0, 80.0, 80.0, 1.0, 0.0, 0.0),
            expected: props(1000.0, 1000.0, 80.0, 4.214943141390651, 1.0, 0.0, 0.0),
        },
    ]
}

#[test]
fn normalize_golden() {
    let eps = 1e-7;
    for case in cases() {
        let result = normalize_viewport_props(&case.input);
        let got = [
            result.width,
            result.height,
            result.longitude,
            result.latitude,
            result.zoom,
            result.bearing.unwrap(),
            result.pitch.unwrap(),
        ];
        let want = [
            case.expected.width,
            case.expected.height,
            case.expected.longitude,
            case.expected.latitude,
            case.expected.zoom,
            case.expected.bearing.unwrap(),
            case.expected.pitch.unwrap(),
        ];
        assert!(
            approx_eq(&got, &want, eps),
            "normalize: {got:?} vs {want:?}"
        );
    }
}

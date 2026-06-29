//! flyToViewport interpolation snapshots and getFlyToDuration golden times.

mod common;

use common::lp;
use web_mercator_viewport::{fly_to_viewport, get_fly_to_duration, FlyToOptions, ViewportProps};

fn start_props() -> ViewportProps {
    ViewportProps {
        width: 800.0,
        height: 600.0,
        longitude: -122.45,
        latitude: 37.78,
        zoom: 12.0,
        pitch: None,
        bearing: None,
    }
}

fn end_props() -> ViewportProps {
    ViewportProps {
        width: 800.0,
        height: 600.0,
        longitude: -74.0,
        latitude: 40.7,
        zoom: 11.0,
        pitch: None,
        bearing: None,
    }
}

#[test]
fn fly_to_interpolation() {
    let cases = [
        (0.25, [-122.4017, 37.78297, 7.518116]),
        (0.5, [-106.3, 38.76683, 3.618313]),
        (0.75, [-74.19253, 40.68864, 6.522422]),
    ];
    for (t, expected) in cases {
        let result = fly_to_viewport(&start_props(), &end_props(), t, &FlyToOptions::default());
        assert_eq!(lp(result.longitude), lp(expected[0]), "longitude at t={t}");
        assert_eq!(lp(result.latitude), lp(expected[1]), "latitude at t={t}");
        assert_eq!(lp(result.zoom), lp(expected[2]), "zoom at t={t}");
    }
}

#[test]
fn fly_to_linear_branch_for_tiny_pan() {
    // Same center, zoom-only change forces the linear interpolation branch.
    let start = start_props();
    let mut end = start_props();
    end.zoom = 10.0;
    let t = 0.5;
    let result = fly_to_viewport(&start, &end, t, &FlyToOptions::default());
    assert_eq!(result.longitude, start.longitude, "linear longitude");
    assert_eq!(result.latitude, start.latitude, "linear latitude");
    assert_eq!(result.zoom, 11.0, "linear zoom lerp");
}

#[test]
fn fly_to_duration_golden() {
    let s = start_props();
    let e = end_props();

    let near = ViewportProps {
        longitude: s.longitude + 0.001,
        ..s
    };
    let zoom_only = ViewportProps { zoom: 10.0, ..s };

    let cases: [(ViewportProps, ViewportProps, FlyToOptions, f64); 10] = [
        (s, e, FlyToOptions::default(), 7325.7943),
        (s, near, FlyToOptions::default(), 8.5802857),
        (
            s,
            e,
            FlyToOptions {
                speed: Some(1.0),
                ..Default::default()
            },
            8790.9532,
        ),
        (
            s,
            e,
            FlyToOptions {
                speed: Some(5.0),
                ..Default::default()
            },
            1758.1906,
        ),
        (
            s,
            e,
            FlyToOptions {
                speed: Some(5.0),
                screen_speed: Some(2.0),
                ..Default::default()
            },
            6215.2039,
        ),
        (
            s,
            e,
            FlyToOptions {
                curve: Some(0.5),
                ..Default::default()
            },
            13787.929,
        ),
        (
            s,
            e,
            FlyToOptions {
                curve: Some(2.0),
                ..Default::default()
            },
            5757.2078,
        ),
        (
            s,
            e,
            FlyToOptions {
                max_duration: Some(5000.0),
                ..Default::default()
            },
            0.0,
        ),
        (s, s, FlyToOptions::default(), 0.014729167),
        (s, zoom_only, FlyToOptions::default(), 817.01417),
    ];

    for (start, end, opts, expected) in cases {
        let duration = get_fly_to_duration(&start, &end, &opts);
        assert_eq!(lp(duration), lp(expected), "duration");
    }
}

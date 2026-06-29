//! fitBounds golden values and degenerate-bounds behavior.

mod common;

use common::lp;
use web_mercator_viewport::{
    fit_bounds, FitBoundsOptions, Padding, PaddingOption, WebMercatorViewport,
    WebMercatorViewportProps,
};

struct Case {
    options: FitBoundsOptions,
    expected: [f64; 3], // longitude, latitude, zoom
}

fn cases() -> Vec<Case> {
    vec![
        Case {
            options: FitBoundsOptions::new(
                100.0,
                100.0,
                [[-73.9876, 40.7661], [-72.9876, 41.7661]],
            ),
            expected: [-73.48759999999997, 41.26801443944763, 5.723804361273887],
        },
        Case {
            // Northeast corner first. Same result.
            options: FitBoundsOptions::new(
                100.0,
                100.0,
                [[-72.9876, 41.7661], [-73.9876, 40.7661]],
            ),
            expected: [-73.48759999999997, 41.26801443944763, 5.723804361273887],
        },
        Case {
            options: FitBoundsOptions {
                max_zoom: Some(22.0),
                ..FitBoundsOptions::new(100.0, 100.0, [[-73.0, 10.0], [-73.0, 10.0]])
            },
            expected: [-73.0, 10.0, 22.0],
        },
        Case {
            options: FitBoundsOptions {
                min_extent: Some(0.01),
                ..FitBoundsOptions::new(100.0, 100.0, [[-73.0, 10.0], [-73.0, 10.0]])
            },
            expected: [-73.0, 10.0, 13.28771238],
        },
        Case {
            options: FitBoundsOptions {
                padding: Some(PaddingOption::Uniform(20.0)),
                offset: Some([0.0, -40.0]),
                ..FitBoundsOptions::new(600.0, 400.0, [[-23.407, 64.863], [-23.406, 64.874]])
            },
            expected: [-23.406499999999973, 64.86850056273362, 12.89199533073045],
        },
        Case {
            options: FitBoundsOptions {
                padding: Some(PaddingOption::Sides(Padding {
                    top: 100.0,
                    bottom: 10.0,
                    left: 30.0,
                    right: 30.0,
                })),
                offset: Some([0.0, -40.0]),
                ..FitBoundsOptions::new(600.0, 400.0, [[-23.407, 64.863], [-23.406, 64.874]])
            },
            expected: [-23.406499999999998, 64.87085760222105, 12.476957831451607],
        },
        Case {
            options: FitBoundsOptions::new(512.0, 512.0, [[-180.0, -90.0], [180.0, 90.0]]),
            expected: [0.0, 0.0, 0.0],
        },
    ]
}

#[test]
fn fit_bounds_golden() {
    for case in cases() {
        let result = fit_bounds(&case.options);
        assert!(result.longitude.is_finite());
        assert!(result.latitude.is_finite());
        assert!(result.zoom.is_finite());
        assert_eq!(lp(result.longitude), lp(case.expected[0]), "longitude");
        assert_eq!(lp(result.latitude), lp(case.expected[1]), "latitude");
        assert_eq!(lp(result.zoom), lp(case.expected[2]), "zoom");
    }
}

#[test]
fn viewport_fit_bounds_method() {
    for case in cases() {
        let viewport = WebMercatorViewport::new(&WebMercatorViewportProps {
            width: case.options.width,
            height: case.options.height,
            longitude: Some(-122.0),
            latitude: Some(37.7),
            zoom: Some(11.0),
            ..Default::default()
        });
        let result = viewport.fit_bounds(case.options.bounds, &case.options);
        assert_eq!(lp(result.longitude), lp(case.expected[0]), "longitude");
        assert_eq!(lp(result.latitude), lp(case.expected[1]), "latitude");
        assert_eq!(lp(result.zoom), lp(case.expected[2]), "zoom");
    }
}

#[test]
fn degenerate_default_does_not_panic() {
    let viewport = degenerate_viewport();
    let opts = FitBoundsOptions::new(100.0, 100.0, [[-70.0, 10.0], [-70.0, 10.0]]);
    let _ = viewport.fit_bounds(opts.bounds, &opts);
}

#[test]
#[should_panic(expected = "assertion failed")]
fn degenerate_infinite_max_zoom_panics() {
    let viewport = degenerate_viewport();
    let opts = FitBoundsOptions {
        max_zoom: Some(f64::INFINITY),
        ..FitBoundsOptions::new(100.0, 100.0, [[-70.0, 10.0], [-70.0, 10.0]])
    };
    let _ = viewport.fit_bounds(opts.bounds, &opts);
}

#[test]
fn degenerate_min_extent_recovers() {
    let viewport = degenerate_viewport();
    let opts = FitBoundsOptions {
        min_extent: Some(0.01),
        max_zoom: Some(f64::INFINITY),
        ..FitBoundsOptions::new(100.0, 100.0, [[-70.0, 10.0], [-70.0, 10.0]])
    };
    let _ = viewport.fit_bounds(opts.bounds, &opts);
}

#[test]
#[should_panic(expected = "assertion failed")]
fn over_padding_panics() {
    // Padding larger than width leaves a non-positive target size.
    let opts = FitBoundsOptions {
        padding: Some(PaddingOption::Uniform(60.0)),
        ..FitBoundsOptions::new(100.0, 100.0, [[-73.9876, 40.7661], [-72.9876, 41.7661]])
    };
    let _ = fit_bounds(&opts);
}

fn degenerate_viewport() -> WebMercatorViewport {
    WebMercatorViewport::new(&WebMercatorViewportProps {
        width: 100.0,
        height: 100.0,
        zoom: Some(2.0),
        ..Default::default()
    })
}

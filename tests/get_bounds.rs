//! getBounds and getBoundingRegion golden quads and rectangles.

mod common;

use common::lp;
use web_mercator_viewport::{get_bounds, WebMercatorViewport, WebMercatorViewportProps};

struct Case {
    props: WebMercatorViewportProps,
    z: f64,
    quad: [[f64; 3]; 4],
    rect: [[f64; 2]; 2],
}

fn cases() -> Vec<Case> {
    vec![
        Case {
            props: WebMercatorViewportProps {
                width: 400.0,
                height: 300.0,
                longitude: Some(-122.45),
                latitude: Some(37.78),
                zoom: Some(12.0),
                ..Default::default()
            },
            z: 0.0,
            quad: [
                [-122.48433228, 37.759645826, 0.0],
                [-122.41566772, 37.759645826, 0.0],
                [-122.41566772, 37.800348571, 0.0],
                [-122.48433228, 37.800348571, 0.0],
            ],
            rect: [[-122.48433228, 37.759645826], [-122.41566772, 37.800348571]],
        },
        Case {
            props: WebMercatorViewportProps {
                width: 400.0,
                height: 300.0,
                longitude: Some(0.0),
                latitude: Some(0.0),
                pitch: Some(30.0),
                bearing: Some(-145.0),
                zoom: Some(5.0),
                ..Default::default()
            },
            z: 0.0,
            quad: [
                [4.8494210119, 0.50056751458, 0.0],
                [-1.188214086, 4.7228142079, 0.0],
                [-7.1607865313, -0.73914046292, 0.0],
                [1.7545491312, -6.9645319778, 0.0],
            ],
            rect: [[-7.1607865313, -6.9645319778], [4.8494210119, 4.7228142079]],
        },
        Case {
            props: WebMercatorViewportProps {
                width: 400.0,
                height: 300.0,
                longitude: Some(0.0),
                latitude: Some(0.0),
                pitch: Some(30.0),
                bearing: Some(-145.0),
                zoom: Some(5.0),
                ..Default::default()
            },
            z: 100.0,
            quad: [
                [4.8492095189, 0.50094025264, 100.0],
                [-1.1877914785, 4.7227432002, 100.0],
                [-7.1597366566, -0.73863754973, 100.0],
                [1.754662676, -6.9633819448, 100.0],
            ],
            rect: [[-7.1597366566, -6.9633819448], [4.8492095189, 4.7227432002]],
        },
        Case {
            props: WebMercatorViewportProps {
                width: 400.0,
                height: 300.0,
                longitude: Some(0.0),
                latitude: Some(0.0),
                pitch: Some(80.0),
                zoom: Some(5.0),
                ..Default::default()
            },
            z: 0.0,
            quad: [
                [-1.520374, -6.552286, 0.0],
                [1.520374, -6.552286, 0.0],
                [43.94531, 66.65714, 0.0],
                [-43.94531, 66.65714, 0.0],
            ],
            rect: [[-43.94531, -6.552286], [43.94531, 66.65714]],
        },
        Case {
            props: WebMercatorViewportProps {
                width: 400.0,
                height: 300.0,
                longitude: Some(0.0),
                latitude: Some(0.0),
                pitch: Some(80.0),
                zoom: Some(5.0),
                ..Default::default()
            },
            z: 100.0,
            quad: [
                [-1.519578, -6.553936, 100.0],
                [1.519578, -6.553936, 100.0],
                [43.94531, 66.6572, 100.0],
                [-43.94531, 66.6572, 100.0],
            ],
            rect: [[-43.94531, -6.553936], [43.94531, 66.6572]],
        },
        Case {
            props: WebMercatorViewportProps {
                width: 400.0,
                height: 300.0,
                longitude: Some(0.0),
                latitude: Some(0.0),
                fovy: Some(25.0),
                pitch: Some(60.0),
                zoom: Some(5.0),
                ..Default::default()
            },
            z: 0.0,
            quad: [
                [-3.1752704999, -4.7574296668, 0.0],
                [3.1752704999, -4.7574296668, 0.0],
                [7.1338220274, 10.639062373, 0.0],
                [-7.1338220274, 10.639062373, 0.0],
            ],
            rect: [[-7.1338220274, -4.7574296668], [7.1338220274, 10.639062373]],
        },
    ]
}

fn flatten<const N: usize>(rows: &[[f64; N]]) -> Vec<f64> {
    rows.iter().flat_map(|r| r.iter().copied()).collect()
}

#[test]
fn get_bounds_quad() {
    for case in cases() {
        let viewport = WebMercatorViewport::new(&case.props);
        let result = get_bounds(&viewport, case.z);
        let flat: Vec<f64> = result.into_iter().flatten().collect();
        let expected = flatten(&case.quad);
        assert_quad(&flat, &expected);
    }
}

#[test]
fn viewport_get_bounds_and_region() {
    for case in cases() {
        let viewport = WebMercatorViewport::new(&case.props);

        let rect = viewport.get_bounds(case.z);
        assert_quad(&flatten(&rect), &flatten(&case.rect));

        let region = viewport.get_bounding_region(case.z);
        let flat: Vec<f64> = region.into_iter().flatten().collect();
        assert_quad(&flat, &flatten(&case.quad));
    }
}

fn assert_quad(a: &[f64], b: &[f64]) {
    assert_eq!(a.len(), b.len(), "lengths differ: {a:?} vs {b:?}");
    for (x, y) in a.iter().zip(b) {
        assert_eq!(lp(*x), lp(*y), "{a:?} vs {b:?}");
    }
}

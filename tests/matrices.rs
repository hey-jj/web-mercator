//! Direct 16-element goldens for the matrix-producing functions.
//!
//! These pin the column-major layout and the Mapbox defaults: near multiplier
//! 0.02, far multiplier 1.01, default altitude 1.5, the 0.01 to PI-0.01 clamp,
//! and the ten-times horizon limit. Round-trip tests can pass with a transposed
//! matrix, so these guard the exact element order.

mod common;

use common::approx_eq;
use web_mercator_viewport::{
    altitude_to_fovy, fovy_to_altitude, get_projection_matrix, get_projection_parameters,
    get_view_matrix, lng_lat_to_world, zoom_to_scale, ProjectionOptions,
};

fn flat_center() -> [f64; 3] {
    let world = lng_lat_to_world(&[-122.43, 37.75]);
    [world[0], world[1], 0.0]
}

#[test]
fn view_matrix_golden() {
    let scale = zoom_to_scale(11.5);
    let center = flat_center();
    let vm = get_view_matrix(600.0, 0.0, 0.0, 1.5, scale, Some(&center));
    let expected = [
        4.827182292900164,
        0.0,
        0.0,
        0.0,
        0.0,
        4.827182292900164,
        0.0,
        0.0,
        0.0,
        0.0,
        4.827182292900164,
        0.0,
        -395.23681365655096,
        -1516.0079031217304,
        -1.5,
        1.0,
    ];
    assert!(approx_eq(&vm, &expected, 1e-9), "view matrix: {vm:?}");
}

#[test]
fn projection_matrix_golden() {
    let scale = zoom_to_scale(11.5);
    let center = flat_center();
    let fovy = altitude_to_fovy(1.5);
    let pm = get_projection_matrix(&ProjectionOptions {
        width: 800.0,
        height: 600.0,
        scale: Some(scale),
        center: Some(center),
        pitch: Some(0.0),
        fovy: Some(fovy),
        near_z_multiplier: Some(0.02),
        far_z_multiplier: Some(1.01),
        ..Default::default()
    });
    let expected = [
        2.25,
        0.0,
        0.0,
        0.0,
        0.0,
        3.0,
        0.0,
        0.0,
        0.0,
        0.0,
        -1.0267558528428093,
        -1.0,
        0.0,
        0.0,
        -0.040535117056856196,
        0.0,
    ];
    assert!(
        approx_eq(&pm, &expected, 1e-12),
        "projection matrix: {pm:?}"
    );
}

#[test]
fn projection_parameters_golden() {
    let scale = zoom_to_scale(11.5);
    let center = flat_center();
    let fovy = altitude_to_fovy(1.5);
    let params = get_projection_parameters(&ProjectionOptions {
        width: 800.0,
        height: 600.0,
        scale: Some(scale),
        center: Some(center),
        pitch: Some(0.0),
        fovy: Some(fovy),
        near_z_multiplier: Some(0.02),
        far_z_multiplier: Some(1.01),
        ..Default::default()
    });
    assert!((params.fov - 0.6435011087932844).abs() < 1e-12, "fov");
    assert!((params.aspect - 1.3333333333333333).abs() < 1e-12, "aspect");
    assert!((params.focal_distance - 1.5).abs() < 1e-12, "focalDistance");
    assert_eq!(params.near, 0.02, "near is the raw multiplier");
    assert!(
        (params.far - 1.5150000000000001).abs() < 1e-12,
        "far horizon clamp"
    );
}

#[test]
fn altitude_fovy_round_trip() {
    let fovy = altitude_to_fovy(1.5);
    assert!(
        (fovy - 36.86989764584402).abs() < 1e-9,
        "altitudeToFovy(1.5)"
    );
    let altitude = fovy_to_altitude(fovy);
    assert!((altitude - 1.5).abs() < 1e-12, "inverse");
}

//! Edge-case behavior the golden tables under-exercise.

mod common;

use web_mercator_viewport::{
    altitude_to_fovy, fovy_to_altitude, get_distance_scales, pixels_to_world, scale_to_zoom,
    units_per_meter, world_to_lng_lat, zoom_to_scale, UnprojectOptions, WebMercatorViewport,
    WebMercatorViewportProps, MAX_LATITUDE,
};

#[test]
fn max_latitude_constant() {
    assert_eq!(MAX_LATITUDE, 85.051129);
}

#[test]
fn zoom_scale_round_trip() {
    assert_eq!(zoom_to_scale(0.0), 1.0);
    assert_eq!(zoom_to_scale(10.0), 1024.0);
    assert_eq!(scale_to_zoom(1024.0), 10.0);
    for z in [0.0, 1.5, 11.5, 22.0] {
        assert!((scale_to_zoom(zoom_to_scale(z)) - z).abs() < 1e-12);
    }
}

#[test]
fn altitude_fovy_are_inverses() {
    for a in [0.75, 1.5, 3.0] {
        assert!((fovy_to_altitude(altitude_to_fovy(a)) - a).abs() < 1e-12);
    }
}

#[test]
fn units_per_meter_matches_distance_scales() {
    for latitude in [0.0, 37.75, 75.0] {
        let direct = units_per_meter(latitude);
        let from_scales = get_distance_scales(latitude, 0.0, false).units_per_meter[0];
        assert_eq!(direct, from_scales);
    }
}

#[test]
fn world_to_lng_lat_direct_golden() {
    let out = world_to_lng_lat(&[82.4888888888889, 314.50692551385134]);
    assert!((out[0] - (-122.0)).abs() < 1e-9);
    assert!((out[1] - 38.0).abs() < 1e-9);
}

#[test]
fn altitude_floor_at_three_quarters() {
    let viewport = WebMercatorViewport::new(&WebMercatorViewportProps {
        width: 800.0,
        height: 600.0,
        altitude: Some(0.1),
        ..Default::default()
    });
    assert_eq!(viewport.altitude, 0.75, "altitude floored to 0.75");
}

#[test]
fn fovy_altitude_derivation_paths() {
    // Both unset: default altitude 1.5 drives fovy.
    let both = WebMercatorViewport::new(&WebMercatorViewportProps {
        width: 1.0,
        height: 1.0,
        ..Default::default()
    });
    assert_eq!(both.altitude, 1.5);
    assert!((both.fovy - altitude_to_fovy(1.5)).abs() < 1e-12);

    // fovy only: altitude derived from fovy, then floored.
    let fovy_only = WebMercatorViewport::new(&WebMercatorViewportProps {
        width: 1.0,
        height: 1.0,
        fovy: Some(40.0),
        ..Default::default()
    });
    assert_eq!(fovy_only.fovy, 40.0);
    assert_eq!(fovy_only.altitude, fovy_to_altitude(40.0).max(0.75));

    // altitude only: fovy derived from altitude.
    let alt_only = WebMercatorViewport::new(&WebMercatorViewportProps {
        width: 1.0,
        height: 1.0,
        altitude: Some(2.0),
        ..Default::default()
    });
    assert_eq!(alt_only.altitude, 2.0);
    assert!((alt_only.fovy - altitude_to_fovy(2.0)).abs() < 1e-12);
}

#[test]
fn pixels_to_world_degenerate_z0_equals_z1() {
    // A matrix whose z output ignores the input z (index 10 is zero) makes the
    // depth-0 and depth-1 unprojections share a world z. That forces the t = 0
    // branch, which returns the depth-0 point without dividing by zero.
    let matrix = [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 0.0, 0.0, //
        0.0, 0.0, 5.0, 1.0,
    ];
    let result = pixels_to_world(&[2.0, 3.0], &matrix, 7.0);
    // t = 0 means the result equals the depth-0 unprojection x and y.
    assert_eq!(result.len(), 2);
    assert!((result[0] - 2.0).abs() < 1e-12);
    assert!((result[1] - 3.0).abs() < 1e-12);
}

#[test]
fn unproject_2d_returns_length_two() {
    let viewport = WebMercatorViewport::new(&WebMercatorViewportProps {
        width: 800.0,
        height: 600.0,
        longitude: Some(-122.0),
        latitude: Some(37.0),
        zoom: Some(10.0),
        ..Default::default()
    });
    let out = viewport.unproject(&[400.0, 300.0], UnprojectOptions::default());
    assert_eq!(out.len(), 2, "no z and no target_z gives length 2");
}

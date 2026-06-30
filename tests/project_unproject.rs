//! Golden project and unproject values for a pitched San Francisco viewport.
//!
//! The expected screen and geographic values match the Mapbox GL camera. The
//! camera math here reproduces those numbers, so passing these cases also
//! confirms Mapbox compatibility.

mod common;

use common::assert_approx;
use web_mercator::{
    ProjectOptions, UnprojectOptions, WebMercatorViewport, WebMercatorViewportProps,
};

fn sf_viewport() -> WebMercatorViewport {
    WebMercatorViewport::new(&WebMercatorViewportProps {
        width: 800.0,
        height: 600.0,
        longitude: Some(-122.43),
        latitude: Some(37.75),
        zoom: Some(11.5),
        pitch: Some(30.0),
        bearing: Some(0.0),
        ..Default::default()
    })
}

#[test]
fn constructor_echoes_input_props() {
    let viewport = sf_viewport();
    assert_eq!(viewport.latitude, 37.75);
    assert_eq!(viewport.longitude, -122.43);
    assert_eq!(viewport.zoom, 11.5);
    assert_eq!(viewport.pitch, 30.0);
    assert_eq!(viewport.bearing, 0.0);
    assert_eq!(viewport.width, 800.0);
    assert_eq!(viewport.height, 600.0);
}

#[test]
fn project_unproject_golden() {
    let eps = 1e-7;
    let viewport = sf_viewport();

    let center = viewport.project(&[-122.43, 37.75], ProjectOptions::default());
    assert_approx(&center, &[400.0, 300.0], eps, "project center");

    let center_back = viewport.unproject(&[400.0, 300.0], UnprojectOptions::default());
    assert_approx(&center_back, &[-122.43, 37.75], eps, "unproject center");

    let corner = viewport.project(&[-122.55, 37.83], ProjectOptions::default());
    assert_approx(
        &corner,
        &[-1.329741801625046, 6.796120915775314],
        eps,
        "project corner",
    );

    let corner_back = viewport.unproject(&[0.0, 0.0], UnprojectOptions::default());
    assert_approx(
        &corner_back,
        &[-122.55024809579456, 37.832294933238586],
        eps,
        "unproject corner",
    );
}

#[test]
fn top_left_orientation() {
    let viewport = sf_viewport();

    let top_left = viewport.unproject(&[0.0, 0.0], UnprojectOptions::default());
    let bottom_left = viewport.unproject(&[0.0, viewport.height], UnprojectOptions::default());
    assert!(
        top_left[1] > bottom_left[1],
        "top-left latitude is north of bottom-left"
    );

    let top_left2 = viewport.unproject(
        &[0.0, viewport.height],
        UnprojectOptions {
            top_left: false,
            ..Default::default()
        },
    );
    let bottom_left2 = viewport.unproject(
        &[0.0, 0.0],
        UnprojectOptions {
            top_left: false,
            ..Default::default()
        },
    );
    assert_eq!(top_left, top_left2, "topLeft true/false match");
    assert_eq!(bottom_left, bottom_left2, "bottomLeft true/false match");
}

//! Viewport construction, equality, and round-trip behavior.

mod common;

use common::{approx_eq, sample_viewports};
use web_mercator_viewport::{
    ProjectOptions, UnprojectOptions, WebMercatorViewport, WebMercatorViewportProps,
};

#[test]
fn default_constructor() {
    let viewport = WebMercatorViewport::default();
    assert_eq!(viewport.width, 1.0);
    assert_eq!(viewport.height, 1.0);
}

#[test]
fn zero_width_height_forced_to_one() {
    let viewport = WebMercatorViewport::new(&WebMercatorViewportProps {
        width: 0.0,
        height: 0.0,
        longitude: Some(-122.43),
        latitude: Some(37.75),
        zoom: Some(11.5),
        ..Default::default()
    });
    assert_eq!(viewport.width, 1.0);
    assert_eq!(viewport.height, 1.0);
}

#[test]
fn camera_offset_sets_center_z() {
    let viewport = WebMercatorViewport::new(&WebMercatorViewportProps {
        width: 800.0,
        height: 600.0,
        longitude: Some(-122.43),
        latitude: Some(37.75),
        zoom: Some(11.5),
        position: Some([0.0, 0.0, 300.0]),
        ..Default::default()
    });
    assert!(
        viewport.center[2] != 0.0,
        "camera offset gives nonzero center z"
    );
}

#[test]
fn equality() {
    let flat = sample_viewports()[0].1.clone();
    let viewport1 = WebMercatorViewport::new(&flat);
    let viewport2 = WebMercatorViewport::new(&flat);
    let mut different = flat.clone();
    different.height = 33.0;
    let viewport3 = WebMercatorViewport::new(&different);

    assert!(viewport1.equals(&viewport1));
    assert!(viewport1.equals(&viewport2));
    assert!(!viewport1.equals(&viewport3));
}

#[test]
fn project_flat_round_trip() {
    let eps = 1e-6;
    for (_, vp) in sample_viewports() {
        let viewport = WebMercatorViewport::new(&vp);
        for (_, tc) in sample_viewports() {
            let lng_lat = [tc.longitude.unwrap(), tc.latitude.unwrap()];
            let xy = viewport.project_flat(&lng_lat);
            let back = viewport.unproject_flat(&xy);
            assert!(approx_eq(&back, &lng_lat, eps), "projectFlat round-trip");
        }
    }
}

#[test]
fn project_3d_round_trip() {
    let eps = 1e-6;
    for (_, vp) in sample_viewports() {
        let viewport = WebMercatorViewport::new(&vp);
        for (_, tc) in sample_viewports() {
            let input = [tc.longitude.unwrap(), tc.latitude.unwrap(), 100.0];
            let projected = viewport.project(&input, ProjectOptions::default());
            let by_depth = viewport.unproject(&projected, UnprojectOptions::default());
            let by_target = viewport.unproject(
                &[projected[0], projected[1]],
                UnprojectOptions {
                    target_z: Some(100.0),
                    ..Default::default()
                },
            );
            assert!(
                approx_eq(&by_depth, &input, eps),
                "unproject with pixel depth"
            );
            assert!(
                approx_eq(&by_target, &input, eps),
                "unproject with target z"
            );
        }
    }
}

#[test]
fn project_2d_round_trip() {
    let eps = 1e-6;
    for (_, vp) in sample_viewports() {
        let viewport = WebMercatorViewport::new(&vp);
        for (_, tc) in sample_viewports() {
            let lng_lat = [tc.longitude.unwrap(), tc.latitude.unwrap()];

            let xy = viewport.project(&lng_lat, ProjectOptions { top_left: true });
            let back = viewport.unproject(
                &xy,
                UnprojectOptions {
                    top_left: true,
                    ..Default::default()
                },
            );
            assert!(approx_eq(&back, &lng_lat, eps), "top-left round-trip");

            let xy = viewport.project(&lng_lat, ProjectOptions { top_left: false });
            let back = viewport.unproject(
                &xy,
                UnprojectOptions {
                    top_left: false,
                    ..Default::default()
                },
            );
            assert!(approx_eq(&back, &lng_lat, eps), "bottom-left round-trip");
        }
    }
}

#[test]
fn get_location_at_point_recenters() {
    let eps = 1e-6;
    let pos = [200.0, 200.0];
    for (_, vp) in sample_viewports() {
        let viewport = WebMercatorViewport::new(&vp);
        for (_, tc) in sample_viewports() {
            let lng_lat = [tc.longitude.unwrap(), tc.latitude.unwrap()];
            let new_center = viewport.get_map_center_by_lng_lat_position(&lng_lat, &pos);

            let mut recentered = vp.clone();
            recentered.longitude = Some(new_center[0]);
            recentered.latitude = Some(new_center[1]);
            let new_viewport = WebMercatorViewport::new(&recentered);

            let xy = new_viewport.project(&lng_lat, ProjectOptions::default());
            assert!(approx_eq(&xy, &pos, eps), "re-center then re-project");
        }
    }
}

//! Arity and falsy-coercion contracts on the viewport project/unproject path.
//!
//! The return length of `project` and `unproject` depends on the input length
//! and on whether z and target_z are finite. The `target_z` short-circuit also
//! mirrors the JS `targetZ && targetZ * scale` coercion, where a zero target_z
//! flows through unchanged. These cases pin that behavior.

mod common;

use web_mercator::{
    ProjectOptions, UnprojectOptions, WebMercatorViewport, WebMercatorViewportProps,
};

fn viewport() -> WebMercatorViewport {
    WebMercatorViewport::new(&WebMercatorViewportProps {
        width: 800.0,
        height: 600.0,
        longitude: Some(-122.0),
        latitude: Some(37.0),
        zoom: Some(10.0),
        ..Default::default()
    })
}

#[test]
fn project_arity_follows_input_length() {
    let vp = viewport();
    let two = vp.project(&[-122.0, 37.0], ProjectOptions::default());
    assert_eq!(two.len(), 2, "2D input gives 2D output");

    let three = vp.project(&[-122.0, 37.0, 100.0], ProjectOptions::default());
    assert_eq!(three.len(), 3, "3D input gives 3D output");
}

#[test]
fn unproject_arity_no_z_no_target() {
    let vp = viewport();
    let out = vp.unproject(&[400.0, 300.0], UnprojectOptions::default());
    assert_eq!(out.len(), 2, "no z and no target_z gives length 2");
}

#[test]
fn unproject_arity_finite_target_z() {
    let vp = viewport();
    let out = vp.unproject(
        &[400.0, 300.0],
        UnprojectOptions {
            target_z: Some(100.0),
            ..Default::default()
        },
    );
    assert_eq!(out.len(), 3, "finite target_z gives length 3");
    assert_eq!(out[2], 100.0, "third element is the target_z in meters");
}

#[test]
fn unproject_arity_finite_input_z() {
    let vp = viewport();
    let out = vp.unproject(&[400.0, 300.0, 0.5], UnprojectOptions::default());
    assert_eq!(out.len(), 3, "finite input z gives length 3");
}

#[test]
fn unproject_target_z_zero_short_circuits() {
    // target_z = 0 is falsy in JS, so `targetZ && targetZ * scale` short-circuits
    // to 0 rather than scaling meters to world units. The unprojected x and y
    // must match the no-target case, and the output carries the literal 0.
    let vp = viewport();
    let no_target = vp.unproject(&[400.0, 300.0], UnprojectOptions::default());
    let zero_target = vp.unproject(
        &[400.0, 300.0],
        UnprojectOptions {
            target_z: Some(0.0),
            ..Default::default()
        },
    );

    assert_eq!(zero_target.len(), 3, "target_z 0 is finite, so length 3");
    assert_eq!(zero_target[2], 0.0, "third element is the literal 0");
    assert!(
        (zero_target[0] - no_target[0]).abs() < 1e-12,
        "x matches the no-target unprojection"
    );
    assert!(
        (zero_target[1] - no_target[1]).abs() < 1e-12,
        "y matches the no-target unprojection"
    );
}

#[test]
fn project_position_treats_missing_and_zero_z_alike() {
    // (xyz[2] || 0): missing z and a zero z both yield a zero world z.
    let vp = viewport();
    let missing = vp.project_position(&[-122.0, 37.0]);
    let zero = vp.project_position(&[-122.0, 37.0, 0.0]);
    assert_eq!(
        missing, zero,
        "missing z and z=0 give the same world position"
    );
    assert_eq!(missing[2], 0.0, "world z is 0");
}

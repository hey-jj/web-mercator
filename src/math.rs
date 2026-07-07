//! Column-major 4x4 matrix and vector helpers.
//!
//! All matrices are length-16 `f64` arrays indexed `m[col * 4 + row]`. The math
//! here mirrors the column-major conventions used by WebGL and gl-matrix so the
//! view and projection matrices match Mapbox GL camera output to within a small
//! numerical tolerance (about 1e-9, see the matrix tests).
//!
//! Everything runs in `f64`. The matrices never truncate to `f32`, which keeps
//! the projection round-trips precise.

/// A column-major 4x4 matrix stored as 16 contiguous `f64` values.
pub type Mat4 = [f64; 16];

/// Message used by the input-validation assertions across the crate.
pub(crate) const ASSERT_MESSAGE: &str = "web-mercator: assertion failed";

/// Mirrors the JS `x || 0` coercion. Zero and NaN map to 0. Every other value
/// passes through.
#[must_use]
pub(crate) fn falsy_or_zero(value: f64) -> f64 {
    if value != 0.0 && !value.is_nan() {
        value
    } else {
        0.0
    }
}

/// Returns the 4x4 identity matrix.
#[must_use]
pub fn identity() -> Mat4 {
    [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ]
}

/// Floored modulo with a non-negative result for a positive divisor.
///
/// Rust's `%` follows the sign of the dividend. This corrects the result into
/// `[0, divisor)`, which is what longitude and bearing wrapping needs.
#[must_use]
pub fn mod_floor(value: f64, divisor: f64) -> f64 {
    let m = value % divisor;
    if m < 0.0 {
        divisor + m
    } else {
        m
    }
}

/// Linear interpolation. `step = 0` returns `start`, `step = 1` returns `end`.
#[must_use]
pub fn lerp(start: f64, end: f64, step: f64) -> f64 {
    step * end + (1.0 - step) * start
}

/// Clamps `x` into `[min, max]`.
#[must_use]
pub fn clamp(x: f64, min: f64, max: f64) -> f64 {
    if x < min {
        min
    } else if x > max {
        max
    } else {
        x
    }
}

/// Transforms a 4-vector by a projection matrix, then divides by the w
/// component. Returns the homogeneous-divided 4-vector `[x, y, z, 1]`.
#[must_use]
pub fn transform_vector(matrix: &Mat4, vector: [f64; 4]) -> [f64; 4] {
    let [x, y, z, w] = vector;
    let m = matrix;
    let mut out = [
        m[0] * x + m[4] * y + m[8] * z + m[12] * w,
        m[1] * x + m[5] * y + m[9] * z + m[13] * w,
        m[2] * x + m[6] * y + m[10] * z + m[14] * w,
        m[3] * x + m[7] * y + m[11] * z + m[15] * w,
    ];
    let s = 1.0 / out[3];
    out[0] *= s;
    out[1] *= s;
    out[2] *= s;
    out[3] *= s;
    out
}

/// Matrix product `a * b` in column-major layout.
#[must_use]
pub fn multiply(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut out = [0.0_f64; 16];
    let mut col = 0;
    while col < 4 {
        let b0 = b[col * 4];
        let b1 = b[col * 4 + 1];
        let b2 = b[col * 4 + 2];
        let b3 = b[col * 4 + 3];
        out[col * 4] = b0 * a[0] + b1 * a[4] + b2 * a[8] + b3 * a[12];
        out[col * 4 + 1] = b0 * a[1] + b1 * a[5] + b2 * a[9] + b3 * a[13];
        out[col * 4 + 2] = b0 * a[2] + b1 * a[6] + b2 * a[10] + b3 * a[14];
        out[col * 4 + 3] = b0 * a[3] + b1 * a[7] + b2 * a[11] + b3 * a[15];
        col += 1;
    }
    out
}

/// Right-multiplies `a` by a translation `[x, y, z]`.
#[must_use]
pub fn translate(a: &Mat4, v: [f64; 3]) -> Mat4 {
    let [x, y, z] = v;
    let mut out = *a;
    out[12] = a[0] * x + a[4] * y + a[8] * z + a[12];
    out[13] = a[1] * x + a[5] * y + a[9] * z + a[13];
    out[14] = a[2] * x + a[6] * y + a[10] * z + a[14];
    out[15] = a[3] * x + a[7] * y + a[11] * z + a[15];
    out
}

/// Right-multiplies `a` by a diagonal scale `[x, y, z]`.
#[must_use]
pub fn scale(a: &Mat4, v: [f64; 3]) -> Mat4 {
    let [x, y, z] = v;
    let mut out = *a;
    for i in 0..4 {
        out[i] *= x;
        out[i + 4] *= y;
        out[i + 8] *= z;
    }
    out
}

/// Right-multiplies `a` by a rotation of `rad` radians about the X axis.
#[must_use]
pub fn rotate_x(a: &Mat4, rad: f64) -> Mat4 {
    let s = rad.sin();
    let c = rad.cos();
    let mut out = *a;
    for i in 0..4 {
        let col1 = a[4 + i];
        let col2 = a[8 + i];
        out[4 + i] = col1 * c + col2 * s;
        out[8 + i] = col2 * c - col1 * s;
    }
    out
}

/// Right-multiplies `a` by a rotation of `rad` radians about the Z axis.
#[must_use]
pub fn rotate_z(a: &Mat4, rad: f64) -> Mat4 {
    let s = rad.sin();
    let c = rad.cos();
    let mut out = *a;
    for i in 0..4 {
        let col0 = a[i];
        let col1 = a[4 + i];
        out[i] = col0 * c + col1 * s;
        out[4 + i] = col1 * c - col0 * s;
    }
    out
}

/// Builds a perspective projection matrix with NDC z in `[-1, 1]`.
///
/// `far` is always a finite clamped value here, so the matrix uses the standard
/// finite-far form.
#[must_use]
pub fn perspective(fovy: f64, aspect: f64, near: f64, far: f64) -> Mat4 {
    let f = 1.0 / (fovy / 2.0).tan();
    let nf = 1.0 / (near - far);
    [
        f / aspect,
        0.0,
        0.0,
        0.0,
        0.0,
        f,
        0.0,
        0.0,
        0.0,
        0.0,
        (far + near) * nf,
        -1.0,
        0.0,
        0.0,
        2.0 * far * near * nf,
        0.0,
    ]
}

/// Relative tolerance used by [`mat4_equals`], matching gl-matrix.
pub const EPSILON: f64 = 0.000_001;

/// Compares two matrices element-wise with a relative tolerance of `1e-6`.
#[must_use]
pub fn mat4_equals(a: &Mat4, b: &Mat4) -> bool {
    for i in 0..16 {
        let limit = EPSILON * 1.0_f64.max(a[i].abs()).max(b[i].abs());
        if (a[i] - b[i]).abs() > limit {
            return false;
        }
    }
    true
}

/// Inverts a 4x4 matrix. Returns `None` when the determinant is zero.
#[must_use]
pub fn invert(a: &Mat4) -> Option<Mat4> {
    let a00 = a[0];
    let a01 = a[1];
    let a02 = a[2];
    let a03 = a[3];
    let a10 = a[4];
    let a11 = a[5];
    let a12 = a[6];
    let a13 = a[7];
    let a20 = a[8];
    let a21 = a[9];
    let a22 = a[10];
    let a23 = a[11];
    let a30 = a[12];
    let a31 = a[13];
    let a32 = a[14];
    let a33 = a[15];

    let b00 = a00 * a11 - a01 * a10;
    let b01 = a00 * a12 - a02 * a10;
    let b02 = a00 * a13 - a03 * a10;
    let b03 = a01 * a12 - a02 * a11;
    let b04 = a01 * a13 - a03 * a11;
    let b05 = a02 * a13 - a03 * a12;
    let b06 = a20 * a31 - a21 * a30;
    let b07 = a20 * a32 - a22 * a30;
    let b08 = a20 * a33 - a23 * a30;
    let b09 = a21 * a32 - a22 * a31;
    let b10 = a21 * a33 - a23 * a31;
    let b11 = a22 * a33 - a23 * a32;

    let mut det = b00 * b11 - b01 * b10 + b02 * b09 + b03 * b08 - b04 * b07 + b05 * b06;
    if det == 0.0 {
        return None;
    }
    det = 1.0 / det;

    Some([
        (a11 * b11 - a12 * b10 + a13 * b09) * det,
        (a02 * b10 - a01 * b11 - a03 * b09) * det,
        (a31 * b05 - a32 * b04 + a33 * b03) * det,
        (a22 * b04 - a21 * b05 - a23 * b03) * det,
        (a12 * b08 - a10 * b11 - a13 * b07) * det,
        (a00 * b11 - a02 * b08 + a03 * b07) * det,
        (a32 * b02 - a30 * b05 - a33 * b01) * det,
        (a20 * b05 - a22 * b02 + a23 * b01) * det,
        (a10 * b10 - a11 * b08 + a13 * b06) * det,
        (a01 * b08 - a00 * b10 - a03 * b06) * det,
        (a30 * b04 - a31 * b02 + a33 * b00) * det,
        (a21 * b02 - a20 * b04 - a23 * b00) * det,
        (a11 * b07 - a10 * b09 - a12 * b06) * det,
        (a00 * b09 - a01 * b07 + a02 * b06) * det,
        (a31 * b01 - a30 * b03 - a32 * b00) * det,
        (a20 * b03 - a21 * b01 + a22 * b00) * det,
    ])
}

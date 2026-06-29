//! Compute a non-perspective viewport that frames a bounding box.

use crate::math::{clamp, ASSERT_MESSAGE};
use crate::utils::{lng_lat_to_world, scale_to_zoom, world_to_lng_lat, MAX_LATITUDE};

/// Padding in pixels to add around the fitted bounds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Padding {
    /// Padding from the top edge.
    pub top: f64,
    /// Padding from the bottom edge.
    pub bottom: f64,
    /// Padding from the left edge.
    pub left: f64,
    /// Padding from the right edge.
    pub right: f64,
}

/// Either a uniform padding value or per-side padding.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PaddingOption {
    /// The same padding on all four sides.
    Uniform(f64),
    /// Distinct padding per side.
    Sides(Padding),
}

/// Options for [`fit_bounds`].
///
/// `bounds` is `[[lng, lat], [lng, lat]]`. Either corner order works. Optional
/// fields take their documented defaults when `None`.
#[derive(Debug, Clone)]
pub struct FitBoundsOptions {
    /// Viewport width in pixels.
    pub width: f64,
    /// Viewport height in pixels.
    pub height: f64,
    /// Corner pair `[[lng, lat], [lng, lat]]`.
    pub bounds: [[f64; 2]; 2],
    /// Lower limit on the bounded extent. Defaults to 0.
    pub min_extent: Option<f64>,
    /// Maximum zoom to fit within. Defaults to 24.
    pub max_zoom: Option<f64>,
    /// Padding around the bounds. Defaults to 0.
    pub padding: Option<PaddingOption>,
    /// Offset of the bounds center from the map center in pixels. Defaults to
    /// `[0, 0]`.
    pub offset: Option<[f64; 2]>,
}

impl FitBoundsOptions {
    /// Creates options with the required fields and all optionals unset.
    #[must_use]
    pub fn new(width: f64, height: f64, bounds: [[f64; 2]; 2]) -> Self {
        Self {
            width,
            height,
            bounds,
            min_extent: None,
            max_zoom: None,
            padding: None,
            offset: None,
        }
    }
}

/// A fitted viewport: center longitude and latitude plus zoom.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FitBoundsResult {
    /// Center longitude in degrees.
    pub longitude: f64,
    /// Center latitude in degrees.
    pub latitude: f64,
    /// Zoom level.
    pub zoom: f64,
}

/// Returns the center and zoom that frame `bounds` within the viewport.
///
/// Only supports non-perspective mode.
///
/// # Panics
///
/// Panics if padding or offset consumes the whole viewport, leaving a
/// non-positive target size. Panics if the resulting zoom is not finite, which
/// happens for degenerate bounds with an infinite `max_zoom` and zero
/// `min_extent`.
#[must_use]
pub fn fit_bounds(options: &FitBoundsOptions) -> FitBoundsResult {
    let width = options.width;
    let height = options.height;
    let min_extent = options.min_extent.unwrap_or(0.0);
    let max_zoom = options.max_zoom.unwrap_or(24.0);
    let offset = options.offset.unwrap_or([0.0, 0.0]);

    let [[west, south], [east, north]] = options.bounds;
    let padding = padding_object(options.padding);

    let nw = lng_lat_to_world(&[west, clamp(north, -MAX_LATITUDE, MAX_LATITUDE)]);
    let se = lng_lat_to_world(&[east, clamp(south, -MAX_LATITUDE, MAX_LATITUDE)]);

    let size = [
        (se[0] - nw[0]).abs().max(min_extent),
        (se[1] - nw[1]).abs().max(min_extent),
    ];

    let target_size = [
        width - padding.left - padding.right - offset[0].abs() * 2.0,
        height - padding.top - padding.bottom - offset[1].abs() * 2.0,
    ];

    assert!(
        target_size[0] > 0.0 && target_size[1] > 0.0,
        "{ASSERT_MESSAGE}"
    );

    let scale_x = target_size[0] / size[0];
    let scale_y = target_size[1] / size[1];

    let offset_x = (padding.right - padding.left) / 2.0 / scale_x;
    let offset_y = (padding.top - padding.bottom) / 2.0 / scale_y;

    let center = [
        (se[0] + nw[0]) / 2.0 + offset_x,
        (se[1] + nw[1]) / 2.0 + offset_y,
    ];

    let center_lng_lat = world_to_lng_lat(&center);
    let zoom = max_zoom.min(scale_to_zoom(scale_x.min(scale_y).abs()));

    assert!(zoom.is_finite(), "{ASSERT_MESSAGE}");

    FitBoundsResult {
        longitude: center_lng_lat[0],
        latitude: center_lng_lat[1],
        zoom,
    }
}

/// Normalizes a padding option into per-side padding.
///
/// # Panics
///
/// Panics if any side of an explicit padding object is not finite.
fn padding_object(padding: Option<PaddingOption>) -> Padding {
    match padding {
        None | Some(PaddingOption::Uniform(_)) => {
            let value = match padding {
                Some(PaddingOption::Uniform(v)) => v,
                _ => 0.0,
            };
            Padding {
                top: value,
                bottom: value,
                left: value,
                right: value,
            }
        }
        Some(PaddingOption::Sides(p)) => {
            assert!(
                p.top.is_finite()
                    && p.bottom.is_finite()
                    && p.left.is_finite()
                    && p.right.is_finite(),
                "{ASSERT_MESSAGE}"
            );
            p
        }
    }
}

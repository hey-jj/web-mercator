# web-mercator-viewport

Web Mercator projection and a perspective map viewport that match the Mapbox GL
and deck.gl camera math. Pure `f64`, no dependencies.

The crate has two layers. Free functions convert between geographic coordinates,
world pixels on a 512x512 zoom-0 tile, screen pixels, meters, zoom, scale,
altitude, and field of view, and build the 4x4 view and projection matrices. The
`WebMercatorViewport` type bundles a full camera state and exposes `project`,
`unproject`, `fit_bounds`, and `get_bounds`.

Three standalone utilities round it out: `fit_bounds` frames a bounding box,
`fly_to_viewport` and `get_fly_to_duration` interpolate a smooth pan and zoom
from the van Wijk and Nuij algorithm, and `normalize_viewport_props` clamps a
viewport to the legal latitude, longitude, and zoom range.

Angles are degrees at the public boundary and radians inside. World coordinates
are pixels on the zoom-0 tile. Matrices are column-major length-16 arrays.

## Installation

```toml
[dependencies]
web-mercator-viewport = "0.1"
```

## Usage

Project a coordinate to screen pixels and back.

```rust
use web_mercator_viewport::{
    ProjectOptions, UnprojectOptions, WebMercatorViewport, WebMercatorViewportProps,
};

let viewport = WebMercatorViewport::new(&WebMercatorViewportProps {
    width: 800.0,
    height: 600.0,
    longitude: Some(-122.43),
    latitude: Some(37.75),
    zoom: Some(11.5),
    pitch: Some(30.0),
    ..Default::default()
});

let screen = viewport.project(&[-122.43, 37.75], ProjectOptions::default());
let lng_lat = viewport.unproject(&[400.0, 300.0], UnprojectOptions::default());
```

Frame a bounding box.

```rust
use web_mercator_viewport::{fit_bounds, FitBoundsOptions};

let result = fit_bounds(&FitBoundsOptions::new(
    100.0,
    100.0,
    [[-73.9876, 40.7661], [-72.9876, 41.7661]],
));
// result.longitude, result.latitude, result.zoom
```

## Behavior notes

Return length follows the input. `project` and `unproject` give a 2- or 3-element
vector depending on input length and whether a z or target z is finite, the same
rule the geographic helpers use.

Invalid input panics with the same message text as the camera model it tracks.
`lng_lat_to_world` panics on a latitude outside `[-90, 90]`. `fit_bounds` panics
when padding consumes the whole viewport or when degenerate bounds produce a
non-finite zoom.

## License

Licensed under the [MIT license](LICENSE).

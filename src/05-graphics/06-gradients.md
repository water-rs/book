# Animated Gradients

> **In this chapter, you will:**
> - Apply linear, radial, angular, and mesh gradients as backgrounds
> - Drive gradient colors and stop positions from reactive signals
> - Drop in self-animating gradients with `AnimatedMeshGradient` and `FlowingGradient`
> - Choose between the high-level `waterui::gradient` types and the low-level GPU `Gradient` view

Think of an app store hero banner, or a login screen where colors drift and blend like liquid paint. Animated gradients are one of the easiest ways to add visual richness, and WaterUI ships several GPU-accelerated gradient components -- from simple static fills to flowing, palette-driven mesh gradients.

## Where Gradient Types Live

WaterUI exposes two complementary gradient layers:

- **`waterui::gradient`** -- semantic gradient *descriptions* used by the rendering pipeline. `LinearGradient`, `RadialGradient`, `AngularGradient`, and `MeshGradient` are pure data types here, with reactive `Computed<Color>` stops and `UnitPoint` anchors.
- **`waterui::graphics`** -- the GPU `View` layer. `Gradient` is a `View` backed by `GradientConfig`, and the `AnimatedMeshGradient` / `FlowingGradient` views ship pre-tuned shader effects.

> **Note:** At the pinned waterui revision, the descriptive types in `waterui::gradient` are not themselves `View` and cannot be passed straight to `.background(...)`. Use `waterui::graphics::Gradient` (or one of the self-animating views below) when you want to drop a gradient into the view tree.

## GPU Gradient Views with `waterui::graphics`

`waterui::graphics::Gradient` is the all-in-one `View`. It accepts `Vec<(f32, ResolvedColor)>` color stops and maps to backend-native rendering for linear, radial, and angular variants, and to a dedicated GPU shader for mesh variants.

### Linear Gradient

```rust,ignore
use waterui::prelude::*;
use waterui::graphics::{Gradient, color::ResolvedColor};

fn linear_bg() -> impl View {
    Gradient::linear(
        vec![
            (0.0, ResolvedColor { red: 1.0, green: 0.0, blue: 0.5, opacity: 1.0, headroom: 0.0 }),
            (1.0, ResolvedColor { red: 0.0, green: 0.3, blue: 1.0, opacity: 1.0, headroom: 0.0 }),
        ],
        [0.5, 0.0], // start point (normalized)
        [0.5, 1.0], // end point
    )
}
```

Wrap the gradient in `zstack` or use `.background(gradient)` to draw content on top of it.

### Radial Gradient

```rust,ignore
fn radial_bg() -> impl View {
    Gradient::radial(
        vec![
            (0.0, ResolvedColor { red: 1.0, green: 1.0, blue: 1.0, opacity: 1.0, headroom: 0.0 }),
            (1.0, ResolvedColor { red: 0.0, green: 0.0, blue: 0.2, opacity: 1.0, headroom: 0.0 }),
        ],
        [0.5, 0.5], // center
        0.0,        // start radius
        0.7,        // end radius
    )
}
```

### Angular (Conic) Gradient

```rust,ignore
use core::f32::consts::TAU;

fn conic_bg() -> impl View {
    Gradient::angular(
        vec![
            (0.0,  ResolvedColor { red: 1.0, green: 0.0, blue: 0.0, opacity: 1.0, headroom: 0.0 }),
            (0.33, ResolvedColor { red: 0.0, green: 1.0, blue: 0.0, opacity: 1.0, headroom: 0.0 }),
            (0.66, ResolvedColor { red: 0.0, green: 0.0, blue: 1.0, opacity: 1.0, headroom: 0.0 }),
            (1.0,  ResolvedColor { red: 1.0, green: 0.0, blue: 0.0, opacity: 1.0, headroom: 0.0 }),
        ],
        [0.5, 0.5],
        0.0,
        TAU,
    )
}
```

### Mesh Gradient

Static mesh gradients interpolate across a vertex grid. Provide `width * height` vertices in row-major order:

```rust,ignore
use waterui::graphics::{Gradient, color::ResolvedColor};

fn mesh_bg() -> impl View {
    let red    = ResolvedColor { red: 1.0, green: 0.0, blue: 0.0, opacity: 1.0, headroom: 0.0 };
    let blue   = ResolvedColor { red: 0.0, green: 0.0, blue: 1.0, opacity: 1.0, headroom: 0.0 };
    let green  = ResolvedColor { red: 0.0, green: 1.0, blue: 0.0, opacity: 1.0, headroom: 0.0 };
    let yellow = ResolvedColor { red: 1.0, green: 1.0, blue: 0.0, opacity: 1.0, headroom: 0.0 };

    Gradient::mesh(
        2, 2,
        vec![
            ([0.0, 0.0], red),
            ([1.0, 0.0], blue),
            ([0.0, 1.0], green),
            ([1.0, 1.0], yellow),
        ],
        true, // smooth color interpolation
    )
}
```

`Gradient::mesh` panics if `vertices.len() != width * height`, so the grid stays internally consistent.

## Composing Gradients with `GradientConfig`

For full control, build a `GradientConfig` directly and hand it to `Gradient::new`:

```rust,ignore
use waterui::graphics::{Gradient, GradientConfig, GradientType};

let config = GradientConfig {
    gradient_type: GradientType::Linear,
    stops: vec![(0.0, color_a), (0.5, color_b), (1.0, color_c)],
    start_point: [0.0, 0.0],
    end_point: [1.0, 1.0],
    start_value: 0.0,
    end_value: 1.0,
    mesh_size: (2, 2),
    mesh_vertices: Vec::new(),
    smooths_colors: true,
};

let view = Gradient::new(config);
```

The same struct also drives `GradientConfig::linear / radial / angular / mesh` constructors if you prefer the named entry points.

## Reactive Mesh Gradients

`waterui::graphics::MeshGradient<C>` accepts any `Signal` whose `Output` iterates `ResolvedColor` values. Update the source binding and the GPU buffer refreshes only when the colors actually change:

```rust,ignore
use waterui::graphics::MeshGradient;
use waterui::graphics::color::ResolvedColor;
use waterui::prelude::*;

fn reactive_mesh(colors: Binding<Vec<ResolvedColor>>) -> impl View {
    MeshGradient::new(3, 3, colors).smooths_colors(true)
}
```

Mesh vertices are arranged in row-major order (`row 0` first). Positions are derived from the grid dimensions automatically -- you only supply the colors.

## Self-Animating Gradients

Two views in `waterui::graphics` animate continuously without any host-side work.

### `AnimatedMeshGradient`

A 4x4 mesh palette warped by GPU noise. The default config is already production-ready:

```rust,ignore
use waterui::graphics::AnimatedMeshGradient;

fn animated_background() -> impl View {
    AnimatedMeshGradient::default()
}
```

Tune the speed, warp, or palette through `AnimatedMeshGradientConfig`:

```rust,ignore
use waterui::graphics::{AnimatedMeshGradient, AnimatedMeshGradientConfig};

fn bespoke_background() -> impl View {
    let config = AnimatedMeshGradientConfig::aqua_bloom()
        .speed(0.8)
        .warp(0.3);

    AnimatedMeshGradient::new(config)
}
```

Built-in palettes include `aqua_bloom`, `pastel_lagoon`, `soft_blush`, and `deep_blue`. Each is a 16-color (`4 * 4`) `ResolvedColor` array, accessible via `AnimatedMeshGradientConfig::palette([...])` if you want to supply your own. The constant `ANIMATED_MESH_PALETTE_LEN` documents the array length.

### `FlowingGradient`

`FlowingGradient` ships a procedural fBm-noise shader that produces a slow, ocean-like flow:

```rust,ignore
use waterui::graphics::flowing_gradient::FlowingGradient;

fn ambient_bg() -> impl View {
    FlowingGradient::default()
}
```

The shader uses gradient noise (4-octave fBm) with two flow fields warping the sample coordinates, plus a soft vignette and a deep navy-to-white palette. There are no public knobs -- `FlowingGradient` is the "set it and forget it" option.

## Composing Gradients with Other Views

Both gradient layers integrate cleanly with the rest of the framework. Stack content on top of an animated background with `zstack`:

```rust,ignore
use waterui::prelude::*;
use waterui::graphics::AnimatedMeshGradient;

fn welcome_card() -> impl View {
    zstack((
        AnimatedMeshGradient::default(),
        vstack((
            text("Welcome"),
            text("Beautiful gradient backgrounds"),
        ))
        .padding(),
    ))
}
```

Or constrain the gradient to a specific frame:

```rust,ignore
use waterui::graphics::{AnimatedMeshGradient, AnimatedMeshGradientConfig};

fn banner() -> impl View {
    AnimatedMeshGradient::new(AnimatedMeshGradientConfig::deep_blue())
        .size(400.0, 200.0)
}
```

## Performance Notes

- **One pass per gradient**: Linear, radial, and angular gradients map to native primitives. Mesh and animated mesh gradients run a single full-screen quad through their respective shaders.
- **Reactive efficiency**: `MeshGradient<C>` (low-level) caches the previous color slice and skips uploads when nothing changed. `ColorStop`s on the high-level types are tracked through their `Computed<Color>` channel.
- **Continuous redraw**: `AnimatedMeshGradient` and `FlowingGradient` request a redraw every frame while their animation speed is non-zero. Set `AnimatedMeshGradientConfig::speed(0.0)` to freeze the gradient when the app is backgrounded.

You now have a full toolbox of gradient effects -- from simple linear fills to self-animating mesh backgrounds. Drop one behind your hero copy, then move on to the next part of the book where you will assemble these pieces into complete screens.

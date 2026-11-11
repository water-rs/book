# Animated gradients

> **In this chapter, you will:**
> - Draw linear, radial, angular, and mesh gradients with `Gradient`
> - Drive a mesh gradient's colors, control points, and smoothing from signals
> - Use `AnimatedMeshGradient` and its presets for a background that moves on its own
> - Add `FlowingGradient`, the GPU noise gradient, and know what each one costs

A gradient is a view. It fills whatever box layout gives it, so one line puts a color wash behind your content, and a signal can drive it without rebuilding anything. The gradient views live in `waterui::gradient` and the prelude re-exports them, so `use waterui::prelude::*;` is all you need for the views. Their colors are `WorkingColor`, which the prelude also exports from `waterui::color`; the helpers that build one live in `waterui::color::working`.

![Static mesh gradient rendered by WaterUI graphics](../assets/visuals/05-graphics/gradient-mesh.png)

*A mesh gradient. [Example source](https://github.com/water-rs/book/tree/main/examples/book-visuals).*

## Building colors

`WorkingColor` is the engine's linear Display P3 color with straight alpha. Components above `1.0` are brighter than SDR white and only show on an HDR display. The `waterui::color::working::from_linear_srgb` helper builds one from linear sRGB components, so the values you pass are linear, not gamma-encoded.

```rust
use waterui::color::WorkingColor;
use waterui::color::working::from_linear_srgb;

fn rgb(red: f32, green: f32, blue: f32) -> WorkingColor {
    from_linear_srgb([red, green, blue], 1.0)
}

fn constants() -> [WorkingColor; 3] {
    [WorkingColor::BLACK, WorkingColor::WHITE, WorkingColor::TRANSPARENT]
}
```

The later sections reuse `rgb`.

## The Gradient view

`Gradient` is fixed: its colors and geometry never change after you build it. Every point is in unit space: `[0.0, 0.0]` is the view's top-left corner and `[1.0, 1.0]` its bottom-right, whatever the size of the box. Stops are `(offset, color)` pairs with offsets in `0.0..=1.0`; they are sorted for you. An empty stop list, an out-of-range offset, or a non-finite point or color component panics.

```rust
use waterui::prelude::*;
use waterui::color::working::from_linear_srgb;

fn rgb(red: f32, green: f32, blue: f32) -> WorkingColor {
    from_linear_srgb([red, green, blue], 1.0)
}

fn linear_bg() -> impl View {
    Gradient::linear(
        vec![(0.0, rgb(1.0, 0.3, 0.5)), (1.0, rgb(0.3, 0.5, 1.0))],
        [0.5, 0.0], // start: top center
        [0.5, 1.0], // end: bottom center
    )
}
```

Radial takes a center and two radii. Radii are fractions of the box's *shorter* side, so the gradient stays a circle on a non-square box instead of stretching into an ellipse. The start radius must be zero or more and the end radius greater than zero.

```rust
use waterui::prelude::*;
use waterui::color::working::from_linear_srgb;

fn rgb(red: f32, green: f32, blue: f32) -> WorkingColor {
    from_linear_srgb([red, green, blue], 1.0)
}

fn radial_bg() -> impl View {
    Gradient::radial(
        vec![
            (0.0, rgb(1.0, 1.0, 0.8)),
            (0.5, rgb(1.0, 0.6, 0.2)),
            (1.0, rgb(0.6, 0.2, 0.1)),
        ],
        [0.5, 0.5], // center
        0.0,        // start radius
        0.7,        // end radius
    )
}
```

Angular (conic) takes a center and a start and end angle in radians. The sweep, `end - start`, must be positive and at most `TAU`.

```rust
use core::f32::consts::TAU;
use waterui::prelude::*;
use waterui::color::working::from_linear_srgb;

fn rgb(red: f32, green: f32, blue: f32) -> WorkingColor {
    from_linear_srgb([red, green, blue], 1.0)
}

fn conic_bg() -> impl View {
    Gradient::angular(
        vec![
            (0.0, rgb(1.0, 0.0, 0.0)),
            (0.33, rgb(0.0, 1.0, 0.0)),
            (0.66, rgb(0.0, 0.0, 1.0)),
            (1.0, rgb(1.0, 0.0, 0.0)),
        ],
        [0.5, 0.5],
        0.0,
        TAU,
    )
}
```

A gradient is greedy on both axes. Give it a size with `.size(width, height)`, put it in a `zstack` under your content, or pass it to `.background(...)` (it is a `View`, so it works there). Because it is a view, the usual modifiers apply, including clipping to a shape:

```rust
use waterui::prelude::*;
use waterui::color::working::from_linear_srgb;
use waterui::shape::RoundedRectangle;

fn rgb(red: f32, green: f32, blue: f32) -> WorkingColor {
    from_linear_srgb([red, green, blue], 1.0)
}

fn tile() -> impl View {
    Gradient::linear(
        vec![(0.0, rgb(1.0, 0.3, 0.5)), (1.0, rgb(0.3, 0.5, 1.0))],
        [0.0, 0.0],
        [1.0, 1.0],
    )
    .size(100.0, 100.0)
    .clip(RoundedRectangle::new(0.18))
}

fn badge() -> impl View {
    text!("New").background(Gradient::linear(
        vec![(0.0, rgb(0.1, 0.2, 0.6)), (1.0, rgb(0.5, 0.1, 0.5))],
        [0.0, 0.5],
        [1.0, 0.5],
    ))
}
```

`Gradient::gradient_type()` returns a `waterui::gradient::GradientType` (`Linear`, `Radial`, `Angular`, `Mesh`; the enum is non-exhaustive) for backends that map a gradient onto a platform type. You rarely need it in app code. See [Modifiers](../02-core/04-modifiers.md) for `.background`, and [Layout](../03-ui/02-layout.md) for how `.size` and greedy views interact.

### Mesh gradients

`Gradient::mesh(columns, rows, vertices, smooths_colors)` interpolates across a grid of control vertices, listed row-major as `(position, color)`. The grid needs at least two vertices per side and exactly `columns * rows` entries, otherwise it panics. With `smooths_colors` on, colors ease across each patch instead of blending bilinearly, which hides the grid's seams.

```rust
use waterui::prelude::*;
use waterui::color::working::from_linear_srgb;

fn rgb(red: f32, green: f32, blue: f32) -> WorkingColor {
    from_linear_srgb([red, green, blue], 1.0)
}

fn mesh_bg() -> impl View {
    Gradient::mesh(
        2,
        2,
        vec![
            ([0.0, 0.0], rgb(0.0, 0.8, 0.4)),
            ([1.0, 0.0], rgb(0.0, 0.4, 0.8)),
            ([0.0, 1.0], rgb(0.8, 0.4, 0.0)),
            ([1.0, 1.0], rgb(0.8, 0.0, 0.4)),
        ],
        true,
    )
}
```

## Reactive mesh gradients

`Gradient` is fixed. When the colors should change, use `MeshGradient::new(columns, rows, colors)`. `colors` is any signal whose output iterates `WorkingColor`, row-major, so a `Binding<Vec<WorkingColor>>` is the usual choice. The vertices start on an even grid spanning the unit square. A change replaces the recorded paint in place; the view is not rebuilt.

```rust
use waterui::prelude::*;

fn reactive_mesh(colors: Binding<Vec<WorkingColor>>) -> impl View {
    MeshGradient::new(3, 3, colors)
}
```

Two builder methods take signals as well:

- `.points(signal)` moves the control points. The signal's output iterates `[f32; 2]` in unit space, row-major, one per vertex.
- `.smooths_colors(bool_or_signal)` toggles the easing described above. It is on by default.

```rust
use waterui::prelude::*;
use waterui::color::working::from_linear_srgb;

fn rgb(red: f32, green: f32, blue: f32) -> WorkingColor {
    from_linear_srgb([red, green, blue], 1.0)
}

fn warped_mesh() -> impl View {
    let colors = Binding::container(vec![
        rgb(0.1, 0.1, 0.3),
        rgb(0.2, 0.1, 0.4),
        rgb(0.1, 0.2, 0.4),
        rgb(0.3, 0.2, 0.5),
    ]);
    // Pulling the bottom-right vertex inward bends the whole patch.
    let points: Binding<Vec<[f32; 2]>> = Binding::container(vec![
        [0.0, 0.0],
        [1.0, 0.0],
        [0.0, 1.0],
        [0.6, 0.6],
    ]);

    MeshGradient::new(2, 2, colors)
        .points(points)
        .smooths_colors(false)
        .size(300.0, 200.0)
}
```

If a signal delivers a number of colors or points other than `columns * rows`, the view panics, so size your collections from the same constants you pass to `new`. Driving a mesh at frame rate, as the repository's [`examples/gradient`](https://github.com/water-rs/waterui/tree/dev/examples/gradient) does with a `.task(...)` that writes a binding every 16 ms, is the general-purpose route; for the common case of a flowing background, use the next section instead and skip the task. Tasks are covered in [Suspense and async views](../06-advanced/03-suspense.md).

## Self-animating gradients

### AnimatedMeshGradient

`AnimatedMeshGradient` is a 4x4 mesh whose vertices and colors flow with the canvas clock. You do not tick it or write a task for it.

```rust
use waterui::prelude::*;

fn animated_background() -> impl View {
    AnimatedMeshGradient::default()
}
```

The default configuration is `AnimatedMeshGradientConfig::aqua_bloom()`. Configuration has four presets (`aqua_bloom`, `pastel_lagoon`, `soft_blush`, `deep_blue`) and three builders:

- `.speed(f32)` multiplies the engine clock. `0.0` holds the gradient still and lets the engine idle. Negative or non-finite values panic.
- `.warp(f32)` sets the flow strength; the recommended range is `0.0..0.5`. Negative or non-finite values panic.
- `.palette([WorkingColor; 16])` replaces the 4x4 palette, row-major, with the first row along the *bottom* edge. Alpha is ignored. `waterui::gradient::ANIMATED_MESH_PALETTE_LEN` is 16.

```rust
use waterui::prelude::*;

fn bespoke_background() -> impl View {
    AnimatedMeshGradient::new(
        AnimatedMeshGradientConfig::aqua_bloom().speed(0.8).warp(0.3),
    )
}
```

`AnimatedMeshGradient::new` takes `impl IntoComputed<AnimatedMeshGradientConfig>`, so the configuration itself can be a signal. Pausing is a signal that maps to `speed(0.0)`:

```rust
use waterui::prelude::*;

fn pausable() -> impl View {
    let animating = Binding::container(true);
    let config = animating.map(|animating| {
        let config = AnimatedMeshGradientConfig::deep_blue();
        if animating { config } else { config.speed(0.0) }
    });

    vstack((
        zstack((
            AnimatedMeshGradient::new(config).size(300.0, 200.0),
            text!("Deep blue").foreground(Color::srgb(255, 255, 255)),
        )),
        toggle("Animate", &animating),
    ))
    .padding()
}
```

The config's fields are private. Read them back with `get_speed()`, `get_warp()` and `get_palette()`.

### FlowingGradient

`FlowingGradient` draws drifting, noise-driven color bands with a GPU shader. It has no configuration. It requires the `gpu` feature, which is on by default.

```rust
use waterui::prelude::*;

fn ambient_bg() -> impl View {
    zstack((
        FlowingGradient::new(),
        text!("Flowing").foreground(Color::srgb(255, 255, 255)),
    ))
    .size(300.0, 200.0)
}
```

Use the shader chapter ([Shaders](03-shaders.md)) when you need your own noise or colors. `FlowingGradient` is the ready-made one.

## Which one to use

| You want | Use |
| --- | --- |
| A fixed wash behind content | `Gradient::linear / radial / angular` |
| A fixed mesh | `Gradient::mesh` |
| Colors or control points you update from state | `MeshGradient` |
| A moving background with presets, no code per frame | `AnimatedMeshGradient` |
| Drifting color bands from a GPU shader | `FlowingGradient` |

## Cost

Linear, radial, and angular gradients render as native gradients where the backend has one. A mesh has no native primitive on every platform, so mesh gradients (static, reactive, and animated) are drawn by the engine. `AnimatedMeshGradient` presents a frame on every refresh while its speed is above zero and none at `0.0`. `FlowingGradient` always animates and has no speed, so the only way to stop it is to remove it from the tree. Measure the cost on your own device with [`water bench`](../07-tools/04-benchmarks.md).

## Next

Gradients fill a box. The [particles chapter](05-particles.md) fills it with moving elements, and the [animation chapter](../06-advanced/01-animation.md) covers animating values you own.

# Shaders

> **In this chapter, you will:**
> - Paint a view with your own WGSL fragment shader using `shader!` and `ShaderPaintView`
> - Read the built-in uniforms and pass your own values that follow a signal
> - Animate a shader only while you need it to
> - Write colors in the right space for the engine's output
> - Post-process any view with a `ShaderEffect` through `.effect(...)`

WaterUI gives you two ways to put your own WGSL on screen, and they answer different questions:

- **`ShaderPaintView`** *generates* pixels. The shader has no input; it fills the view's rectangle from `uv`, time and your parameters. Starfields, plasma and procedural backgrounds belong here.
- **`ShaderEffect`** *transforms* pixels. The shader receives the rendered content of a view as a texture and writes the result. CRT scanlines, distortion and color grading belong here.

Both need the `gpu` feature of the `waterui` crate, which is on by default. When your shader needs its own pipelines, buffers or a compute pass, step down to [`GpuContent`](02-gpu-surface.md).

Shader source lives in its own `.wgsl` file, never in a Rust string literal. You get syntax highlighting, and the file can be read on its own.

## Painting a view: `shader!`

```rust,ignore
use waterui::graphics::shader;
use waterui::prelude::*;

fn starfield() -> impl View {
    shader!("starfield.wgsl").animated(true).size(400.0, 500.0)
}
```

`shader!` expands to `ShaderPaintView::new(include_str!("starfield.wgsl"))`. Because `include_str!` resolves relative to the file that contains the call, `starfield.wgsl` sits **next to that Rust source file**. The fence is `rust,ignore` because the book crate has no such file; `examples/starfield` in the WaterUI repository is the working version.

A `ShaderPaintView` stretches on both axes, so constrain it with `.size(w, h)`, `.width(..)`, `.height(..)` or the `min_`/`max_` variants like any other greedy view (see [layout](../03-ui/02-layout.md)).

`ShaderPaintView::new` does not validate the WGSL. The module is compiled when the view is first set up on the GPU, so a mistake in a paint shader shows up when the view renders, not when you build it.

## What your fragment can use

You write only the fragment stage. WaterUI puts a prelude in front of your file that provides the uniforms, the parameter block and a full-screen vertex stage:

```wgsl
struct Uniforms {
    time: f32,
    _padding: f32,
    resolution: vec2<f32>,
}
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<uniform> params: array<vec4<f32>, 16>;
```

| Name | Meaning |
| --- | --- |
| `uniforms.time` | Seconds since the view's GPU content was set up |
| `uniforms.resolution` | The render target size in pixels |
| `params` | Your values, sixteen `vec4` (see below) |
| `uv` | The fragment's position in `[0, 1]`, **`(0, 0)` at the top-left** |

The vertex stage is also provided (`vs_main`, a single oversized triangle that covers the view, writing a `VertexOutput` struct). `Uniforms`, `uniforms`, `params`, `VertexOutput` and `vs_main` are therefore taken; do not declare them again. Your file defines the fragment entry point, which must be named `main`, and takes `uv` at location 0:

```wgsl
@fragment
fn main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let pulse = 0.5 + 0.5 * sin(uniforms.time * 2.0);
    let color = vec3<f32>(uv.x, uv.y, pulse);
    return vec4<f32>(cherenkov_linear_srgb(color), 1.0);
}
```

Names starting with `cherenkov_` come from the engine and are in scope without declaring them, as are the engine's color-space declarations named `space_*` and `SPACE_*`.

### Aspect-correct coordinates

`uv` is normalized per axis, so a circle drawn from it stretches with the view. Correct it with the resolution:

```wgsl
let aspect = uniforms.resolution.x / uniforms.resolution.y;
let p = vec2<f32>((uv.x - 0.5) * aspect, uv.y - 0.5);
```

`examples/starfield/src/starfield.wgsl` does exactly this, then layers noise and stars on `p`.

## Color output

A shader paints into the engine's working space: **premultiplied, linear Display P3**. Returning sRGB values unconverted shifts both brightness and saturation. The engine's helpers convert for you:

| Helper | Converts |
| --- | --- |
| `cherenkov_linear_srgb(rgb: vec3)` | linear sRGB to the working space |
| `cherenkov_srgb_decode(rgb: vec3)` | the sRGB transfer curve, decoding to linear |
| `cherenkov_srgb(color: vec4)` | straight-alpha sRGB to premultiplied working color |
| `cherenkov_premultiplied_srgb(color: vec4)` | encoded premultiplied sRGB, as browser compositors produce it |

Do your lighting math in linear space, then pass the result through `cherenkov_linear_srgb`. If you write a translucent color, multiply the color by its alpha yourself.

## Passing parameters

`ShaderPaintView::uniforms` takes a `Vec<f32>` that follows a signal. Each change reaches the shader without rebuilding the view, and an idle (non-animated) shader re-renders once per change. At most 64 floats are accepted (sixteen `vec4`); a longer list panics when read.

```rust
use waterui::graphics::ShaderPaintView;
use waterui::prelude::*;

fn tunable(fragment: &'static str, params: Binding<Vec<f32>>, running: Binding<bool>) -> impl View {
    ShaderPaintView::new(fragment)
        .uniforms(params)
        .animated(running)
        .size(320.0, 240.0)
}
```

The floats are packed flat, so element `i` is `params[i / 4u][i % 4u]` in WGSL:

```wgsl
@fragment
fn main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let speed = params[0].x;
    let hue = params[0].y;
    let wave = 0.5 + 0.5 * sin(uv.x * 10.0 + uniforms.time * speed);
    return vec4<f32>(cherenkov_linear_srgb(vec3<f32>(wave, hue, 1.0 - wave)), 1.0);
}
```

Because `uniforms` takes `impl IntoComputed<Vec<f32>>`, a plain `Vec<f32>`, a `Binding<Vec<f32>>` or a `Computed` that packs several slider bindings into one list all work. Signals are covered in [Reactive state](../02-core/02-reactive.md).

## Animating on demand

`.animated(signal)` decides whether the shader keeps rendering:

- While the signal is `true`, the view asks for another frame after every frame, and `uniforms.time` advances.
- While it is `false`, the shader renders once per change of its parameters or size, and the host stays idle.

`.animated(true)` is the constant case. Bind it to a `Binding<bool>` (as in the example above) to pause a background when it is off-screen or when the user prefers reduced motion. A paused shader requests no frames.

## Post-processing a view: `.effect(...)`

A `ShaderEffect` runs your fragment over the **captured content of a view**. Build it from WGSL source, then apply it with `.effect(...)` from `FilterViewExt`, which the prelude brings in:

```rust,ignore
use waterui::graphics::filter_view::{ShaderEffect, ShaderEffectError};
use waterui::prelude::*;

fn scanlines(content: impl View) -> Result<impl View, ShaderEffectError> {
    let shader = ShaderEffect::new(include_str!("crt.wgsl"))?
        .param(0.35)
        .animated();
    Ok(content.effect(shader))
}
```

`ShaderEffect::param(f32)` adds a constant parameter, and `ShaderEffect::animated()` marks the shader as time-driven, so the host renders another frame after each one. This `crt.wgsl` darkens every other pair of pixel rows by the strength in parameter 0 and flickers faintly:

```wgsl
@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(input_texture, input_sampler, in.uv);
    let row = u32(in.position.y);
    let line = select(1.0, 0.0, (row / 2u) % 2u == 1u);
    let flicker = 0.97 + 0.03 * sin(uniforms.time * 50.0);
    let shade = mix(1.0, line, effect_param(0u)) * flicker;
    return vec4<f32>(color.rgb * shade, color.a);
}
```

An effect shader sees a different prelude from a paint shader. These declarations are available without writing them:

| Name | Meaning |
| --- | --- |
| `input_texture`, `input_sampler` | The captured content, and a bilinear clamp-to-edge sampler |
| `uniforms.resolution`, `uniforms.input_resolution` | Output and input size in physical pixels |
| `uniforms.time`, `uniforms.time_delta` | Seconds on the host timeline, and since the previous frame |
| `uniforms.frame`, `uniforms.param_count` | Frame sequence (wrapping at 2^32) and the number of parameters added |
| `effect_param(index: u32) -> f32` | The parameters you added, in order |
| `VertexOutput` | `position` (output pixel, top-left origin) and `uv` (addresses the input texture) |

The names above and the vertex entry `shader_effect_vs` are reserved. The input carries premultiplied alpha in the working space, and your output must too.

### Errors surface at construction

`ShaderEffect::new` parses and validates the WGSL on the CPU and returns `Result<ShaderEffect, ShaderEffectError>`. The error is one of `Parse`, `Validation` or `MissingEntryPoint` (no `@fragment fn main`), with line and column numbers counted in *your* source, so no invalid shader reaches the renderer. Validation uses the capabilities of a core WebGPU device, so a shader that validates runs on every device; the one device-dependent check, that the input format can be filtered when your shader reads `input_sampler`, happens when the effect is set up on the GPU.

A shader compiled into the binary propagates the error with `?`, as above. If you load a shader at run time, from a user's configuration file for example, handle the error where you read it and report it through the facade's `waterui::log` (the `tracing` crate):

```rust
use waterui::graphics::filter_view::{ShaderEffect, ShaderEffectError};
use waterui::prelude::*;

fn load_effect(source: &str) -> Option<ShaderEffect> {
    match ShaderEffect::new(source) {
        Ok(effect) => Some(effect.param(0.35)),
        Err(error) => {
            waterui::log::error!(%error, "rejected user shader");
            None
        }
    }
}

fn is_missing_entry(error: &ShaderEffectError) -> bool {
    matches!(error, ShaderEffectError::MissingEntryPoint)
}
```

### Reactive parameters

`.effect(...)` returns a `WithEffect` (`waterui::graphics::WithEffect`). For a `ShaderEffect`, its `param` takes any signal of a primitive number (`impl IntoSignalF32`, such as `Binding<f32>`) and its `animated` takes `impl IntoComputed<bool>`, so neither change rebuilds the view. Parameters are numbered in the order you add them, so the constants the `ShaderEffect` already carries come first:

```rust
use waterui::graphics::filter_view::ShaderEffect;
use waterui::prelude::*;

fn crt(
    content: impl View,
    shader: ShaderEffect,
    strength: Binding<f32>,
    running: Binding<bool>,
) -> impl View {
    content.effect(shader).param(strength).animated(running)
}
```

A change to `strength` re-renders the effect and follows any animation the change carries (see [Animation](../06-advanced/01-animation.md)). An effect holds at most `waterui::graphics::SHADER_EFFECT_MAX_PARAMS` (16) parameters, constants and signals together. Adding a seventeenth is a contract violation: it panics in a debug build, and in a release build the extra parameter is logged and dropped.

For built-in filters such as blur or brightness, which fuse into shared passes and need no WGSL, see [Filters](04-filters.md).

## Which one to use

| You want to... | Use |
| --- | --- |
| Fill a rectangle with generated pixels | `shader!` / `ShaderPaintView` |
| Change how existing views look | `.effect(ShaderEffect)` |
| A standard blur, tint or sharpen | a [filter](04-filters.md) |
| Your own pipelines, buffers or compute | [`GpuContent`](02-gpu-surface.md) |

Next: [Filters](04-filters.md).

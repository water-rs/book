# Shaders

> **In this chapter, you will:**
> - Write WGSL fragment shaders and display them with `ShaderSurface`
> - Use the built-in uniforms for time, resolution, and aspect-ratio correction
> - Load shader files at compile time with the `shader!` macro
> - Build animated effects like plasma and procedural noise
> - Know when to graduate from `ShaderSurface` to `GpuView`

`ShaderSurface` is the shortest path from "I have a WGSL fragment shader" to "it is on screen." You supply the fragment; WaterUI supplies the vertex stage, the uniform buffer, the pipeline, and the render loop.

## Quick start

```rust,ignore
use waterui::prelude::*;
use waterui::graphics::shader;

fn my_effect() -> impl View {
    shader!("shaders/plasma.wgsl")
}
```

`shader!` reads the WGSL source at compile time with `include_str!` and builds a `ShaderSurface` labeled with the path. **The path is resolved against your crate's `src/` directory, not against the file that calls the macro** — `shader!("shaders/plasma.wgsl")` loads `<your-crate>/src/shaders/plasma.wgsl`.

![ShaderSurface preview with a plasma fragment shader](../assets/visuals/05-graphics/shader-plasma.png)

*A WGSL fragment shader rendered through ShaderSurface. [Example source](https://github.com/water-rs/book/tree/main/examples/book-visuals).*

## Creating a ShaderSurface from a string

When the source is not a fixed file path — generated WGSL, a shader assembled at runtime — construct the surface directly:

```rust,ignore
use waterui::graphics::ShaderSurface;

fn gradient_effect(source: String) -> impl View {
    ShaderSurface::new(source)
}
```

`ShaderSurface::new` accepts anything convertible into `Cow<'static, str>`, so a `&'static str` from `include_str!` works too. Prefer `shader!` when you have a file: it keeps the path in one place and labels the shader for graphics diagnostics.

`include_fragment_shader!("shaders/plasma.wgsl")` gives you the same compile-time load as a value — a `ShaderSource { label, source }` — when you want to hold the source before deciding what to do with it.

## Built-in uniforms

Every `ShaderSurface` shader is prefixed with a fixed prelude, so you never declare this yourself:

```wgsl
struct Uniforms {
    time: f32,             // seconds since the surface was set up
    resolution: vec2<f32>, // surface size in pixels
    _padding: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;
```

The prelude also declares a `VertexOutput` struct and a `vs_main` vertex shader that emits a six-vertex full-screen quad. Your file only defines the fragment stage, and the entry point must be named `main`:

```wgsl
@fragment
fn main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    // uv: (0,0) at bottom-left, (1,1) at top-right
    return vec4<f32>(uv.x, uv.y, sin(uniforms.time) * 0.5 + 0.5, 1.0);
}
```

The exact prelude text is available as the `waterui::graphics::shader_surface::PRELUDE` constant if you need to reproduce the environment in a standalone WGSL tool.

## Writing WGSL

### Time-based animation

`uniforms.time` ticks up continuously, which is all you need for pulsing, rotating, and morphing:

```wgsl
@fragment
fn main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let t = uniforms.time;
    let dist = distance(uv, vec2<f32>(0.5, 0.5));
    let radius = 0.3 + 0.1 * sin(t * 2.0);
    let circle = smoothstep(radius + 0.01, radius - 0.01, dist);

    return vec4<f32>(circle, circle * 0.5, 1.0 - circle, 1.0);
}
```

### Aspect-ratio correction

`uv` is normalized to the surface, so circles turn into ellipses on a non-square surface unless you correct with `uniforms.resolution`:

```wgsl
@fragment
fn main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let res = max(uniforms.resolution, vec2<f32>(1.0));
    let aspect = res.x / res.y;

    let p = vec2<f32>((uv.x - 0.5) * aspect, uv.y - 0.5);
    let ring = smoothstep(0.01, 0.0, abs(length(p) - 0.3));

    return vec4<f32>(ring, ring, ring, 1.0);
}
```

### Procedural noise

A hash-based value noise, the building block for fire, clouds, and terrain:

```wgsl
fn hash21(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453123);
}

@fragment
fn main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let cell = floor(uv * 10.0);
    let n = hash21(cell + vec2<f32>(uniforms.time * 0.1, 0.0));

    return vec4<f32>(n, n, n, 1.0);
}
```

## How ShaderSurface behaves

`ShaderSurface` wraps a `GpuSurface` around an internal `GpuView`:

1. **Setup** concatenates the prelude with your fragment, compiles it into a `wgpu::ShaderModule`, and builds a 24-byte uniform buffer, a bind group, and a render pipeline against the current surface format. Blending is `REPLACE` on SDR surfaces and disabled on HDR ones.
2. **Render** rewrites the uniform buffer with the latest time and resolution, clears to transparent, and draws the six-vertex quad.
3. **Every frame requests the next one.** `ShaderSurface` is unconditionally animated — there is no static mode. If your effect does not use `uniforms.time`, you are paying for frames you do not need; write a `GpuView` that only requests redraws when something changes.
4. **The surface format is fixed at setup.** If it changes afterwards (an HDR toggle, for instance) the renderer panics rather than silently rendering into a mismatched target.

### Compile time vs. run time

WaterUI's own built-in shaders — the mesh gradients, the image generator, the scene blit — are compiled ahead of time during `cargo build` and shipped in the binary as `shaderloom::CompiledShader` constants under `waterui::graphics::shaders`. That replaced the previous approach of compiling lazily on first use and hiding the stall behind a disk-persisted pipeline cache; the pre-warm module and the cache are both gone.

Shaders *you* author through `shader!` or `ShaderSurface::new` are still compiled when the surface sets up. That is a one-time cost per surface, and it is why the format is captured at setup rather than re-checked per frame.

## Accessing the inner GpuSurface

`into_inner()` returns the `GpuSurface`, so you can apply per-surface settings like the MSAA cap or the HDR preference:

```rust,ignore
use core::num::NonZeroU32;

let surface = shader!("shaders/plasma.wgsl")
    .into_inner()
    .msaa_max_samples(NonZeroU32::new(4).unwrap());
```

## When to drop down to GpuView

`ShaderSurface` binds exactly one uniform buffer with time and resolution in it. The moment you need extra uniforms, textures, samplers, storage buffers, or a compute pass, write a `GpuView` and wrap it in a `GpuSurface` — see [GPU rendering with GpuSurface](02-gpu-surface.md). `AnimatedMeshGradient` is the shipped example: it carries a 16-entry color palette as a uniform, which `ShaderSurface` has no way to express.

Two smaller notes for shader authors:

- GPUs prefer uniform control flow. Reach for `select()`, `step()`, and `smoothstep()` before `if`.
- WGSL floats are 32-bit. For pixel-precise work, multiply `uv` by `uniforms.resolution` rather than chasing precision in normalized space.

## Example: a plasma effect

```wgsl
// src/shaders/plasma.wgsl

const PI: f32 = 3.14159265359;

@fragment
fn main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let t = uniforms.time * 0.5;
    let p = uv * 10.0;

    var v = 0.0;
    v += sin(p.x + t);
    v += sin(p.y + t * 0.7);
    v += sin((p.x + p.y) * 0.5 + t * 1.3);
    v += sin(length(p - vec2<f32>(5.0)) + t);

    let r = sin(v * PI) * 0.5 + 0.5;
    let g = sin(v * PI + 2.094) * 0.5 + 0.5;
    let b = sin(v * PI + 4.189) * 0.5 + 0.5;

    return vec4<f32>(r, g, b, 1.0);
}
```

```rust,ignore
fn plasma_background() -> impl View {
    shader!("shaders/plasma.wgsl").size(400.0, 300.0)
}
```

## Next

Shaders compose visual content from scratch. To transform views you already have, continue to [Filters and visual effects](04-filters.md).

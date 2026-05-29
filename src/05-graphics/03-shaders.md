# Shaders

> **In this chapter, you will:**
> - Write WGSL fragment shaders and display them with `ShaderSurface`
> - Use built-in uniforms for time, resolution, and aspect-ratio correction
> - Load shaders at compile time with the `shader!` macro
> - Build animated effects like plasma, noise, and pulsing shapes
> - Know when to graduate from `ShaderSurface` to `GpuView`

`ShaderSurface` is the shortest path from "I have a WGSL fragment shader" to "it is on screen." You supply the fragment, and WaterUI handles pipeline creation, the uniform buffer, and the render loop.

## Quick start

The fastest way to get a shader on screen is the `shader!` macro:

```rust,ignore
use waterui::prelude::*;
use waterui::graphics::shader;

fn my_effect() -> impl View {
    shader!("shaders/plasma.wgsl")
}
```

`shader!` loads the WGSL source at compile time, registers it for pre-warming, and creates a `ShaderSurface` with the file path as a label so the GPU pipeline cache can deduplicate.

![ShaderSurface preview with a plasma fragment shader](../assets/visuals/05-graphics/shader-plasma.png)

*A WGSL fragment shader rendered through ShaderSurface.*

## Creating a ShaderSurface manually

`shader!` is sugar over two more explicit constructors. Reach for them when you need to wire something the macro does not cover (computed paths, generated shader source, and so on).

```rust,ignore
use waterui::graphics::ShaderSurface;

// from a static string -- no cache key
fn gradient_effect() -> impl View {
    ShaderSurface::new(include_str!("shaders/gradient.wgsl"))
}

// with a label for the pipeline cache
fn labeled_effect() -> impl View {
    ShaderSurface::with_label(
        "shaders/gradient.wgsl",
        include_str!("shaders/gradient.wgsl"),
    )
}
```

WaterUI keeps a long shader inline in a string literal off-limits in production code -- always pull from a `.wgsl` file with `include_str!` (or `include_fragment_shader!`).

## Built-in uniforms

Every `ShaderSurface` shader receives a standard uniform buffer automatically. You do not declare this struct yourself -- it is prepended by the `ShaderSurface` prelude:

```wgsl
struct Uniforms {
    time: f32,           // Elapsed time in seconds since creation
    resolution: vec2<f32>, // Surface size in pixels (width, height)
    padding: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;
```

A full-screen quad vertex shader is also provided automatically. Your shader only needs to define a fragment function named `main`:

```wgsl
@fragment
fn main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    // uv: normalized coordinates (0,0) at bottom-left, (1,1) at top-right
    let t = uniforms.time;
    let res = uniforms.resolution;
    return vec4<f32>(uv.x, uv.y, sin(t) * 0.5 + 0.5, 1.0);
}
```

### The prelude

The `ShaderSurface` prelude that is auto-prepended to your shader includes:

1. The `Uniforms` struct and binding declaration
2. A `VertexOutput` struct with `position` and `uv` fields
3. A `vs_main` vertex shader that draws a full-screen quad (6 vertices, 2 triangles)

Your fragment function should be named `main` (not `fs_main`) and accept `@location(0) uv: vec2<f32>`.

## Writing WGSL shaders

Now for the fun part. The patterns below progress from a static gradient to time-warped procedural noise.

### Basic color pattern

```wgsl
@fragment
fn main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    // Horizontal gradient from red to blue
    let r = uv.x;
    let b = 1.0 - uv.x;
    return vec4<f32>(r, 0.0, b, 1.0);
}
```

### Time-based animation

This is where shaders start to feel alive. The `uniforms.time` value ticks up continuously, letting you create pulsing, rotating, and morphing effects:

```wgsl
@fragment
fn main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let t = uniforms.time;

    // Pulsing circle
    let center = vec2<f32>(0.5, 0.5);
    let dist = distance(uv, center);
    let radius = 0.3 + 0.1 * sin(t * 2.0);
    let circle = smoothstep(radius + 0.01, radius - 0.01, dist);

    return vec4<f32>(circle, circle * 0.5, 1.0 - circle, 1.0);
}
```

### Resolution-aware rendering

When your effect needs correct aspect ratio:

```wgsl
@fragment
fn main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let res = max(uniforms.resolution, vec2<f32>(1.0));
    let aspect = res.x / res.y;

    // Correct for aspect ratio
    var p = vec2<f32>((uv.x - 0.5) * aspect, uv.y - 0.5);

    let dist = length(p);
    let ring = smoothstep(0.01, 0.0, abs(dist - 0.3));

    return vec4<f32>(ring, ring, ring, 1.0);
}
```

### Noise and procedural patterns

Here is a simple hash-based noise pattern -- the building block for fire, clouds, terrain, and countless other effects:

```wgsl
fn hash21(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453123);
}

@fragment
fn main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let t = uniforms.time;
    let scale = 10.0;
    let cell = floor(uv * scale);
    let n = hash21(cell + vec2<f32>(t * 0.1, 0.0));

    return vec4<f32>(n, n, n, 1.0);
}
```

## Shader loading macros

WaterUI provides two compile-time macros, both returning a `ShaderSource` (alias `PrewarmedShader`):

### `include_shader!`

Loads a complete WGSL shader with both vertex and fragment stages. Use this when you write your own vertex stage:

```rust,ignore
use waterui::graphics::{include_shader, prewarm::ShaderSource};

static MY_SHADER: ShaderSource = include_shader!("shaders/my_effect.wgsl");
```

### `include_fragment_shader!`

Loads a fragment-only shader. The `ShaderSurface` prelude (uniforms + full-screen quad vertex shader) is prepended at runtime:

```rust,ignore
use waterui::graphics::{include_fragment_shader, prewarm::ShaderSource, ShaderSurface};

static MY_FRAGMENT: ShaderSource = include_fragment_shader!("shaders/my_fragment.wgsl");

ShaderSurface::with_label(MY_FRAGMENT.label, MY_FRAGMENT.source)
```

### The `shader!` convenience macro

`shader!("path.wgsl")` expands to roughly:

```rust,ignore
{
    static SHADER: ShaderSource = include_fragment_shader!("path.wgsl");
    ShaderSurface::with_label(SHADER.label, SHADER.source)
}
```

Reach for it whenever you would otherwise inline a shader path twice.

## How ShaderSurface works internally

Under the hood, `ShaderSurface` wraps a `GpuSurface` with an internal `ShaderRenderer` (a `GpuView`):

1. **Setup**: the full WGSL source (prelude + your fragment) is compiled into a `wgpu::ShaderModule`. A 24-byte uniform buffer, bind group, and render pipeline are created against the current surface format.
2. **Render**: each frame the uniform buffer is rewritten with the latest time and resolution, then a 6-vertex full-screen quad is drawn with your shader.
3. **Continuous animation**: `ShaderRenderer` calls `frame.request_redraw()` so time-based animations advance every frame.
4. **Format safety**: if the surface format changes between setup and render (HDR toggle, for instance) the pipeline is invalidated and rebuilt.

## Accessing the inner GpuSurface

If you need the underlying `GpuSurface` (to apply a per-surface MSAA cap, or stack with other GPU views), unwrap it:

```rust,ignore
let surface = ShaderSurface::new(my_shader).into_inner();
```

## Going beyond: custom uniforms

`ShaderSurface` provides only the standard uniforms (time, resolution). If you need extra uniforms, samplers, textures, or storage buffers, write your own `GpuView` and wrap it in a `GpuSurface`. See [GPU rendering with GpuSurface](02-gpu-surface.md). The shipped `AnimatedMeshGradient` is a good example -- it carries a 4x4 palette array uniform, which is exactly the kind of thing `ShaderSurface` will not give you.

## Performance tips

- **Shader compilation**: WGSL is compiled at setup time. Use `shader!` or `with_label` so the backend can cache compiled pipelines via the pre-warm system.
- **Avoid branching**: GPUs prefer uniform control flow. Replace branches with `select()`, `step()`, and `smoothstep()` where possible.
- **Texture reads**: `ShaderSurface` does not expose texture bindings. If you need to sample images, drop down to `GpuView`.
- **Precision**: WGSL is 32-bit float by default. For pixel-precise work, multiply UVs by `uniforms.resolution`.
- **Pipeline cache**: WaterUI threads a `PipelineCache` through setup; the `shader!` and `with_label` paths automatically take advantage of it.

## Example: a plasma effect

Let's put it all together with a classic plasma shader -- the kind of swirling, colorful effect that has mesmerized programmers since the demoscene era:

```wgsl
// shaders/plasma.wgsl

const PI: f32 = 3.14159265359;

@fragment
fn main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let t = uniforms.time * 0.5;
    let res = max(uniforms.resolution, vec2<f32>(1.0));

    var p = uv * 10.0;

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

Use it in your app:

```rust,ignore
fn plasma_background() -> impl View {
    shader!("shaders/plasma.wgsl").size(400.0, 300.0)
}
```

## Next

Shaders compose visual effects from scratch. To transform views you already have, continue to [Filters and visual effects](04-filters.md).

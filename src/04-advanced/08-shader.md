# Shaders

WaterUI supports custom GPU shaders via `waterui::graphics::ShaderSurface`. You can write WGSL
fragment shaders that render directly to the view's surface.

## Using ShaderSurface

The easiest way to use shaders is with the `shader!` macro, which loads a WGSL file at compile time:

```rust
use waterui::prelude::*;
use waterui::graphics::shader;

fn flame_effect() -> impl View {
    shader!("flame.wgsl")
        .width(400.0).height(500.0)
}
```

Or define the shader inline:

```rust
use waterui::graphics::ShaderSurface;

fn gradient() -> impl View {
    ShaderSurface::new(r#"
        @fragment
        fn main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
            return vec4<f32>(uv.x, uv.y, 0.5, 1.0);
        }
    "#)
}
```

## Built-in Uniforms

`ShaderSurface` automatically provides a uniform buffer at `@group(0) @binding(0)` containing:

```wgsl
struct Uniforms {
    time: f32,           // Elapsed time in seconds
    resolution: vec2<f32>, // Surface size in pixels
    _padding: f32,
}
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
```

Your shader entry point should look like this:

```wgsl
@fragment
fn main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    // ...
}
```

`uv` coordinates are normalized (0.0 to 1.0).

## Advanced GPU Rendering

For full control over the rendering pipeline (custom vertex buffers, compute shaders, multiple passes),
implement the `GpuRenderer` trait and use `GpuSurface`.

```rust
use waterui::graphics::{GpuRenderer, GpuSurface, GpuContext, GpuFrame};

struct MyRenderer;

impl GpuRenderer for MyRenderer {
    fn setup(&mut self, ctx: &GpuContext) {
        // Create pipelines, buffers...
    }

    fn render(&mut self, frame: &GpuFrame) {
        // Encode render commands...
    }
}

fn custom_render() -> impl View {
    GpuSurface::new(MyRenderer)
}
```

This allows for high-performance, custom GPU effects integrated seamlessly into the UI.
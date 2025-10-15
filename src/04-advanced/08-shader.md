# GPU shaders

For GPU backends compiled with the `wgpu` feature, WaterUI exposes a `Shader` view that renders WGSL
programs into a `RendererView`. This is useful for backgrounds, particle systems, or other effects
that benefit from programmable pipelines.

## Creating a shader view

Construct a shader from WGSL source and attach it to your layout like any other view.

```rust,ignore
use waterui::prelude::*;
use waterui::component::graphics::Shader;

const SOURCE: &str = r#"
@vertex
fn vs_main(@builtin(vertex_index) vertex: u32) -> @builtin(position) vec4<f32> {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let pos = positions[vertex];
    return vec4<f32>(pos, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.1, 0.4, 0.9, 1.0);
}
"#;

fn hero_background() -> impl View {
    Shader::from_wgsl(SOURCE)
        .width(320.0)
        .height(200.0)
}
```

The view compiles the shader the first time it receives a GPU surface and caches the pipeline for
subsequent frames. Changing the surface format (for example due to HDR support) triggers a rebuild.

## Custom entry points

Override the vertex or fragment entry names if your module uses different function names.

```rust,ignore
fn with_custom_entries(source: &'static str) -> Shader {
    Shader::from_wgsl(source)
        .vertex_entry("main_vs")
        .fragment_entry("main_fs")
}
```

## Fallback behaviour

When WaterUI runs on a backend without `wgpu` support, the shader view clears the CPU surface to
transparent so your layout remains predictable. Pair shaders with traditional views to provide
content on platforms that lack GPU acceleration.

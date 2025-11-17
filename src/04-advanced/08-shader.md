# Shaders

Custom shaders are still experimental in WaterUI, but the architecture already exposes two hooks
you can use today:

1. **Native views** – Wrap a platform-specific shader view just like the canvas chapter described.
2. **Metadata** – Pass shader parameters through the environment so renderers can pipe them into the
   GPU pipeline.

## Embedding a Metal/WebGL View

On Apple platforms, create a SwiftUI `Representable` that hosts a Metal view. Expose it to Rust as a
`Native<MyShader>` view (see `component::native`). Provide a lightweight Rust struct with the shader
configuration (uniforms, textures) and let the backend translate it to GPU calls.

## Declarative Uniforms

Even without an official shader DSL, you can keep uniforms reactive by storing them in bindings:

```rust
use waterui::prelude::*;
use waterui::reactive::binding;
use waterui::Binding;
use waterui::core::{Native, View};

pub struct PlasmaUniforms {
    pub time: f32,
    pub intensity: f32,
}

pub fn plasma() -> impl View {
    let time: Binding<f32> = binding(0.0).animated();
    Native(PlasmaUniforms {
        time: time.get(),
        intensity: 0.8,
    })
}
```

The backend reads `PlasmaUniforms` each frame and updates the shader. Once the official shader API
lands, those bindings will plug directly into the declarative shader graph.

## Roadmap

- Shared WGSL shading language compiled to Metal, WebGPU, and Vulkan targets.
- Editor tooling for hot-reloading shader parameters.
- Integration with the layout system so shaders participate in hit-testing and accessibility.

For now, treat shaders like any other native integration: define a Rust struct for configuration,
render it with a backend-specific view, and keep the data reactive.

# GPU rendering with GpuSurface

> **In this chapter, you will:**
> - Implement the `GpuView` trait for custom GPU rendering
> - Understand the setup, resize, and render lifecycle
> - Handle pointer and gesture input inside GPU surfaces
> - Use offscreen rendering for visual testing
> - Configure HDR and MSAA for production-quality output

`GpuSurface` is the foundation of every GPU-rendered view in WaterUI. It hands you a wgpu device, queue, and a per-frame texture, and renders whatever you draw straight onto the platform's swapchain. `Canvas`, `ShaderSurface`, `AnimatedMeshGradient`, `Gradient`, and `ParticleSystem` are all built on top of it.

If `Canvas` is a paintbrush, `GpuSurface` is the bare canvas frame and a tube of pigment.

## Architecture

`GpuSurface` is a *raw view*. The native backend allocates the wgpu surface and swapchain for it, and calls your renderer for every frame.

```
your code (impl GpuView) <-> GpuSurface <-> Native backend (Swift / Kotlin / Hydrolysis)
                                                    |
                                              wgpu device + queue
                                                    |
                                              Metal / Vulkan / GL
```

A `GpuSurface` owns exactly one `GpuView` instance for its lifetime. `GpuView::setup` is the only place persistent GPU resources for that instance live. Do not move that state into shared caches to survive teardown -- when the surface is dropped, the renderer should drop with it.

### Layout behaviour

`GpuSurface` stretches to fill its parent on both axes (the same default as `Color`). Use `.size(w, h)` (from `ViewExt`) when you want a fixed footprint:

```rust
use waterui::prelude::*;
use waterui_graphics::GpuSurface;

GpuSurface::new(MyRenderer::default())             // fills available space
GpuSurface::new(MyRenderer::default()).size(400.0, 300.0) // fixed
```

## The GpuView trait

```rust
use waterui_graphics::{GpuContext, GpuFrame};

pub trait GpuView: 'static {
    async fn setup(
        &mut self,
        ctx: &GpuContext<'_>,
        env: &mut waterui_core::Environment,
    );

    fn render(&mut self, frame: &mut GpuFrame);
}
```

A few details that the type signature does not show:

- `setup` is `async`. Awaitable work (asset loading, shader compilation queues) is allowed; for synchronous setup, the body is just straight-line Rust.
- `render` receives the frame by `&mut` so you can call `frame.request_redraw()` to schedule another frame for animation.
- `GpuView` requires a `SubView` impl for layout. Use the `impl_gpu_subview!` macro at the concrete impl site -- it wires the default `StretchAxis::Both` layout for you.

### Lifecycle

1. **Setup** runs once after the wgpu device is ready. Build pipelines, buffers, bind groups, and any owned textures here. Clone `ctx.redraw_handle` if you need to wake the surface from outside the render loop (for example, when a `nami` signal changes, a timer fires, or a network response arrives).
2. **Resize** is implicit: each call to `render` carries the current `frame.width`/`frame.height`. Recreate size-dependent resources by detecting a size change inside `render`.
3. **Render** runs whenever the surface is dirty. Submit your wgpu commands into `frame.queue`. Call `frame.request_redraw()` to ask for the next frame.

There is no separate `needs_redraw` callback. Frames advance because either the surface dirtied (size, input, theme), the renderer requested another frame, or a `RedrawHandle` was poked.

## GpuContext

`GpuContext` is the setup-time payload:

```rust
pub struct GpuContext<'a> {
    pub adapter: Option<&'a wgpu::Adapter>,
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub surface_format: wgpu::TextureFormat,
    pub msaa_samples: u32,
    pub pipeline_cache: Option<&'a wgpu::PipelineCache>,
    pub redraw_handle: RedrawHandle,
}
```

- `surface_format` may be `Rgba16Float` when the platform supports HDR. Call `ctx.is_hdr()` and gate your blend state on that result -- when HDR is active, use `blend: None` rather than `BlendState::REPLACE`.
- `msaa_samples` reflects the backend's supported sample count, capped by `WATERUI_GPU_MSAA` (default 4). Use it for both pipeline configuration and any MSAA attachments you create.
- `pipeline_cache`, when present, should be threaded into every `RenderPipelineDescriptor`. It dramatically reduces pipeline compile time on subsequent launches.
- `redraw_handle` is a cheap, thread-safe handle. Clone and stash it; call `request_redraw()` whenever new state should drive a frame.

## GpuFrame

```rust
pub struct GpuFrame<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub texture: &'a wgpu::Texture,
    pub view: wgpu::TextureView,
    pub format: wgpu::TextureFormat,
    pub width: u32,
    pub height: u32,
    pub pointer: PointerState,
    pub gesture: GestureState,
    // ...
}
```

- `frame.elapsed()` returns the accumulated animation time since the surface was first presented; `frame.delta()` is the gap since the previous frame.
- `frame.is_hovering()`, `frame.pointer_normalized()` give you quick access to pointer interaction.
- `frame.gesture` exposes pinch/pan/double-tap state forwarded by the backend.
- `frame.request_redraw()` schedules another frame. `frame.was_redraw_requested()` lets nested helpers check the flag.

## Triangle example

Here is a complete "hello triangle" implementation. The shader lives in its own file, as required for anything beyond a couple of lines.

```rust
// triangle.rs
use waterui::prelude::*;
use waterui_graphics::{GpuContext, GpuFrame, GpuSurface, GpuView, impl_gpu_subview, wgpu};

#[derive(Default)]
struct TriangleRenderer {
    pipeline: Option<wgpu::RenderPipeline>,
}

impl GpuView for TriangleRenderer {
    async fn setup(
        &mut self,
        ctx: &GpuContext<'_>,
        _env: &mut waterui_core::Environment,
    ) {
        let shader = ctx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("triangle"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/triangle.wgsl").into()),
        });

        let layout = ctx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("triangle-layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let blend = if ctx.is_hdr() { None } else { Some(wgpu::BlendState::REPLACE) };

        self.pipeline = Some(ctx.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("triangle-pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: ctx.surface_format,
                    blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: ctx.pipeline_cache,
        }));
    }

    fn render(&mut self, frame: &mut GpuFrame) {
        let Some(pipeline) = &self.pipeline else { return };

        let mut encoder = frame.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("triangle-encoder"),
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("triangle-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(pipeline);
            pass.draw(0..3, 0..1);
        }

        frame.queue.submit(std::iter::once(encoder.finish()));
    }
}

impl_gpu_subview!(TriangleRenderer);

pub fn triangle_view() -> impl View {
    GpuSurface::new(TriangleRenderer::default())
}
```

The macro `impl_gpu_subview!` provides the layout hooks (`StretchAxis::Both`, default priority) so `GpuSurface` can wrap your renderer as a view.

## Interactive rendering

Pointer and gesture state arrive on every frame. There is nothing to subscribe to -- just read it.

```rust
fn render(&mut self, frame: &mut GpuFrame) {
    if let Some((nx, ny)) = frame.pointer_normalized() {
        self.update_hover(nx, ny);
        frame.request_redraw(); // keep animating while the pointer is over us
    }

    if frame.gesture.is_pinching() {
        self.zoom *= frame.gesture.pinch_scale;
    }

    if frame.gesture.double_tap {
        self.reset_view();
    }
}
```

## Driving redraws from outside

For animations that depend on something other than the frame loop -- a timer, a `Binding`, an incoming network message -- clone `ctx.redraw_handle` during `setup` and call `request_redraw()` from anywhere:

```rust
async fn setup(
    &mut self,
    ctx: &GpuContext<'_>,
    _env: &mut waterui_core::Environment,
) {
    let redraw = ctx.redraw_handle.clone();
    self.guard = Some(self.signal.watch(move |_| redraw.request_redraw()));
    // ...
}
```

This pattern is exactly how `MeshGradient` reacts to its color signal without the surface being torn down.

## Offscreen rendering

`GpuSurface` ships with offscreen rendering for visual tests and CI snapshots. Always render to GPU here -- never read back the swapchain texture in a runtime path.

```rust
use std::num::NonZeroU32;
use waterui_graphics::{GpuSurface, OffscreenRenderConfig, OffscreenSize, wgpu};

let size = OffscreenSize::try_from_pixels(1024, 768)?;
let config = OffscreenRenderConfig::new(size)
    .format(wgpu::TextureFormat::Rgba8UnormSrgb);

let output = GpuSurface::new(MyRenderer::default())
    .render_offscreen(config, &mut env)?;

assert_eq!(output.rgba8.len(), 1024 * 768 * 4);
output.save_png("snapshot.png")?;
```

For HDR snapshots, swap the format and the entry point:

```rust
let config = OffscreenRenderConfig::new(size)
    .format(wgpu::TextureFormat::Rgba16Float);

let output = GpuSurface::new(MyHdrRenderer::default())
    .render_offscreen_hdr(config, &mut env)?;

let max_luminance = output.max_rgb_linear();
let hdr_ratio = output.hdr_pixel_ratio();

output.save_png("hdr_snapshot.png")?;     // PQ-coded HDR PNG with cICP metadata
output.save_sdr_png("sdr_snapshot.png")?; // tone-mapped SDR fallback
```

`OffscreenRenderConfig` lets you simulate input for hover/gesture tests:

```rust
use waterui_core::layout::Point;
use waterui_graphics::{GestureState, PointerState};

let config = OffscreenRenderConfig::new(size)
    .format(wgpu::TextureFormat::Rgba8UnormSrgb)
    .msaa_samples(NonZeroU32::new(4).unwrap())
    .pointer(PointerState {
        position: Some(Point::new(512.0, 384.0)),
        hit: None,
    })
    .gesture(GestureState::new());
```

## MSAA configuration

```rust
use std::num::NonZeroU32;

GpuSurface::new(MyRenderer::default())
    .msaa_max_samples(NonZeroU32::new(8).unwrap())
```

Globally: set `WATERUI_GPU_MSAA=4` (accepts 1, 2, 4, 8, or 16). The backend clamps the request to what the adapter and format actually support.

## HDR preference

WaterUI defaults to HDR (`Rgba16Float`) when the platform offers it. To opt out for a single surface:

```rust
GpuSurface::new(MyRenderer::default()).prefer_sdr_surface();
```

Globally: `WATERUI_GPU_PREFER_HDR=0` forces SDR. In your renderer, gate blend state on `ctx.is_hdr()` so the same code compiles into either pipeline.

## Reference

| Item | Role |
|------|------|
| `GpuView` | Trait you implement for custom GPU rendering |
| `GpuContext` | Setup-time wgpu handles + redraw handle |
| `GpuFrame` | Per-frame texture, pointer, gesture, timing |
| `GpuSurface` | Raw view that owns a single `GpuView` instance |
| `RedrawHandle` | Wakes the surface from outside the render loop |
| `OffscreenRenderConfig` | Headless render configuration |
| `OffscreenRenderOutput` | SDR pixel output with PNG encoding |
| `OffscreenRenderOutputHdr` | HDR pixel output with PQ + tone-mapped PNG |
| `impl_gpu_subview!` | Layout glue at the impl site |

## Next

For the most common GPU use case -- a single fragment shader full-screen quad -- `ShaderSurface` skips most of this boilerplate. Continue to [Shaders](03-shaders.md).

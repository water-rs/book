# GPU rendering with GpuSurface

> **In this chapter, you will:**
> - Implement the `GpuView` trait for custom GPU rendering
> - Understand the setup, resize, and render lifecycle
> - Handle pointer and gesture input inside GPU surfaces
> - Render offscreen for visual tests using an explicit `GpuRuntime`
> - Configure HDR and MSAA per surface

`GpuSurface` is the foundation of every GPU-rendered view in WaterUI. It hands you a wgpu device, queue, and a per-frame texture, and renders whatever you draw straight onto the platform's swapchain. `Canvas`, `ShaderSurface`, `AnimatedMeshGradient`, `Gradient`, and `ParticleSystem` are all built on top of it.

`waterui_graphics` does not re-export `wgpu`, so a crate implementing `GpuView` depends on it directly. The version must match the one WaterUI links:

```toml
[dependencies]
wgpu = "29"
```

![GPU surface preview rendering a colored triangle](../assets/visuals/05-graphics/gpu-surface-triangle.png)

*A custom GpuView rendered through water preview. [Example source](https://github.com/water-rs/book/tree/main/examples/book-visuals).*

## Architecture

`GpuSurface` is a *raw view*. The native backend allocates the wgpu surface and swapchain for it, and calls your renderer for every frame.

```text
your code (impl GpuView) <-> GpuSurface <-> Native backend (Swift / Kotlin / Hydrolysis)
                                                    |
                                              wgpu device + queue
                                                    |
                                              Metal / Vulkan / GL
```

A `GpuSurface` owns exactly one `GpuView` instance for its lifetime, and `GpuView::setup` is the only place persistent GPU resources for that instance live. Do not move that state into shared caches to survive teardown — when the surface is dropped, the renderer should drop with it.

### Layout behavior

`GpuSurface` stretches to fill its parent on both axes. Use `.size(w, h)` (from `ViewExt`) when you want a fixed footprint:

```rust,ignore
use waterui::prelude::*;
use waterui::graphics::GpuSurface;

GpuSurface::new(MyRenderer::default())                    // fills available space
GpuSurface::new(MyRenderer::default()).size(400.0, 300.0) // fixed
```

## The GpuView trait

```rust,ignore
use waterui::Environment;
use waterui::graphics::{GpuContext, GpuFrame};
use waterui::layout::{ProposalSize, Size, StretchAxis, ViewDimensions};

pub trait GpuView: 'static {
    async fn setup(&mut self, ctx: &GpuContext<'_>, env: &mut Environment);

    fn render(&mut self, frame: &mut GpuFrame);

    // Everything below has a default implementation.
    fn preferred_surface_hdr(&self) -> Option<bool> { None }
    fn measure(&self, proposal: ProposalSize) -> ViewDimensions { /* fills the proposal */ }
    fn stretch_axis(&self) -> StretchAxis { StretchAxis::Both }
    fn priority(&self) -> i32 { 0 }
}
```

Only `setup` and `render` are required. The layout hooks are defaults on the trait itself — earlier releases required a separate `SubView` implementation plus an `impl_gpu_subview!` macro call, and both are gone. Override `measure` when your content has an intrinsic size (an image respecting its aspect ratio, for example) rather than filling whatever it is offered.

`setup` is `async`, so awaitable initialization — asset loading, shader-source fetching — is allowed. The future is not required to be `Send`: it is created and awaited on the same thread. Push heavy CPU work onto a thread pool (`smol::unblock`) rather than blocking it.

### Lifecycle

1. **Setup** runs once, after the wgpu device is ready. Build pipelines, buffers, bind groups, and owned textures here. Clone `ctx.redraw_handle` if you need to wake the surface from outside the render loop.
2. **Resize** is implicit: every `render` call carries the current `frame.width`/`frame.height`. Detect a size change there and recreate size-dependent resources.
3. **Render** runs whenever the surface is dirty. Submit your wgpu commands through `frame.queue`, and call `frame.request_redraw()` to ask for another frame.

There is no separate `needs_redraw` callback. A frame happens because the surface dirtied (size, input, theme), the renderer requested one, or a `RedrawHandle` was poked.

## GpuContext

`GpuContext` is the setup-time payload:

```rust,ignore
pub struct GpuContext<'a> {
    pub adapter: &'a wgpu::Adapter,
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub surface_format: wgpu::TextureFormat,
    pub msaa_samples: u32,
    pub redraw_handle: RedrawHandle,
}
```

- `adapter` is always present. Use it for `get_texture_format_features` when you need to know what the hardware supports.
- `surface_format` may be `Rgba16Float` when the platform supports HDR. Call `ctx.is_hdr()` and gate your blend state on it — with HDR active, use `blend: None` rather than `BlendState::REPLACE`.
- `msaa_samples` is the sample count the backend selected for this surface. Use it for both pipeline configuration and any MSAA attachments you create.
- `redraw_handle` is a cheap, thread-safe handle. Clone and stash it; call `request_redraw()` whenever new state should drive a frame. It also exposes `is_dirty()`, `take_dirty()`, and `set_waker(...)` for hosts driving their own frame loop.

There is no `pipeline_cache` field. WaterUI's own shaders are compiled ahead of time at build time (see [Shaders](03-shaders.md)), and the wgpu pipeline-cache plumbing that used to be threaded through here was removed along with the runtime pre-warm system. Pass `cache: None` in your pipeline descriptors.

## GpuFrame

```rust,ignore
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

- `frame.elapsed()` is the accumulated animation time since the surface started rendering; `frame.delta()` is the frame-to-frame time step.
- `frame.is_hovering()` and `frame.pointer_normalized()` give quick access to pointer state.
- `frame.gesture` carries pinch/pan/double-tap state forwarded by the backend.
- `frame.request_redraw()` schedules another frame; `frame.was_redraw_requested()` lets nested helpers read the flag back.

## Triangle example

A complete "hello triangle". The shader lives in its own file — WGSL does not belong in a string literal.

```rust,ignore
// triangle.rs
use waterui::{Environment, prelude::*};
use waterui::graphics::{GpuContext, GpuFrame, GpuSurface, GpuView};

#[derive(Default)]
struct TriangleRenderer {
    pipeline: Option<wgpu::RenderPipeline>,
}

impl GpuView for TriangleRenderer {
    async fn setup(&mut self, ctx: &GpuContext<'_>, env: &mut Environment) {
        let shader = ctx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("triangle"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/triangle.wgsl").into()),
        });

        let layout = ctx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("triangle-layout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });

        let blend = (!ctx.is_hdr()).then_some(wgpu::BlendState::REPLACE);

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
            multiview_mask: None,
            cache: None,
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
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(pipeline);
            pass.draw(0..3, 0..1);
        }

        frame.queue.submit([encoder.finish()]);
    }
}

pub fn triangle_view() -> impl View {
    GpuSurface::new(TriangleRenderer::default())
}
```

## Interactive rendering

Pointer and gesture state arrive on every frame. There is nothing to subscribe to — just read it.

```rust,ignore
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

For anything that changes outside the frame loop — a timer, a `Binding`, an incoming network message — clone `ctx.redraw_handle` during `setup` and call `request_redraw()` from wherever the change lands. Keep the watch guard alive on the renderer; dropping it unsubscribes.

```rust,ignore
async fn setup(&mut self, ctx: &GpuContext<'_>, env: &mut Environment) {
    let redraw = ctx.redraw_handle.clone();
    self.guard = Some(self.signal.watch(move |_| redraw.request_redraw()));
    // ...
}
```

This is how `MeshGradient` tracks its color signal without the surface being torn down and rebuilt.

## Offscreen rendering

`GpuSurface` renders headlessly for visual tests and snapshots. The entry points are `async` and take a `&GpuRuntime` — an explicitly constructed GPU context, which replaced the process-global one that earlier releases reached for implicitly.

```rust,ignore
use waterui::graphics::{
    GpuRuntime, GpuSurface, OffscreenRenderConfig, OffscreenSize,
};

let runtime = GpuRuntime::new().await?;
let mut env = waterui::Environment::new();

let size = OffscreenSize::try_from_pixels(1024, 768)?;
let config = OffscreenRenderConfig::new(size).format(wgpu::TextureFormat::Rgba8Unorm);

let output = GpuSurface::new(MyRenderer::default())
    .render_offscreen(&runtime, config, &mut env)
    .await?;

assert_eq!(output.rgba8.len(), 1024 * 768 * 4);
output.save_png("snapshot.png")?;
```

RGBA readback accepts `Rgba8Unorm` and `Rgba8UnormSrgb`; anything else returns `OffscreenRenderError::UnsupportedReadbackFormat`. `render_offscreen_frames(..., frame_count)` runs several frames before the readback, which is what you want for a renderer that animates from `frame.elapsed()` — offscreen frames advance at a fixed 1/60 s step, so the frame count maps directly to simulated time.

Never read a swapchain texture back to the CPU in a runtime path — offscreen readback exists for tests and snapshot generation.

For HDR, use the `Rgba16Float` entry point. It returns half-float pixels in `output.rgba16f`:

```rust,ignore
let config = OffscreenRenderConfig::new(size).format(wgpu::TextureFormat::Rgba16Float);

let output = GpuSurface::new(MyHdrRenderer::default())
    .render_offscreen_hdr(&runtime, config, &mut env)
    .await?;

output.save_png("hdr_snapshot.png")?;     // PQ-coded HDR PNG when headroom is detected
output.save_sdr_png("sdr_snapshot.png")?; // tone-mapped SDR version
```

`OffscreenRenderConfig` also carries the simulated input used for hover and gesture tests:

```rust,ignore
use core::num::NonZeroU32;
use waterui::layout::Point;
use waterui::graphics::{GestureState, PointerState};

let config = OffscreenRenderConfig::new(size)
    .format(wgpu::TextureFormat::Rgba8Unorm)
    .msaa_samples(NonZeroU32::new(4).unwrap())
    .pointer(PointerState {
        position: Some(Point::new(512.0, 384.0)),
        hit: None,
    })
    .gesture(GestureState::new());
```

## MSAA

Each surface carries a maximum sample count, defaulting to 4. Backends clamp it to what the adapter and format actually support.

```rust,ignore
use core::num::NonZeroU32;

GpuSurface::new(MyRenderer::default())
    .msaa_max_samples(NonZeroU32::new(8).unwrap())
```

`msaa_sample_limit()` reads the configured cap back; `ctx.msaa_samples` in `setup` is the resolved value you should build pipelines against.

## HDR preference

By default a surface follows the surrounding platform style. Override it per surface:

```rust,ignore
GpuSurface::new(MyRenderer::default()).prefer_hdr_surface();
GpuSurface::new(MyRenderer::default()).prefer_sdr_surface();
```

A renderer can also express the preference itself by overriding `GpuView::preferred_surface_hdr`, which the surface falls back to when no explicit builder call was made. `resolved_hdr_preference()` reports the combined answer, where `None` means "follow the platform". Whatever the outcome, gate your blend state on `ctx.is_hdr()` so one renderer compiles into either pipeline.

## Reference

| Item | Role |
|------|------|
| `GpuView` | Trait you implement for custom GPU rendering |
| `GpuContext` | Setup-time wgpu handles + redraw handle |
| `GpuFrame` | Per-frame texture, pointer, gesture, timing |
| `GpuSurface` | Raw view that owns a single `GpuView` instance |
| `RedrawHandle` | Wakes the surface from outside the render loop |
| `GpuRuntime` | Explicitly constructed GPU context for headless work |
| `OffscreenRenderConfig` | Headless render configuration |
| `OffscreenRenderOutput` | RGBA8 pixel output with PNG encoding |
| `OffscreenRenderOutputHdr` | RGBA16F output with PQ + tone-mapped PNG |

## Next

For the most common GPU use case — a single fragment shader over a full-screen quad — `ShaderSurface` skips most of this boilerplate. Continue to [Shaders](03-shaders.md).

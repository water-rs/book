# GPU content with GpuContentView

> **In this chapter, you will:**
> - Implement `GpuContent` (`setup` and `render`) to draw with `wgpu` inside a view
> - Host it with `GpuContentView` and wire input, a per-frame hook and accessibility
> - Wake the host from other threads with `RedrawHandle`, and choose HDR or SDR output
> - Present frames produced elsewhere (a decoder, a camera) with `ExternalFrameView`
> - Capture pixels headlessly with `GpuRuntime::render_content` and `OffscreenRenderer`

[Canvas](01-canvas.md) and [shaders](03-shaders.md) cover most custom drawing. When you need real `wgpu` access (your own render pipelines, compute passes, instanced draws, a 3D viewport), you implement `GpuContent` and hand it to a `GpuContentView`. The engine owns the device and composites your output like any other layer, so a GPU view sits in the tree next to text and buttons, takes modifiers, and clips and animates like they do.

The `GpuContent` surface needs the `gpu` feature of `waterui`, which is on by default.

## Architecture

```text
your code (impl GpuContent) <-> GpuContentView <-> backend
                                                     |
                                      engine device + queue (wgpu)
                                                     |
                                     texture composited as a layer
```

Your content does not own a swapchain. The engine gives you a texture each frame (`Frame::texture`), you draw into it, and the engine composites it. Two consequences follow from that:

- Content runs on the **render thread**, so `GpuContent` is `Send + 'static`. State you share with the UI (a pointer position, a simulation parameter) crosses through your own synchronized state, such as an `Arc` of atomics.
- A `GpuContentView` owns exactly one content instance for its lifetime. When a parent rebuild tears the view down, the content goes with it and the rebuilt view starts a new one. Do not park content state in a global cache to survive that.

## wgpu and bytemuck come from the facade

A crate implementing `GpuContent` must use exactly the `wgpu` WaterUI links, or the types will not match. Take it from the facade instead of adding a dependency of your own. The same goes for `bytemuck`, which you use to cast uniform and vertex data into buffers:

```rust,ignore
use waterui::graphics::gpu::wgpu;
use waterui::graphics::gpu::bytemuck;
```

## The GpuContent trait

```rust,ignore
pub trait GpuContent: Send + 'static {
    fn setup(&mut self, gpu: &Context<'_>);
    fn render(&mut self, frame: &mut Frame<'_>);

    // Everything below has a default.
    fn is_opaque(&self) -> bool { false }
    fn intrinsic_size(&self) -> Option<Size> { None }
    fn measure(&self, proposal: ProposalSize) -> ViewDimensions { /* intrinsic size or fill */ }
    fn preferred_surface_hdr(&self) -> Option<bool> { None }
}
```

Only `setup` and `render` are required.

- **`setup`** runs on the render thread before the first `render`, and runs again on each new device after the runtime replaces a lost one. Each call replaces every device resource the previous call created, so build pipelines, buffers and textures here and never use handles from an earlier call.
- **`render`** draws one frame into `frame.texture`. Call `frame.request_redraw()` when the content animates and needs the next frame too; a frame happens only when the view is dirty or a redraw was requested.
- **`is_opaque`** lets the engine skip blending underneath the content when every pixel you draw is opaque.
- **`intrinsic_size` and `measure`** control layout, in logical points. With no intrinsic size the view stretches to fill both axes; with one it takes that size and does not stretch. Override `measure` when the size resolves later (a decoder that learns its stream dimensions after setup). It runs on every layout pass and must not touch GPU state. `Size`, `ProposalSize` and `ViewDimensions` are the layout types from `waterui::layout`, which the prelude also brings in.

`GpuContentView::new` reads `is_opaque` and `intrinsic_size` once, when it builds the view. `measure` and `preferred_surface_hdr` are asked again on the live content each time they are needed.

### Context

`setup` receives a `Context` with getters for the device-side handles:

| Getter | Returns |
| --- | --- |
| `instance()`, `adapter()` | the `wgpu` instance and the adapter the device came from |
| `device()`, `queue()` | the device the engine renders with, and its queue |
| `format()` | the `wgpu::TextureFormat` of the texture you render into (`Rgba16Float`) |
| `redraw()` | a `&RedrawHandle` that wakes the host from any thread |

### Frame

`render` receives a `Frame` with public fields:

| Field | Meaning |
| --- | --- |
| `device`, `queue` | the engine's device and queue |
| `texture`, `view` | the texture to draw into and a view of all of it |
| `format` | the texture's format |
| `width`, `height` | the texture size in pixels |
| `scale` | pixels per logical point |
| `elapsed`, `delta` | time since setup and since the previous frame, as `Duration` |
| `presentation_time` | when the platform will present this frame, where it reports one |

`elapsed` and `delta` are the animation clock. Drive motion from them rather than from wall-clock reads of your own.

The texture is always `Rgba16Float` and holds **premultiplied, extended linear Display P3**, the engine's working space, where 1.0 is reference white. Write colors in that space: premultiply by alpha, and do not gamma-encode.

## A first view

This content clears the texture to a color that pulses over time and shifts with the pointer. It needs no pipeline, so the whole `GpuContent` contract fits on one screen:

```rust
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use waterui::graphics::gpu::{Context, Frame};
use waterui::graphics::gpu::wgpu;
use waterui::graphics::{GpuContent, GpuContentView, SurfaceInputEvent};
use waterui::prelude::*;

struct Pulse {
    // The pointer's logical x position, as f32 bits, written by the UI thread.
    pointer_x: Arc<AtomicU32>,
}

impl GpuContent for Pulse {
    fn setup(&mut self, _gpu: &Context<'_>) {}

    fn render(&mut self, frame: &mut Frame<'_>) {
        let x = f32::from_bits(self.pointer_x.load(Ordering::Relaxed));
        let logical_width = frame.width as f32 / frame.scale;
        let hue = (x / logical_width).clamp(0.0, 1.0);
        let wave = (frame.elapsed.as_secs_f32() * 2.0).sin() * 0.5 + 0.5;

        let mut encoder = frame
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("pulse"),
            });
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("pulse"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: frame.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: f64::from(hue * wave),
                            g: f64::from(0.2 + 0.3 * wave),
                            b: f64::from((1.0 - hue) * wave),
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
        }
        frame.queue.submit([encoder.finish()]);
        frame.request_redraw();
    }

    fn is_opaque(&self) -> bool {
        true
    }
}

pub fn pulse_view() -> impl View {
    let pointer_x = Arc::new(AtomicU32::new(0));
    let writer = Arc::clone(&pointer_x);

    GpuContentView::new(Pulse { pointer_x })
        .on_input(move |event| {
            if let SurfaceInputEvent::PointerMove { position } = event {
                writer.store((position.x as f32).to_bits(), Ordering::Relaxed);
            }
        })
        .labeled("Pulse")
        .described("A color field that follows the pointer")
        .size(400.0, 300.0)
}
```

`Frame` here is `waterui::graphics::gpu::Frame`, also exported as `waterui::graphics::Frame`. The `.size(...)` modifier returns an unrelated layout type, `waterui::layout::frame::Frame`, which you never need to name.

### A real pipeline

Most content builds pipelines in `setup`, keeps them in fields, and draws with them in `render`. Keep the WGSL in its own file and pull it in with `include_str!`; a shader does not belong in a string literal. Build the color target against the format the context reports:

```wgsl
// triangle.wgsl
@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    var corners = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 0.7),
        vec2<f32>(-0.7, -0.7),
        vec2<f32>(0.7, -0.7),
    );
    return vec4<f32>(corners[index], 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.95, 0.45, 0.2, 1.0);
}
```

```rust,ignore
use waterui::graphics::gpu::{Context, Frame};
use waterui::graphics::gpu::wgpu;
use waterui::graphics::GpuContent;

#[derive(Default)]
struct Triangle {
    pipeline: Option<wgpu::RenderPipeline>,
}

impl GpuContent for Triangle {
    fn setup(&mut self, gpu: &Context<'_>) {
        let device = gpu.device();
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("triangle"),
            source: wgpu::ShaderSource::Wgsl(include_str!("triangle.wgsl").into()),
        });
        self.pipeline = Some(device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("triangle"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: gpu.format(),
                    blend: Some(wgpu::BlendState::REPLACE),
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

    fn render(&mut self, frame: &mut Frame<'_>) {
        let Some(pipeline) = &self.pipeline else {
            return;
        };
        let mut encoder = frame
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("triangle") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("triangle"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: frame.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(pipeline);
            pass.draw(0..3, 0..1);
        }
        frame.queue.submit([encoder.finish()]);
    }
}
```

The `Option` field exists only because `setup` runs after construction. Uniform data goes into buffers with `frame.queue.write_buffer(&buffer, 0, bytemuck::cast_slice(&values))`, where `values` is a slice of plain-old-data such as `[f32; 4]`.

The engine renders through its own device, so the descriptor fields above follow the `wgpu` version WaterUI links. Read them from `waterui::graphics::gpu::wgpu`, not from another crate's copy.

## GpuContentView

`GpuContentView::new(content)` builds the view. It is a native view: layout asks the content, and the backend installs it on an engine layer.

| Method | Purpose |
| --- | --- |
| `on_input(handler)` | receive pointer, keyboard, IME and scroll events as `SurfaceInputEvent` |
| `on_frame(hook)` | run a UI-thread closure once per frame, before the content renders |
| `on_ime_caret(query)` | answer where the text caret is, as `Option<Rect>` (`waterui::graphics::gpu::Rect`) in logical, view-local points, so the host can place its input-method panel |
| `labeled(text)` | the accessibility name |
| `described(text)` | the accessibility value |
| `prefer_hdr_surface()` / `prefer_sdr_surface()` | override the dynamic-range preference for this view |

Give every GPU view a `labeled` name. A texture carries no semantics of its own: the label and value you set are what the backend publishes to assistive technology, so without `labeled` the view has no accessible name.

Installing `on_input` is also what makes the view take input events at all. `on_input`, `on_frame` and `on_ime_caret` take plain `Fn` closures (no `Send` bound) that run on the UI thread, while the content runs on the render thread. Anything they share goes through your own thread-safe state, as `Pulse` does with its atomic. `on_frame` suits producers that must be pumped from the UI thread; hand their results to the content through a channel.

### Layout

With no `intrinsic_size`, a `GpuContentView` stretches on both axes. Size it with the usual modifiers:

```rust
use waterui::graphics::{GpuContent, GpuContentView};
use waterui::prelude::*;

fn fixed(content: impl GpuContent) -> impl View {
    GpuContentView::new(content).size(400.0, 300.0)
}
```

## Input

`on_input` receives a `SurfaceInputEvent`. Every position is **logical and surface-local**: the view's own top-left is `(0, 0)`, in points, regardless of display scale or where the view sits in the window.

| Variant | When |
| --- | --- |
| `Resize { size, scale }` | before the first frame, and whenever the logical size or scale changes |
| `Focus(bool)` | keyboard focus entered or left; key and text events arrive only while focused |
| `Modifiers(Modifiers)` | the modifier chord changed without a key event |
| `PointerMove { position }` | the pointer moved over the view, or anywhere while it holds the press capture |
| `PointerButton { pressed, button, position, click_count }` | a button went down or up; `click_count` counts a multi-click run |
| `Scroll { position, delta_x, delta_y, unit, phase }` | a wheel or trackpad scroll, with `ScrollUnit` and `ScrollPhase` |
| `Key { pressed, key, code, modifiers, repeat }` | a raw key press or release |
| `TextInput(Str)` | committed text to insert |
| `Paste(Str)` | the platform's paste command, carrying the clipboard text |
| `CompositionStart`, `CompositionUpdate { text, caret }`, `CompositionCommit(text)`, `CompositionCancel` | an input-method session |

Key events follow the W3C UI Events model through the `keyboard_types` vocabulary: `Key` is the logical value, `Code` the physical key, `Modifiers` the chord. All of them, with `NamedKey`, `GesturePhase`, `ScrollUnit`, `ScrollPhase` and `SurfacePointerButton`, live under `waterui::graphics` (the key types also under `waterui::graphics::input`).

Text typed through an input method is not a key event. It arrives as a composition session or as `TextInput`, so a view that edits text handles both.

When taps and drags are all you need, the modifiers in [gestures](../06-advanced/02-gestures.md) cover them. `on_input` is for content that interprets raw input itself, such as an editor, a terminal or a game.

## Waking the host with RedrawHandle

A frame happens when the view is dirty, when `render` asks for another one, or when a `RedrawHandle` fires. The handle is cheap to clone and safe to use from any thread, so a decoder thread, a network task or a compositor callback can wake the view the moment new data exists:

```rust,ignore
use waterui::graphics::RedrawHandle;

fn setup(&mut self, gpu: &Context<'_>) {
    let redraw: RedrawHandle = gpu.redraw().clone();
    // `worker` is your own producer; it runs the closure after each new sample.
    self.worker.on_new_data(move || redraw.request_redraw());
}
```

Requesting a redraw while a frame is already scheduled is a no-op. The handle wakes the same view instance only: after a parent rebuild tears the view down, a request through an old handle redraws nothing.

`RedrawHandle::new(wake)` builds a handle from a closure, which is how a host or a test supplies one.

## HDR and SDR

Content can state which dynamic range it prefers its presentation target to carry, and the view can override it:

- `GpuContent::preferred_surface_hdr` returns `Some(true)` to prefer an HDR target, `Some(false)` for SDR, or `None` (the default) to follow the host's surrounding policy.
- `GpuContentView::prefer_hdr_surface()` and `prefer_sdr_surface()` override the content's answer for that one view.

The preference chooses the presentation target, not the texture you draw into. That texture stays `Rgba16Float` in extended linear Display P3 either way, where 1.0 is reference white; values above 1.0 are highlights brighter than reference white, which an HDR target with headroom can show.

## Frames produced elsewhere

A video decoder, a camera or a compositor-backed web view already holds its pixels in GPU memory. `ExternalFrameView` presents those frames in place, without a copy and without a draw pass of your own: the engine samples the frame's planes where they are, converts YCbCr to its working space from the color description each frame carries, and orders its reads behind the frame's own GPU synchronization.

You implement `ExternalFrameSource`, the producer. The source itself lives on the UI thread with its view (the trait asks only for `'static`); the thread that produces frames keeps the `FrameOutput` the host hands to `start` and publishes through it:

```rust
use std::sync::mpsc::{Receiver, Sender};

use waterui::graphics::gpu::interop::ExternalFrame;
use waterui::graphics::gpu::wgpu::Device;
use waterui::graphics::{ExternalFrameSource, ExternalFrameView, FrameOutput};
use waterui::prelude::*;

struct Stream {
    // Hands each output to the producing thread.
    outputs: Sender<FrameOutput>,
}

impl ExternalFrameSource for Stream {
    // Called on the UI thread once the host has built the layer, and again
    // with a new output on the replacement device after a device loss.
    fn start(&mut self, output: FrameOutput) {
        // A send fails only when the producing thread has already exited,
        // and then there is nothing left to start.
        let _ = self.outputs.send(output);
    }

    fn is_opaque(&self) -> bool {
        true
    }
}

// The producing thread's loop. `next_frame` imports the next frame's planes
// onto the device it is given, and returns `None` when the stream ends.
fn produce(
    outputs: Receiver<FrameOutput>,
    mut next_frame: impl FnMut(&Device) -> Option<ExternalFrame>,
) {
    for output in outputs {
        loop {
            let Some(frame) = next_frame(output.device()) else {
                return;
            };
            if output.present(frame).is_err() {
                // `RetiredOutput`: this host is gone. Wait for the next output.
                break;
            }
        }
    }
}

pub fn stream_view(outputs: Sender<FrameOutput>) -> impl View {
    ExternalFrameView::new(Stream { outputs }).labeled("Live stream")
}
```

Points to know:

- Frames flow through a **single-slot mailbox**. `FrameOutput::present` replaces a frame the host has not drained yet, and dropping that frame releases its planes; a producer that pools its planes learns when the host is done with a frame through `ExternalFrame::on_release`. `FrameOutput` is `Clone`, every clone publishes into the same mailbox, and on native targets it is `Send`.
- Import planes onto `FrameOutput::device()`. An output whose host is gone refuses frames with `RetiredOutput` (`waterui::graphics::gpu::RetiredOutput`), which means stop producing for it.
- A source whose frames have no natural size (a web view) renders at `FrameOutput::presented_size()`, the layer's pixel extent and scale, so the engine never resamples its pixels. It is `None` until the host has run its first pass.
- `ExternalFrameView` carries the same hooks as `GpuContentView`: `on_input`, `on_frame`, `on_ime_caret`, `labeled`, `described`, `prefer_hdr_surface` and `prefer_sdr_surface`. `ExternalFrameSource` has the same `is_opaque`, `intrinsic_size`, `measure` and `preferred_surface_hdr` defaults.
- Frames are stretched to the view's bounds, so a source that must keep an aspect ratio answers `measure` with a size of that ratio.

You will rarely write a source by hand. The media components build on it; see [media](../04-rich/01-media.md).

## Capturing pixels headlessly

Runtime render paths never read GPU textures back to CPU memory: a frame goes from your texture to the compositor and no further. Reading pixels back exists for verification (visual tests, previews, exports), and it is a separate path you invoke yourself.

### GPU content

`GpuRuntime::render_content` renders owned `GpuContent` for one frame into an offscreen target and returns an `OffscreenImage`:

```rust
use waterui::graphics::gpu::GpuRuntime;
use waterui::graphics::{GpuContent, OffscreenSize};

fn capture(
    runtime: &GpuRuntime,
    content: impl GpuContent,
) -> Result<(), Box<dyn std::error::Error>> {
    let size = OffscreenSize::try_from_pixels(800, 600).ok_or("zero-sized capture")?;
    let image = runtime.render_content(content, size, 2.0);
    image.save_png("capture.png")?;
    Ok(())
}
```

`GpuRuntime::new()` is an async constructor that opens a device and fails with a `GpuRuntimeError` when no adapter is available. `render_content` takes the size in pixels and the display scale (pixels per logical point), and panics when engine creation, rendering or readback fails; it is a test and tooling API, not an app path. On the web (`wasm32`) `render_content` is `async` as well. The fence above shows the native signature.

### Scenes and canvases

`OffscreenRenderer` does the same for scene content such as [Canvas](01-canvas.md), and reports failure as `OffscreenError`:

```rust
use waterui::graphics::backend::SceneContent;
use waterui::graphics::{OffscreenImage, OffscreenRenderer, OffscreenSize};

fn capture_scene(
    content: &mut dyn SceneContent,
) -> Result<OffscreenImage, Box<dyn std::error::Error>> {
    let renderer = OffscreenRenderer::new()?;
    let size = OffscreenSize::try_from_pixels(512, 512).ok_or("zero-sized capture")?;
    Ok(renderer.render(content, size, 1.0)?)
}
```

`render` lays the content out at `size / scale` points and renders it at `size` pixels. On the web, `OffscreenRenderer::new` and `render` are `async`.

`OffscreenRenderer::cpu()` builds the CPU raster variant for hosts without a GPU. It needs the `cpu` feature of the `waterui-graphics` crate, which the `waterui` facade does not expose, so enable it on a direct `waterui-graphics` dependency of the same version.

### OffscreenImage

`OffscreenImage` is straight-alpha sRGB8 in row-major order. The engine reads back premultiplied linear Display P3 floats, and `OffscreenImage` unpremultiplies them and encodes sRGB, clipping what falls outside the sRGB gamut. You read it with `width()`, `height()`, `rgba8()` and `pixel(x, y)`, or write it with `save_png`. Judge a render by looking at the saved image, not by counting pixels; see [testing](../07-tools/03-testing.md) for the project's snapshot workflow.

## Platforms

GPU rendering on Linux and Android is Vulkan-only and on Windows DX12-only: no other backend is selected or fallen back to there, and a device without that backend fails with a clear unsupported-device error. See [platforms](../01-getting-started/05-platforms.md) for the tiers.

## What next

- Wrap a simple fragment shader without writing a `GpuContent`: [Shaders](03-shaders.md).
- Apply GPU effects to any view, native widgets included: [Filters](04-filters.md).
- See `ParticleSystem`, built on the same machinery: [Particles](05-particles.md).
- How the engine composites layers: [Rendering](../08-internals/01-rendering.md).

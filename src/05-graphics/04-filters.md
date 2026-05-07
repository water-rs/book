# Filters and visual effects

> **In this chapter, you will:**
> - Apply blur, brightness, contrast, and other filters to any view
> - Chain and compose filters with automatic GPU pass fusion
> - Drive filter parameters with reactive signals
> - Build custom effects with `ViewEffect` and `GpuFilter`
> - Understand how the multi-pass pipeline optimizes your filter chains

Filters turn ordinary views into polished UI: blurred photo galleries, frosted-glass cards, dramatic black-and-white portraits. WaterUI's GPU filter system captures the rendered output of a view, runs it through one or more shader passes, and displays the result with automatic pass fusion and animation support.

## Quick start

`FilterViewExt` adds filter methods to every view. Pull it in with the prelude:

```rust
use waterui::prelude::*;
use waterui_graphics::FilterViewExt;

fn frosted_card() -> impl View {
    text("Hello, World!")
        .blur(10.0)
        .brightness(0.1)
        .contrast(1.2)
}
```

Three filters on a text view -- and thanks to automatic fusion, the brightness and contrast adjustments execute as a single GPU pass.

## Architecture

The filter pipeline has four layers:

1. **`FilterViewExt`** -- the convenience methods (`.blur()`, `.brightness()`, ...) that you call from view code.
2. **`FilterAdapter<F>`** -- bridges the pure-data `Filter` trait from `filtrate-core` to the GPU-aware `GpuFilter` trait, handling pass fusion and signal-driven animation.
3. **`GpuFilter`** -- the low-level trait you implement for custom filters.
4. **Native backend** -- captures the child view to a texture, runs the GPU filter in Rust, and displays the resulting texture.

```
view.blur(10.0)
    -> Filtered<V, FilterAdapter<Blur>>
        -> AppliedFilter metadata
            -> backend captures child to texture
            -> GpuFilter::render(input, output)
            -> backend displays output texture
```

## Built-in filters

All built-in filters accept reactive values through the `IntoSignalF32` trait, so you can pass static `f32`s, `Binding<f64>`, `Computed<f32>`, or any signal that yields a finite float. The adapter converts to `Computed<f32>` and watches it for animations.

### Color filters

Color filters run per pixel and fuse into a single GPU pass when chained consecutively.

| Method | Parameter | Description |
|--------|-----------|-------------|
| `.brightness(amount)` | `f32` | Adjust brightness. 0.0 = unchanged, positive = brighter, negative = darker |
| `.contrast(amount)` | `f32` | Adjust contrast. 1.0 = unchanged, >1.0 = more contrast |
| `.saturation(amount)` | `f32` | Adjust color saturation. 1.0 = unchanged, 0.0 = desaturated |
| `.grayscale(intensity)` | `f32` | Desaturate to grayscale. 0.0 = full color, 1.0 = fully gray |
| `.hue_rotation(angle)` | `f32` | Rotate hue by angle (in radians) |
| `.opacity(amount)` | `f32` | Adjust opacity. 1.0 = fully opaque, 0.0 = transparent |
| `.sepia(intensity)` | `f32` | Apply sepia tone. 0.0 = no effect, 1.0 = full sepia |
| `.invert()` | (none) | Invert all colors |
| `.vignette(radius, softness)` | `f32, f32` | Darken edges with a vignette effect |

### Spatial Filters

Spatial filters sample neighboring pixels and require a separate GPU pass (compute shader).

| Method | Parameter | Description |
|--------|-----------|-------------|
| `.blur(radius)` | `f32` | Gaussian blur with the given pixel radius |
| `.sharpen(amount)` | `f32` | Sharpen edges. Higher values = more sharpening |

## Basic usage

### A single filter

```rust
use waterui::prelude::*;
use waterui::media::Photo;
use waterui_graphics::FilterViewExt;

fn blurred_photo(url: impl Into<waterui::Url>) -> impl View {
    Photo::new(url).blur(8.0)
}
```

### Chaining filters

Use `.then()` to stack low-level `Filter` types from `filtrate_core::filters`:

```rust
fn stylized_photo(url: impl Into<waterui::Url>) -> impl View {
    Photo::new(url)
        .blur(5.0)
        .then(filtrate_core::filters::Brightness(0.15))
        .then(filtrate_core::filters::Contrast(1.3))
        .then(filtrate_core::filters::Saturation(0.8))
}
```

Or use the convenience methods directly -- consecutive color filters are automatically fused:

```rust
fn warm_vintage(url: impl Into<waterui::Url>) -> impl View {
    Photo::new(url)
        .brightness(0.05)
        .then(filtrate_core::filters::Sepia(0.3))
        .then(filtrate_core::filters::Contrast(1.1))
        .then(filtrate_core::filters::Vignette(0.7, 0.5))
}
```

### Filter fusion

The filter system automatically optimizes consecutive color-only filters into a single GPU pass. When you chain `brightness -> contrast -> saturation`, these three fragment shader snippets are fused into one render pass rather than three separate texture reads and writes.

Spatial filters (like `blur` and `sharpen`) always require their own pass. A chain like `blur -> brightness -> contrast -> sharpen` produces three passes:
1. Compute pass: blur
2. Fragment pass: brightness + contrast (fused)
3. Compute pass: sharpen

> **Tip:** Ordering matters for performance. Group your color-only filters together to maximize fusion. Interleaving spatial and color filters creates unnecessary pass boundaries.

## Reactive filters

Every filter convenience method accepts a reactive signal -- in particular, a `Binding<f64>` from a slider works directly. Pass the binding by clone; never call `.get()` to feed a reactive sink.

```rust
use waterui::prelude::*;
use waterui::media::Photo;
use waterui_graphics::FilterViewExt;

fn interactive_blur(url: waterui::Url) -> impl View {
    let blur_radius = Binding::f64(0.0);

    vstack((
        Photo::new(url).blur(blur_radius.clone()),
        Slider::new(&blur_radius).range(0.0..=30.0),
    ))
}
```

### Animated transitions

When the binding feeding a filter is wrapped with an animation, the `FilterAdapter` interpolates between the previous and current values automatically.

```rust
fn animated_filter_demo(url: waterui::Url) -> impl View {
    let blur = Binding::f64(0.0);

    vstack((
        Photo::new(url).blur(blur.clone()),
        button("Toggle blur")
            .state(&blur)
            .action(|State(blur): State<Binding<f64>>| {
                let target = if blur.get() > 0.0 { 0.0 } else { 20.0 };
                with_animation(Animation::spring(180.0, 22.0), || blur.set(target));
            }),
    ))
}
```

The animation system supports three modes:
- **Default**: ease-in-out over 250 ms.
- **Bezier**: cubic bezier curves with configurable duration.
- **Spring**: physics-based spring with stiffness and damping.

## HDR policy

Filter pipelines handle HDR-capable surfaces automatically. Override the default per filter chain:

```rust
fn hdr_aware_filter(url: waterui::Url) -> impl View {
    Photo::new(url)
        .blur(10.0)
        .prefer_hdr()   // use HDR intermediates when available (default)
    //  .require_hdr()  // fail if HDR is unavailable
    //  .force_ldr()    // always use LDR intermediates
}
```

| Policy | Behavior |
|--------|----------|
| `PreferHdr` | Use HDR intermediates with automatic LDR fallback (default) |
| `RequireHdr` | Require HDR-capable pipeline; fail setup if unavailable |
| `ForceLdr` | Force LDR intermediates for compatibility or performance |

When the input or output surface is HDR (`Rgba16Float`), the scratch textures between passes use `Rgba16Float` to preserve dynamic range. Otherwise, `Rgba8Unorm` is used.

## ViewEffect: a lower-level effect API

For effects that go beyond simple filters -- distortion, particle overlays, custom post-processing -- use `ViewEffect` with the `EffectRenderer` trait:

```rust
use core::future::Future;
use waterui_graphics::{ViewEffect, EffectRenderer, EffectContext, EffectInput, EffectOutput, wgpu};

struct WaveDistortion {
    pipeline: Option<wgpu::RenderPipeline>,
    bind_group_layout: Option<wgpu::BindGroupLayout>,
    sampler: Option<wgpu::Sampler>,
}

impl EffectRenderer for WaveDistortion {
    fn setup(&mut self, ctx: &EffectContext) -> impl Future<Output = ()> {
        // create pipeline, bind group layout, sampler.
        // ctx.input_format and ctx.output_format may differ.
        async {}
    }

    fn render(&mut self, input: &EffectInput, output: &EffectOutput) {
        // read from input.texture/input.view, write to output.texture/output.view.
        // input and output may have different dimensions.
    }
}
```

### Using ViewEffect

```rust
fn distorted_content() -> impl View {
    ViewEffect::new(
        text("Wavy text"),
        WaveDistortion::default(),
    )
}
```

### Output size control

By default, the output texture matches the captured view's size. Override it via `OutputSize`:

```rust
// double resolution for higher quality
ViewEffect::new(my_view(), effect)
    .output_size(OutputSize::Scale(2.0))

// fixed-size output
ViewEffect::new(my_view(), effect)
    .output_size(OutputSize::Fixed { width: 1920, height: 1080 })
```

`OutputSize` does not affect layout -- it changes only the GPU processing resolution.

## Custom GpuFilter

For maximum control, implement `GpuFilter` directly. Notice that the trait's `setup` returns `FilterSetupResult` (i.e. `Result<(), &'static str>`) and `render` returns `FilterRenderResult` (`Result<bool, &'static str>`). The `bool` answers "is animation still running?".

```rust
use core::future::Future;
use waterui_graphics::{
    FilterContext, FilterInput, FilterOutput, FilterRenderResult, FilterSetupResult, GpuFilter, wgpu,
};

struct CustomFilter {
    pipeline: Option<wgpu::RenderPipeline>,
    bind_group_layout: Option<wgpu::BindGroupLayout>,
    sampler: Option<wgpu::Sampler>,
    animating: bool,
}

impl GpuFilter for CustomFilter {
    fn setup(&mut self, ctx: &FilterContext) -> impl Future<Output = FilterSetupResult> {
        // build pipelines from ctx.device, ctx.queue, ctx.input_format, ctx.output_format,
        // and (when present) ctx.pipeline_cache.
        async { Ok(()) }
    }

    fn render(&mut self, input: &FilterInput, output: &FilterOutput) -> FilterRenderResult {
        // render input.view -> output.view. Return Ok(true) while interpolating.
        Ok(self.animating)
    }
}
```

Apply it using `FilterViewExt::filter`:

```rust
fn custom_filtered() -> impl View {
    text("Hello").filter(CustomFilter::default())
}
```

`FilteredView::new` is the lower-level wrapper -- it takes an `AnyView` and the filter directly:

```rust
use waterui_core::AnyView;
use waterui_graphics::FilteredView;

fn custom_filtered() -> impl View {
    FilteredView::new(AnyView::new(text("Hello")), CustomFilter::default())
}
```

## The filtrate-core Filter trait

WaterUI's high-level filters sit on top of the `filtrate-core` crate, which provides a pure-data `Filter` trait with parameter arrays. `FilterAdapter` bridges that into `GpuFilter`:

```
filtrate_core::Filter
    -> FilterAdapter<F>  (implements GpuFilter)
        -> AppliedFilter (type-erased metadata on the view)
            -> backend renders
```

`Filter` provides:
- `COLOR_ONLY` -- whether the filter only modifies pixel colors (no spatial sampling).
- `Params` -- a `ParamArray` type for reactive parameters.
- `params()` -- returns the current parameter values.

`FilterGraph` (internal) enables stage collection for pass planning, animation watcher installation for reactive parameters, and pass fusion.

## Composing filters in practice

### Photo editor effect stack

```rust
use filtrate_core::filters::{Brightness, Contrast, Saturation};

fn photo_adjustments(
    url: waterui::Url,
    blur: Binding<f64>,
    brightness: Binding<f64>,
    contrast: Binding<f64>,
    saturation: Binding<f64>,
) -> impl View {
    Photo::new(url)
        .blur(blur)
        .then(Brightness(brightness.computed()))
        .then(Contrast(contrast.computed()))
        .then(Saturation(saturation.computed()))
}
```

### Frosted glass

```rust
use filtrate_core::filters::{Brightness, Saturation};

fn frosted_glass(content: impl View) -> impl View {
    content
        .blur(20.0)
        .then(Brightness(0.05))
        .then(Saturation(1.2))
}
```

### Dramatic black-and-white

```rust
use filtrate_core::filters::{Contrast, Vignette};

fn dramatic_bw(url: waterui::Url) -> impl View {
    Photo::new(url)
        .grayscale(1.0)
        .then(Contrast(1.6))
        .then(Vignette(0.6, 0.4))
}
```

## Performance notes

- **Pass fusion**: consecutive color-only filters fuse into a single fragment shader pass. Five colour filters cost roughly the same as one.
- **Spatial filters**: blur and sharpen use compute shaders with intermediate textures. Each spatial filter is a separate pass.
- **Texture captures**: the backend captures the child view to a texture before filtering. For `GpuSurface` children, the backend can sample the existing texture without an extra capture step.
- **Animation**: when filter parameters change with animation context, `FilterAdapter` keeps the surface dirty until the animation settles. Spring animations finish naturally; bezier animations run for a fixed duration.
- **Scratch textures**: multi-pass filters ping-pong between two scratch textures. They are allocated lazily and resized only when needed.

## Next

Filters transform existing content. To create new visual content from scratch on the GPU, continue to [Particle systems](05-particles.md).

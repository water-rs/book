# Filters and visual effects

> **In this chapter, you will:**
> - Apply blur, brightness, contrast, and dozens of other filters to any view
> - Chain filters so consecutive color operations fuse into a single GPU pass
> - Drive filter parameters with reactive signals and animate them
> - Build custom effects with the `Effect` and `EffectRenderer` traits
> - Choose an HDR policy for a filter chain

WaterUI's filter system captures the rendered output of a view, runs it through one or more GPU passes, and displays the result. Blurred photo galleries, frosted-glass cards, dramatic black-and-white portraits: all of it is one method call on an existing view.

## Quick start

`FilterViewExt` is in the prelude whenever the default `gpu` feature is on, so there is nothing extra to import:

```rust,ignore
use waterui::prelude::*;

fn frosted_card() -> impl View {
    text("Hello, World!")
        .blur(10.0)
        .brightness(0.1)
        .contrast(1.2)
}
```

Three filters on a text view, executed as two GPU passes: the blur, then brightness and contrast fused together.

![Filtered gradient card with blur brightness and contrast](../assets/visuals/05-graphics/filter-frosted-gradient.png)

*FilterViewExt applied to a WaterUI view snapshot. [Example source](https://github.com/water-rs/book/tree/main/examples/book-visuals).*

## How a filter reaches the GPU

```text
view.blur(10.0)
    -> Filtered<V, FilterAdapter<Blur>>
        -> AppliedFilter metadata on the view
            -> backend captures the child view to a texture
            -> Effect::encode_render(input, output)
            -> backend displays the output texture
```

`FilterViewExt` methods return `Filtered<V, FilterAdapter<F>>`. `FilterAdapter` is the bridge: it takes a pure-data `Filter` (a filter kind plus its parameters, from the `filtrate` crate), watches any reactive parameters, plans the passes, and presents the whole thing to the backend as one `Effect`. `Effect` is the low-level GPU trait, and it is also what you implement for a custom effect.

## Built-in filters

Every parameter is `impl IntoSignalF32`, so a literal `10.0`, a `Binding<f64>`, a `Computed<f32>`, or any signal yielding a number all work in the same slot.

Color filters run per pixel and fuse with their neighbors:

| Method | Parameters | Description |
|--------|------------|-------------|
| `.brightness(amount)` | 1 | 0.0 leaves the image unchanged; positive brightens |
| `.exposure(ev)` | 1 | Exposure in photographic stops |
| `.contrast(amount)` | 1 | 1.0 unchanged, above 1.0 increases contrast |
| `.gamma(gamma)` | 1 | Gamma adjustment |
| `.saturation(amount)` | 1 | 1.0 unchanged, 0.0 fully desaturated |
| `.vibrance(amount)` | 1 | Saturates muted colors more than saturated ones |
| `.grayscale(intensity)` | 1 | 0.0 full color, 1.0 fully gray |
| `.sepia(intensity)` | 1 | 0.0 no effect, 1.0 full sepia |
| `.hue_rotation(angle)` | 1 | Rotate hue, in radians |
| `.invert()` | 0 | Invert all channels |
| `.temperature_tint(temp, tint)` | 2 | White-balance adjustment |
| `.highlights_shadows(hi, lo)` | 2 | Recover highlights, lift shadows |
| `.color_matrix(m)` | `[[f32; 4]; 3]` | Arbitrary 3×4 color transform |
| `.white_point(r, g, b)` | 3 | Color balance from an explicit white point |
| `.vignette(radius, softness)` | 2 | Darken toward the edges |

Spatial filters sample neighboring pixels and therefore need a pass of their own:

| Method | Parameters | Description |
|--------|------------|-------------|
| `.blur(radius)` | 1 | Blur with the given pixel radius |
| `.gaussian_blur(sigma)` | 1 | Gaussian blur by standard deviation |
| `.motion_blur(radius, angle)` | 2 | Directional blur |
| `.zoom_blur(amount, x, y)` | 3 | Radial blur around a focal point |
| `.sharpen(amount)` | 1 | Edge sharpening |
| `.unsharp_mask(radius, amount)` | 2 | Classic unsharp mask |
| `.bloom(radius, intensity, threshold)` | 3 | Glow around bright regions |
| `.sobel()` / `.prewitt()` | 0 | 3×3 edge detection |
| `.median3x3()` | 0 | Salt-and-pepper denoise |
| `.convolution3x3(kernel)` / `.convolution5x5(kernel)` | kernel array | Arbitrary convolution |
| `.pixellate(size)` / `.crystallize(size)` | 1 | Mosaic effects |
| `.kaleidoscope(...)` / `.mirror_tile(...)` | 4 / 2 | Tiling and reflection |
| `.bump_distortion(...)`, `.pinch_distortion(...)`, `.twirl_distortion(...)`, `.vortex_distortion(...)` | 4 each | Geometric warps around a center |

There are also eight presets that need no parameters at all — `.photo_effect_mono()`, `.photo_effect_noir()`, `.photo_effect_chrome()`, `.photo_effect_instant()`, `.photo_effect_fade()`, `.photo_effect_process()`, `.photo_effect_tonal()`, `.photo_effect_transfer()` — plus multi-input filters that take a second image (`.blend_with_image`, `.masked_blur`, `.lut_color_grade`, the `*_transition_to_image` family, and more). The complete list lives on the `FilterViewExt` trait.

## Chaining and fusion

Keep calling filter methods on the result. Consecutive calls extend one chain rather than nesting a second capture:

```rust,ignore
use waterui::prelude::*;
use waterui::media::Photo;

fn warm_vintage(url: waterui::Url) -> impl View {
    Photo::new(url)
        .brightness(0.05)
        .sepia(0.3)
        .contrast(1.1)
        .vignette(0.7, 0.5)
}
```

Every color-only filter in a run is compiled into a single fragment shader pass, so those four calls cost roughly what one costs. Spatial filters break the run: `blur -> brightness -> contrast -> sharpen` produces three passes — blur, the fused brightness+contrast pair, then sharpen.

Ordering therefore matters for performance. Group your color adjustments together instead of interleaving them with blurs.

## Reactive filters

Pass the binding, not its value. A slider bound to `Binding<f64>` drives a filter directly:

```rust,ignore
use waterui::prelude::*;
use waterui::media::Photo;

fn interactive_blur(url: waterui::Url) -> impl View {
    let blur_radius = Binding::f64(0.0);

    vstack((
        Photo::new(url).blur(blur_radius.clone()),
        slider("Blur radius", &blur_radius).range(0.0..=30.0),
    ))
}
```

`FilterAdapter` watches the signal and repaints the filtered texture. The view itself is never rebuilt, so nothing below it loses state.

### Animating a parameter

Wrap the signal with an animation and the adapter interpolates between the old and new values, keeping the surface dirty until it settles:

```rust,ignore
use core::time::Duration;
use waterui::prelude::*;
use waterui::animation::{Animation, AnimationExt};
use waterui::media::Photo;

fn animated_blur(url: waterui::Url) -> impl View {
    let blur = Binding::f64(0.0);

    vstack((
        Photo::new(url).blur(blur.clone().with_animation(Animation::spring(180.0, 22.0))),
        button("Toggle blur")
            .action(|State(blur): State<Binding<f64>>| {
                let target = if blur.get() > 0.0 { 0.0 } else { 20.0 };
                blur.set(target);
            })
            .state(&blur),
    ))
}
```

`Animation::spring(stiffness, damping)` runs until the spring settles; `Animation::ease_in_out(Duration::from_millis(250))` and the other bezier curves run for a fixed duration. `.animated()` is shorthand for the default curve.

## HDR policy

Filter chains adapt to HDR-capable surfaces on their own. Override the choice per chain:

```rust,ignore
fn hdr_aware_filter(url: waterui::Url) -> impl View {
    Photo::new(url)
        .blur(10.0)
        .prefer_hdr()   // HDR intermediates when available (default)
    //  .require_hdr()  // fail setup if HDR is unavailable
    //  .force_ldr()    // always use LDR intermediates
}
```

`.hdr_policy(HdrPolicy::PreferHdr | RequireHdr | ForceLdr)` is the same thing spelled out. When the surface is HDR (`Rgba16Float`), the scratch textures between passes are `Rgba16Float` too; otherwise they are 8-bit.

## ViewEffect: post-processing a view

For work that is not a filter — distortion, overlays, custom post-processing — `ViewEffect` gives you the captured texture and an output texture, and stays out of the way:

```rust,ignore
use core::future::Future;
use waterui::graphics::{
    EffectRenderer, ViewEffect, ViewEffectContext, ViewEffectInput, ViewEffectOutput,
};

#[derive(Default)]
struct WaveDistortion {
    pipeline: Option<wgpu::RenderPipeline>,
    sampler: Option<wgpu::Sampler>,
}

impl EffectRenderer for WaveDistortion {
    fn setup(&mut self, ctx: &ViewEffectContext) -> impl Future<Output = ()> {
        // ctx.device, ctx.queue, ctx.input_format, ctx.output_format
        // input and output formats may differ.
        async {}
    }

    fn render(&mut self, input: &ViewEffectInput, output: &ViewEffectOutput) {
        // sample input.view, draw into output.view.
        // input and output may have different dimensions.
    }
}

fn distorted_content() -> impl View {
    ViewEffect::new(text("Wavy text"), WaveDistortion::default())
}
```

By default the output texture matches the captured view. `OutputSize` changes the GPU processing resolution without touching layout:

```rust,ignore
use waterui::graphics::OutputSize;

ViewEffect::new(my_view(), effect).output_size(OutputSize::Scale(2.0));
ViewEffect::new(my_view(), effect).output_size(OutputSize::Fixed { width: 1920, height: 1080 });
```

## Custom Effect

`Effect` is what `.filter(...)` accepts, and it is the trait the built-in filters compile down to. Two things about its shape are worth knowing before you write one:

- **`encode_render` is the required method, not `render`.** You write your commands into a caller-supplied encoder so a host rendering several effects in one frame can submit once. `render` has a default implementation that creates an encoder, calls `encode_render`, and submits.
- **The result types carry meaning.** `EffectSetupResult` is `Result<(), &'static str>`; `EffectRenderResult` is `Result<bool, &'static str>`, where the `bool` answers "does this need another frame?"

```rust,ignore
use core::future::Future;
use waterui::prelude::*;
use waterui::graphics::filter_view::{
    Effect, EffectContext, EffectInput, EffectOutput, EffectRenderResult, EffectSetupResult,
};

#[derive(Default)]
struct CustomEffect {
    pipeline: Option<wgpu::RenderPipeline>,
    animating: bool,
}

impl Effect for CustomEffect {
    fn setup(&mut self, ctx: &EffectContext) -> impl Future<Output = EffectSetupResult> {
        // build pipelines from ctx.device, ctx.input_format, ctx.output_format
        async { Ok(()) }
    }

    fn encode_render(
        &mut self,
        input: &EffectInput,
        output: &EffectOutput,
        encoder: &mut wgpu::CommandEncoder,
    ) -> EffectRenderResult {
        // encode a pass reading input.view and writing output.view
        Ok(self.animating)
    }
}

fn custom_filtered() -> impl View {
    text("Hello").filter(CustomEffect::default())
}
```

Two optional hooks round it out: `output_size(input_w, input_h)` when the effect wants a different output resolution, and `redraw_hint()` so a backend doing on-demand rendering knows the effect has pending state.

## Performance notes

- Consecutive color-only filters fuse into one fragment pass. Five of them cost about what one costs.
- Each spatial filter is its own pass with its own intermediate texture.
- The backend captures the child view to a texture before filtering. When the child is already a `GpuSurface`, it samples that texture directly and skips the capture.
- Multi-pass chains ping-pong between two scratch textures, allocated lazily and resized only when the surface changes size.
- An animating parameter keeps the surface dirty until it settles. Springs settle on their own; bezier curves run for their declared duration.

## Next

Filters transform existing content. To generate new visual content on the GPU, continue to [Particle systems](05-particles.md).

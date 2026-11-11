# Filters and visual effects

> **In this chapter, you will:**
> - Apply blur, brightness, contrast, and dozens of other filters to any view
> - Chain filters so consecutive color operations compose into one stage
> - Drive filter parameters with reactive signals and animate them
> - Write a custom post-process shader with `ShaderEffect` and `.effect(...)`
> - Know where the `Filter`, `Effect` and `FilterDescription` layers meet, and what the color contract promises

WaterUI's filter system takes the rendered output of a view, runs it through the filter's stages, and displays the result. Blurred galleries, frosted-glass cards, dramatic black-and-white portraits: all of it is one method call on an existing view.

Filters live behind the facade's `effects` feature. The default `gpu` feature implies it, so a default build already has everything in this chapter. `FilterViewExt` is in the prelude whenever `effects` is on.

## Quick start

```rust
use waterui::prelude::*;

fn frosted_card() -> impl View {
    text("Hello, World!")
        .blur(10.0)
        .brightness(0.1)
        .contrast(1.2)
}
```

Three filters on a text view. The blur samples neighboring pixels; brightness and contrast are per-pixel color operations that compose into one segment after it.

![Filtered gradient card with blur brightness and contrast](../assets/visuals/05-graphics/filter-frosted-gradient.png)

*FilterViewExt applied to a WaterUI view snapshot. [Example source](https://github.com/water-rs/book/tree/main/examples/book-visuals).*

## How a filter reaches the backend

There are two entry points on `FilterViewExt`, and everything else is a shortcut for the first:

```text
view.filter(F)   F: Filter + RenderTransfer   -> Filtered<V, F>
view.effect(E)   E: filtrate::Effect + RenderTransfer   -> WithEffect<V, E>   (needs `gpu`)
```

- A **`Filter`** is pure data from the `filtrate` crate: a typed chain of stages (each a WGSL function) plus its parameters. `view.blur(10.0)` is `view.filter(...)` with a `Blur` whose parameter is bound to a signal, returning `Filtered<V, Blur>`. `Filtered::then` appends any further filter to the same chain, and so do the single-parameter shortcuts (see [Chaining and composition](#chaining-and-composition)).
- An **`Effect`** is the GPU-side runtime trait: it owns pipelines between `setup` and teardown, reads an input texture and writes an output texture. `view.effect(E)` accepts any `Effect`, such as `filtrate::Executor` or `ShaderEffect`, and returns `WithEffect<V, E>`.

Both erase into the same `FilteredView` leaf, which carries an `AnyEffect` and the subscriptions that feed its reactive parameters. A backend then realizes it in one of two ways:

- **GPU effect**: the backend moves the effect to its render thread and runs it against the engine's device. Every `Filter` can run this way, and an arbitrary `Effect` can only run this way.
- **Native lowering**: `AnyEffect::description()` returns a `FilterDescription` for a filter, which lists its stages, flattened parameters and reactive signals, so a backend can map them onto its own primitives. For an arbitrary effect it returns `None`, because a custom GPU effect has no portable description.

The filter types are re-exported by the facade, so you never name `filtrate` as a dependency: the filter vocabulary is `waterui::graphics::filter_view`, the built-in filters `waterui::graphics::filter_view::filters`.

## Built-in filters

Every scalar parameter is `impl IntoSignalF32`, so a literal `10.0`, a `Binding<f64>`, a `Computed<f32>`, or any signal yielding a number all work in the same slot. Color matrices and perspective quads are plain `f32` arrays, convolution kernels are arrays of a `Copy` scalar, and images are plain values. The `blur` radius and the `gaussian_blur` sigma are lengths in points, which the pipeline scales to device pixels; the other filters document their own units (a `pixellate` cell, for instance, is in pixels).

The methods below come from `FilterViewExt`. Color filters map each pixel's color on its own:

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
| `.hue_rotation(angle)` | 1 | Rotate hue, in degrees |
| `.invert()` | 0 | Invert all channels |
| `.temperature_tint(temp, tint)` | 2 | White-balance adjustment |
| `.highlights_shadows(hi, lo)` | 2 | Recover highlights, lift shadows |
| `.color_matrix(m)` | `[[f32; 4]; 3]` | Arbitrary 3x4 color transform |
| `.white_point(r, g, b)` | 3 | Color balance from an explicit white point |
| `.tone_curve(shadows, midtones, highlights, gamma, amount)` | 5 | Master tone curve |

Spatial filters read other pixels or depend on the pixel's position:

| Method | Parameters | Description |
|--------|------------|-------------|
| `.vignette(radius, softness)` | 2 | Darken toward the edges |
| `.lut_color_grade(lut, intensity)` | `LutImage` + 1 | 3D lookup-table grading from a 2D strip |
| `.blur(radius)` | 1 | Blur with the given radius |
| `.gaussian_blur(sigma)` | 1 | Gaussian blur by standard deviation |
| `.motion_blur(radius, angle)` | 2 | Directional blur |
| `.zoom_blur(amount, x, y)` | 3 | Radial blur around a focal point |
| `.sharpen(amount)` | 1 | Edge sharpening |
| `.unsharp_mask(radius, amount)` | 2 | Classic unsharp mask |
| `.bloom(radius, intensity, threshold)` / `.gloom(radius, intensity, threshold)` | 3 each | Add a glow around bright regions / dull the highlights |
| `.sobel()` / `.prewitt()` | 0 | 3x3 edge detection |
| `.median3x3()` | 0 | Salt-and-pepper denoise |
| `.morphology_min()` / `.morphology_max()` / `.morphology_gradient()` | 0 | Erode, dilate, outline |
| `.convolution3x3(kernel)` / `.convolution5x5(kernel)` | kernel array | Arbitrary convolution |
| `.pixellate(size)` / `.crystallize(size)` | 1 | Mosaic effects |
| `.edge_work(...)`, `.dot_halftone(...)`, `.line_halftone(...)` | 2 / 4 / 4 | Stylized rendering |
| `.kaleidoscope(...)` / `.mirror_tile(...)` | 4 / 2 | Tiling and reflection |
| `.bump_distortion(...)`, `.pinch_distortion(...)`, `.twirl_distortion(...)`, `.vortex_distortion(...)` | 4 each | Geometric warps around a center |
| `.perspective_transform(quad)` / `.perspective_correction(quad)` | `[[f32; 2]; 4]` | Four-corner warps |
| `.guided_smooth(...)`, `.depth_aware_blur(...)`, `.temporal_denoise(...)`, `.replace_background(...)` | images + scalars | Smoothing and matting with auxiliary images |

There are also eight color presets that need no parameters: `.photo_effect_mono()`, `.photo_effect_noir()`, `.photo_effect_chrome()`, `.photo_effect_instant()`, `.photo_effect_fade()`, `.photo_effect_process()`, `.photo_effect_tonal()`, `.photo_effect_transfer()`. Multi-image filters take a second image as a `FilterImage`: `.blend_with_image(...)`, `.masked_blur(...)`, `.displacement_warp(...)`, `.transition_to_image(...)`, `.swipe_transition_to_image(...)`, `.radial_transition_to_image(...)`, `.zoom_transition_to_image(...)` and `.displacement_transition_to_image(...)`. `FilterImage` and `LutImage` come from `waterui::graphics::filter_view`. The authoritative list is the `FilterViewExt` trait; the filter each method builds is a type in `waterui::graphics::filter_view::filters` (`Blur`, `Brightness`, `Bloom`, and so on), generic over how its parameters are held. The same names at the root of `waterui::graphics` are aliases fixed to the reactive form, which the shortcuts use; they are not what you construct from a constant.

A filter the shortcuts do not cover goes through `.filter(...)` directly:

```rust
use waterui::graphics::filter_view::filters::{Blur, Brightness};
use waterui::prelude::*;

fn soft_and_light() -> impl View {
    text("Soft").filter(Blur(6.0_f32)).then(Brightness(0.1_f32))
}
```

Constants are plain values here. Only the named shortcuts (`.blur(...)` and the rest) bind reactive signals for you.

## Chaining and composition

Keep calling filter methods on the result. Consecutive calls extend one chain rather than nesting a second capture:

```rust
use waterui::prelude::*;

fn warm_vintage() -> impl View {
    vstack((Red.size(60.0, 40.0), Yellow.size(60.0, 40.0), Purple.size(60.0, 40.0)))
        .brightness(0.05)
        .sepia(0.3)
        .contrast(1.1)
}
```

`Filtered` has inherent versions of the single-parameter shortcuts (`blur`, `gaussian_blur`, `brightness`, `contrast`, `exposure`, `gamma`, `saturation`, `vibrance`, `grayscale`, `sepia`, `hue_rotation`, `sharpen`, `pixellate`, `crystallize`) and of `invert()`, and these extend the existing chain. A shortcut with two or more parameters, such as `.vignette(radius, softness)`, comes from `FilterViewExt` and wraps the already-filtered view in a second `Filtered`, which captures it again. Put multi-parameter filters where one capture more is acceptable, or append the `filtrate` filter with `Filtered::then`.

The type of a chain records its kind: color followed by color is a color chain, and anything involving a spatial filter makes the chain spatial. The shader composer joins each run of consecutive color stages into one segment, and where a color run precedes a spatial stage it also offers a folded alternative that applies the colors inside the spatial stage's samples. The reference `Executor` never folds: every piece is its own full-screen pass and every spatial stage is materialized. The Cherenkov engine chooses among the alternatives. How many passes a chain costs is therefore a property of the engine, not of your code, but the guidance holds everywhere: group color adjustments together instead of interleaving them with blurs.

## Reactive filters

Pass the binding, not its value. A slider bound to `Binding<f64>` drives a filter directly:

```rust
use waterui::prelude::*;

fn interactive_blur() -> impl View {
    let blur_radius = Binding::f64(0.0);

    vstack((
        hstack((Red.size(50.0, 50.0), Green.size(50.0, 50.0), Blue.size(50.0, 50.0)))
            .blur(blur_radius.clone()),
        slider("Blur radius", &blur_radius).range(0.0..=30.0),
    ))
}
```

The filter watches the signal and re-renders the filtered texture. The view itself is never rebuilt, so nothing below it loses state.

### Animating a parameter

Attach an animation to the signal with `SignalExt::with`, and the filter interpolates between the old and new values, keeping the surface dirty until it settles:

```rust
use core::time::Duration;
use waterui::animation::Animation;
use waterui::prelude::*;

fn animated_blur() -> impl View {
    let blur = Binding::f64(0.0);

    vstack((
        hstack((Red.size(50.0, 50.0), Green.size(50.0, 50.0), Blue.size(50.0, 50.0)))
            .blur(blur.with(Animation::spring(180.0, 22.0))),
        hstack((
            button("Sharp")
                .action(|State(blur): State<Binding<f64>>| blur.set(0.0))
                .state(&blur),
            button("Blurred")
                .action(|State(blur): State<Binding<f64>>| blur.set(20.0))
                .state(&blur),
        )),
        text("Eased")
            .blur(blur.with(Animation::ease_in_out(Duration::from_millis(250)))),
    ))
}
```

`Animation::spring(stiffness, damping)` runs until the spring settles; `Animation::ease_in_out(duration)` and the other bezier curves run for a fixed duration. `.animated()` from `AnimationExt` attaches the platform's default curve.

## Output size

An effect renders into an output texture that, by default, matches the captured view. `WithEffect::output_size`, and `FilteredView::output_size` on the erased form a backend receives, change the GPU processing resolution without touching layout. They take any `IntoComputed<OutputSize>`, with `OutputSize::MatchInput`, `OutputSize::Scale(f32)` or `OutputSize::Fixed { width, height }` (in pixels); `OutputSize` is at `waterui::graphics::OutputSize`. A `Filtered` chain has no `output_size` method.

## Custom shaders with ShaderEffect

For work that is not one of the built-in filters (a CRT scanline pass, an animated distortion, a terminal's custom shader), write one WGSL fragment shader and let `ShaderEffect` do the rest. It is re-exported at `waterui::graphics::filter_view::ShaderEffect` with the `gpu` feature. The shader defines `@fragment fn main`, and the effect appends a prelude that declares the input texture, a bilinear sampler, a uniform block, and `effect_param(index)`:

```wgsl
struct ShaderEffectUniforms {
    resolution: vec2<f32>,       // output size in physical pixels
    input_resolution: vec2<f32>, // input size in physical pixels
    time: f32,                   // seconds on the host frame timeline
    time_delta: f32,             // seconds since the previous frame
    frame: u32,                  // host frame sequence
    param_count: u32,
    params: array<vec4<f32>, 4>,
}
@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var input_sampler: sampler;
@group(0) @binding(2) var<uniform> uniforms: ShaderEffectUniforms;
fn effect_param(index: u32) -> f32;
// VertexOutput { position: vec4<f32>, uv: vec2<f32> }, uv (0,0) is top-left
```

Those names and the vertex entry point `shader_effect_vs` are reserved. A module carries at most `SHADER_EFFECT_MAX_PARAMS` (16) parameters.

```rust
use waterui::graphics::filter_view::{ShaderEffect, ShaderEffectError};
use waterui::prelude::*;

const SCANLINES: &str = r"
@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(input_texture, input_sampler, in.uv);
    let row = u32(in.position.y);
    let line = select(1.0, 0.0, (row / 2u) % 2u == 1u);
    let flicker = 0.97 + 0.03 * sin(uniforms.time * 50.0);
    let shade = mix(1.0, line, effect_param(0u)) * flicker;
    return vec4<f32>(color.rgb * shade, color.a);
}
";

/// WGSL is validated on the CPU when the effect is built, so a malformed
/// shader fails here with line and column numbers in your own source.
fn scanline_effect() -> Result<ShaderEffect, ShaderEffectError> {
    Ok(ShaderEffect::new(SCANLINES)?.animated())
}

fn crt(strength: Binding<f64>, effect: ShaderEffect) -> impl View {
    text("日本語 한국어 中文 Hello")
        .effect(effect)
        .param(strength)
}
```

`ShaderEffect::new` returns `ShaderEffectError` (`Parse`, `Validation` or `MissingEntryPoint`) at construction, so a bad shader never reaches a backend. Handle it where you load the source and report it through `tracing`.

On the view side, `WithEffect<V, ShaderEffect>` adds two builders:

- `.param(signal)` appends the next parameter, read in the shader as `effect_param(n)`. Constants set on the effect with `ShaderEffect::param(f32)` come first. Each change follows the animation the signal carries.
- `.animated(flag)` drives the animation flag reactively. The effect asks for another frame after each one while the flag is true, which a shader that reads `uniforms.time` needs. `ShaderEffect::animated()` sets it constant.

A shader that does not read time is re-rendered only when its input or a parameter changes. What depends on the device is checked at `setup`: a shader that reads `input_sampler` needs an input format the device can filter, and setup fails with `EffectSetupError::InputNotFilterable` otherwise.

### Implementing Effect yourself

`ShaderEffect` covers single-pass fragment work. For anything else, such as several pipelines, compute work or your own textures, implement `filtrate::Effect` (at `waterui::graphics::filter_view::Effect`) and pass it to `.effect(...)`. Three things about its shape matter:

- **`encode_render` is the required render method**, not `render`. You record commands into a caller-supplied encoder so a host drawing several effects in one frame submits once. `render` has a default that creates an encoder, calls `encode_render`, and submits.
- **The result types carry meaning.** `EffectSetupResult` is `Result<(), EffectSetupError>`; `EffectRenderResult` is `Result<bool, EffectRenderError>`, where the `bool` answers "does this need another frame?"
- **Retained memory is reported.** `retained_memory` and `trim` have no default, so an effect that keeps no targets says so explicitly.

```rust,ignore
use core::future::Future;
use waterui::graphics::filter_view::{
    Effect, EffectContext, EffectInput, EffectOutput, EffectRenderResult, EffectSetupResult,
    RetainedMemory,
};
use waterui::graphics::gpu::wgpu;

struct CustomEffect {
    pipeline: Option<wgpu::RenderPipeline>,
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
        // encode a pass reading input.texture() and writing output.texture()
        Ok(false)
    }

    fn retained_memory(&self) -> RetainedMemory {
        RetainedMemory::default()
    }

    fn trim(&mut self) {}
}
```

The optional hooks are `output_size(input_w, input_h)` for a different output resolution, `set_redraw_callback` for state driven from outside the frame loop, and `redraw_hint()` so a backend rendering on demand knows the effect has pending state. A custom effect has no `FilterDescription`, so only a backend that runs effects on a GPU can realize it.

## Color and alpha contract

An effect's input, intermediates and output follow one contract (documented in `filtrate::effect`):

- **Premultiplied alpha, end to end.** Every stage receives and returns premultiplied color.
- **Linear working space.** Texel values are premultiplied colors in the linear working space (linear Display P3), exactly as sampled; no transfer function is applied implicitly. Stages that operate in sRGB are wrapped in conversions by the executor.
- **f16 intermediates.** Executor intermediates are `Rgba16Float`, so values round to f16 at each materialization point and extended-range values survive between passes.

There is no per-chain HDR policy API to set; the extended range follows from the formats above. Write custom shaders against the same contract: sample premultiplied linear color, and return premultiplied linear color.

## Writing a backend

A backend that lowers filters natively reads `AnyEffect::description()` on the `FilteredView` it receives:

- `FilterDescription::params()` gives the flattened current values, and `collect_stages` reports each stage in application order with the offsets (`Placed::param_base`, `Placed::image_base`) that place its parameters and images.
- `visit_signals` reports every reactive parameter by index, with `FilterSignal::snapshot` and `watch_animated`. A parameter that is not visited is constant.
- `visit_links` walks the individual filters, and `visit_images` the auxiliary images.

A backend that runs effects on the GPU moves the `AnyEffect` to its render thread instead, and keeps `FilteredView::guards` alive as long as the realization. See [Backends](../08-internals/04-backends.md) for how a backend consumes leaf views.

## Verifying filters

Filter output is judged by eye, not by pixel counts. In the WaterUI repository, the `filtrate` crate renders every built-in filter into `/tmp/waterui_filter_gallery/`; open the PNGs it writes:

```sh
cargo nextest run -p filtrate --lib -E 'test(gpu_export_filter_gallery_images)' --no-capture
```

The `filter` example in the WaterUI repository shows sliders bound to filter parameters with animation. See [Testing](../07-tools/03-testing.md) for the offscreen rendering tools.

## Performance notes

- Consecutive color-only filters compose into one fragment segment, so grouping them costs far less than interleaving them with spatial filters.
- A spatial stage that is materialized needs an intermediate texture; a multi-pass chain ping-pongs between intermediates. `vignette` and `lut_color_grade` are spatial, so they break a run of color filters.
- The backend captures the child view to a texture before filtering, so filtering a large subtree costs a capture plus the passes.
- An animating parameter keeps the surface dirty until it settles. Springs settle on their own; bezier curves run for their declared duration.
- `Effect::trim` drops a chain's recreatable intermediates, which a host calls under memory pressure.

## Next

Filters transform existing content. To generate new visual content on the GPU, continue to [Particle systems](05-particles.md). For the shader basics `ShaderEffect` builds on, see [Shaders](03-shaders.md).

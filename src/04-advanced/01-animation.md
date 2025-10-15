# Animation

WaterUI wraps reactive values with animation metadata. When a value changes, renderers inspect the
attached `Animation` and interpolate frames until the transition completes.

## Declarative animation metadata

All reactive types gain helpers from `AnimationExt` and `SignalExt`.

```rust,ignore
use core::time::Duration;
use waterui::prelude::*;

fn dashboard() -> impl View {
    let opacity = binding(1.0).animated();
    let scale = binding(0.85).with_animation(Animation::ease_in_out(Duration::from_millis(240)));

    vstack((
        text!("Animated headline"),
        text!("{}", opacity.clone()),
    ))
    .scale(scale)
    .opacity(opacity)
}
```

Calling `.animated()` attaches the default easing curve, while `.with_animation` lets you pick a
specific variant. The animation metadata travels with the binding—any view that consumes the binding
will animate when the value changes.

## Coordinating multiple signals

Use `.map` and `.zip` to derive additional animated signals. Animation metadata survives these
transformations.

```rust,ignore
use core::time::Duration;
use waterui::prelude::*;

fn playback_controls() -> impl View {
    let progress = binding(0.0).with_animation(Animation::linear(Duration::from_secs(1)));
    let highlight = progress.clone().map(|value| value > 0.95).animated();

    hstack((
        progress(progress.clone()),
        text!("{:.0}%", progress.map(|v| v * 100.0)),
    ))
    .badge(highlight.map(|active| if active { 1 } else { 0 }))
}
```

The badge animates independently from the progress bar because both bindings carry their own
metadata.

## Spring physics

`Animation::spring` interpolates values with a critically damped spring. Pass stiffness and damping
constants to tune the response.

```rust,ignore
use waterui::prelude::*;

fn bouncing_card() -> impl View {
    let expansion = binding(1.0).with_animation(Animation::spring(120.0, 16.0));

    card(text!("Inspect details"))
        .scale(expansion.clone())
        .on_tap(move || expansion.set(1.1))
}
```

When the tap handler updates `expansion`, WaterUI emits intermediate values according to the spring
model.

## Global animation policies

Install `Hook<TextConfig>` or other configuration hooks in the environment to override defaults.
You can also store bespoke animation helpers alongside services and extract them with `Use<T>`.

```rust,ignore
use core::time::Duration;
use waterui::component::text::TextConfig;
use waterui::prelude::*;
use waterui::view::Hook;
use waterui_core::extract::{Extractor, Use};

#[derive(Clone)]
struct AnimationPolicy {
    emphasize: Animation,
}

fn emphasize_hook(env: &Environment, mut config: TextConfig) -> TextConfig {
    let Use(policy) = Extractor::extract::<Use<AnimationPolicy>>(env).unwrap();
    config.content = config.content.with_animation(policy.emphasize.clone());
    config
}

fn animated_text() -> impl View {
    text!("Status")
        .with(AnimationPolicy {
            emphasize: Animation::ease_out(Duration::from_millis(180)),
        })
        .with(Hook::new(emphasize_hook))
}
```

Hooks run every time WaterUI resolves a view configuration, letting you standardise animation curves
across your app without touching every call site.

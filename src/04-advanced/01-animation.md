# Animation

The animation system lives in `waterui::animation` and is wired into every `Signal` via the
`AnimationExt` trait. Instead of imperatively driving tweens, you attach animation metadata to the
binding that powers your view, and the renderer interpolates between old and new values.

## Animated Bindings

```rust
use waterui::prelude::*;
use waterui::animation::Animation;
use core::time::Duration;

pub fn fading_badge() -> impl View {
    let visible = binding(true);
    let opacity = visible
        .map(|flag| if flag { 1.0 } else { 0.0 })
        .with_animation(Animation::ease_in_out(Duration::from_millis(250)));

    text!("Opacity: {opacity:.0}")
}
```

- `.animated()` applies the platform-default animation.
- `.with_animation(Animation::Linear(..))` lets you choose easing.
- `.with_animation(Animation::spring(stiffness, damping))` yields physically based motion.

When the binding changes, the animation metadata travels with it through `map`, `zip`, or any other
combinator.

## Coordinated Transitions

Attach animation metadata to multiple bindings and update them together:

```rust
use waterui::prelude::*;
use waterui::animation::Animation;

let offset = binding((0.0_f32, 0.0_f32));
let font_size = binding(14.0_f32);

let offset = offset.with_animation(Animation::spring(200.0, 15.0));
let font_size = font_size.animated();

vstack((text!("offset: {offset:?}, size: {font_size}"),))
```

Calling `offset.set((0.0, 50.0))` and `opacity.set(1.0)` triggers both animations concurrently.

## Testing and Debugging

- Run with the `WATERUI_ANIMATION=off` environment variable (or a custom hook) to disable animations
  during snapshot testing.
- When a view fails to animate, ensure the binding changed (animations only run when the value
  differs) and that you applied the animation to the reactive value, not the literal view.

Animations are declarative in WaterUI—keep state updates pure and describe how values should
transition. The runtime handles frame-by-frame interpolation on every platform.
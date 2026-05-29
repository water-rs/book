# Animation

> **In this chapter, you will:**
> - Learn how WaterUI's declarative animation system works with reactive state
> - Use `.animated()` and `.with_animation()` to bring values to life
> - Choose between bezier curves and spring physics for different effects
> - Compose multiple animations that run in parallel
> - Implement custom interpolation for your own types

Your app works, but it feels static. Buttons snap into place, views appear
instantly, and state changes feel jarring. Animation is the difference between
software that *functions* and software that *feels good*. WaterUI's animation
system makes this easy: instead of writing imperative "start animation from A to
B" code, you attach animation metadata to reactive values and let the framework
interpolate automatically whenever those values change.

```text
Reactive Values  --->  Change Propagation  --->  Animation System
(Binding/Compute)      (With Metadata)           (Renderer)
```

## Core Concepts

The animation system lives in `waterui_core::animation` and exposes two
fundamental primitives:

- **Bezier** -- Timed interpolation along a cubic bezier curve. Great for
  predictable, time-based transitions like fades and slides.
- **Spring** -- Physics-based movement with configurable stiffness and damping.
  Ideal for interactions that should feel organic, like drag releases or
  toggles.

Both primitives are variants of the `Animation` enum:

```rust,ignore
pub enum Animation {
    Default,
    Bezier { duration: Duration, x1: f32, y1: f32, x2: f32, y2: f32 },
    Spring { stiffness: f32, damping: f32 },
}
```

Now let's see how to apply these to your reactive values.

## Animated Signals

The `AnimationExt` trait is implemented for every WaterUI reactive signal. It
provides two methods that cover most use cases.

### `.animated()`

The quickest way to add animation. This applies a sensible default (ease-in-out,
250 ms) to any reactive value:

```rust,ignore
use waterui::prelude::*;

let opacity = Binding::f64(1.0);
let animated_opacity = opacity.clone().animated();
```

When `opacity` changes from `1.0` to `0.0`, the renderer will smoothly
transition through intermediate values over 250 ms using an ease-in-out curve.

> **Tip:** `.animated()` is perfect for quick prototyping. You can always switch
> to `.with_animation()` later for finer control.

### `.with_animation(animation)`

When you need a specific curve or duration, use this method instead:

```rust,ignore
use waterui::prelude::*;
use waterui_core::animation::Animation;
use core::time::Duration;

let scale = Binding::f64(1.0);
let animated_scale = scale.with_animation(
    Animation::ease_in_out(Duration::from_millis(300))
);
```

Both methods return a `WithMetadata<Self, Animation>` -- the original signal
wrapped with animation metadata that the renderer inspects during each frame.

## Bezier Animations

Bezier animations use cubic bezier control points to define the easing curve.
The curve starts at `(0, 0)` and ends at `(1, 1)`. The four control-point values
`(x1, y1, x2, y2)` shape the acceleration and deceleration profile.

### Convenience Constructors

WaterUI provides four standard curves that match the CSS easing keywords:

| Constructor | Control Points | Behavior |
|---|---|---|
| `Animation::linear(duration)` | `(0.0, 0.0, 1.0, 1.0)` | Constant velocity |
| `Animation::ease_in(duration)` | `(0.42, 0.0, 1.0, 1.0)` | Starts slow, accelerates |
| `Animation::ease_out(duration)` | `(0.0, 0.0, 0.58, 1.0)` | Starts fast, decelerates |
| `Animation::ease_in_out(duration)` | `(0.42, 0.0, 0.58, 1.0)` | Slow start and end |

If you have worked with CSS transitions before, these will feel familiar.

### Custom Bezier Curves

For fine-grained control, use `Animation::bezier`:

```rust,ignore
use waterui_core::animation::Animation;
use core::time::Duration;

// A bounce-like feel
let bounce = Animation::bezier(
    Duration::from_millis(400),
    0.25, 0.1, 0.25, 1.0,
);
```

> **Note:** The `x1` and `x2` values must be in the range `[0.0, 1.0]`. The
> `y1` and `y2` values are unclamped, allowing overshoot effects. Providing
> out-of-range `x` values or non-finite values panics from inside
> `Animation::bezier`.

## Spring Animations

Sometimes a fixed-duration curve does not capture the right feel. Drag releases,
toggles, and pull-to-refresh interactions feel more natural with physics-based
motion. That is what spring animations are for.

```rust,ignore
use waterui_core::animation::Animation;

// stiffness: how quickly the spring accelerates (higher = faster)
// damping:   how quickly oscillation decays     (higher = less bounce)
let springy = Animation::spring(100.0, 10.0);
```

The physics simulation uses the following model:

- **Underdamped** (`damping / (2 * sqrt(stiffness)) < 1.0`) -- the spring
  overshoots and oscillates before settling.
- **Critically damped** -- the spring reaches its target as fast as possible
  without overshooting.
- **Overdamped** -- the spring approaches the target slowly without overshoot.

> **Tip:** Start with `stiffness: 100.0` and `damping: 10.0`, then tweak from
> there. Lower damping gives more bounce; higher stiffness makes things snappier.

Spring animations do not have a fixed duration. The framework uses a default
duration of 600 ms for timing calculations, but the actual visual completion
depends on the physics parameters.

```rust,ignore
use waterui::prelude::*;
use waterui_core::animation::Animation;

let position = Binding::container((0.0, 0.0));
let animated_pos = position.with_animation(Animation::spring(120.0, 12.0));
```

## The Easing System

Under the hood, `Animation` delegates to `EasingCurve` for progress
calculation. `EasingCurve` is a standalone type in `waterui_core::easing` with
two variants:

```rust,ignore
pub enum EasingCurve {
    CubicBezier(f32, f32, f32, f32),
    Spring { stiffness: f32, damping: f32 },
}
```

You can use `EasingCurve` directly if you need easing outside of the animation
metadata system. Common constants are available:

```rust,ignore
use waterui_core::easing::EasingCurve;

let _ = EasingCurve::LINEAR;       // (0, 0, 1, 1)
let _ = EasingCurve::EASE_IN;      // (0.42, 0, 1, 1)
let _ = EasingCurve::EASE_OUT;     // (0, 0, 0.58, 1)
let _ = EasingCurve::EASE_IN_OUT;  // (0.42, 0, 0.58, 1)
let _ = EasingCurve::EASE;         // (0.25, 0.1, 0.25, 1) -- CSS default
```

## The Animatable Trait

For the animation system to interpolate between two values, the type must
implement `Animatable`. Each animatable value exposes a payload that the
renderer blends frame-by-frame using the lower-level `Interpolatable` trait
from `waterui_core::easing`:

```rust,ignore
pub trait Animatable: Clone {
    type AnimatableData: Interpolatable;
    fn animatable_data(&self) -> Self::AnimatableData;
    fn from_animatable_data(data: Self::AnimatableData) -> Self;
}

pub trait Interpolatable: Clone {
    fn lerp(&self, other: &Self, t: f32) -> Self;
}
```

WaterUI provides built-in `Animatable` implementations for `f32`, `f64`, tuples
up to four elements, and fixed-size arrays `[T; N]` where the element type is
`Animatable + Copy`. The matching `Interpolatable` impls cover the same shapes
on the easing side.

To animate a custom type (for example, a color struct), implement `Animatable`
and pick a tuple or array `AnimatableData` that the easing system already knows
how to interpolate.

## Coordinated Transitions

Real-world UIs rarely animate a single property. A card appearing on screen
might fade in, slide up, and scale all at once. Because each signal carries its
own animation metadata, different properties can use different curves and
durations:

```rust,ignore
use waterui::prelude::*;
use waterui_core::animation::Animation;
use core::time::Duration;

let opacity = Binding::f64(0.0);
let position = Binding::container((0.0, 100.0));
let scale = Binding::f64(0.8);

// Opacity fades in with ease-in-out
let anim_opacity = opacity.with_animation(
    Animation::ease_in_out(Duration::from_millis(300))
);

// Position slides up with a spring
let anim_position = position.with_animation(
    Animation::spring(100.0, 10.0)
);

// Scale grows to 1.0 with ease-out
let anim_scale = scale.with_animation(
    Animation::ease_out(Duration::from_millis(250))
);
```

When you trigger the state change, all three properties animate in parallel:

```rust,ignore
# // Trigger the "appear" state
# let opacity = waterui::prelude::Binding::f64(0.0);
# let position = waterui::prelude::Binding::container((0.0, 100.0));
# let scale = waterui::prelude::Binding::f64(0.8);
opacity.set(1.0);
position.set((0.0, 0.0));
scale.set(1.0);
```

The framework handles each animation independently, so a 300 ms opacity fade
will finish before a bouncy spring position settles.

## Composition with Reactive Operators

Animation metadata composes naturally with `map`, `zip`, and other signal
combinators. This means you can derive animated values from other signals
without any special effort:

```rust,ignore
use waterui::prelude::*;
use waterui_core::animation::Animation;
use core::time::Duration;

let count = Binding::i32(0);

// Map count to opacity, then animate
let opacity = count
    .map(|n: i32| if n > 5 { 1.0 } else { 0.5 })
    .animated();

// Combine two values and animate the result
let value1 = Binding::i32(1);
let value2 = Binding::i32(2);

let combined = value1
    .zip(&value2)
    .map(|(a, b)| a + b)
    .with_animation(Animation::ease_in_out(Duration::from_millis(250)));
```

## Manual Interpolation

If you need to compute intermediate values outside of the signal system (for
example, in a custom view renderer), use `Animation::interpolate` directly.
It accepts the bounds by reference so you can interpolate any
`Animatable` type, including tuples and arrays:

```rust,ignore
use waterui_core::animation::Animation;
use core::time::Duration;

let anim = Animation::ease_in_out(Duration::from_millis(300));
let elapsed = Duration::from_millis(150);

let value = anim.interpolate(&0.0_f32, &100.0_f32, elapsed);
// value is approximately 50.0, but eased

let is_done = anim.is_complete(elapsed); // false
let is_done = anim.is_complete(Duration::from_millis(300)); // true
```

The `progress` method returns the eased progress as a float:

```rust,ignore
# use waterui_core::animation::Animation;
# use core::time::Duration;
# let anim = Animation::ease_in_out(Duration::from_millis(300));
let p = anim.progress(Duration::from_millis(150));
// p is between 0.0 and 1.0, shaped by the easing curve
```

## Using Animations with View Modifiers

Many `ViewExt` modifiers accept reactive values. Passing an animated signal
automatically animates the visual property -- no extra wiring needed:

```rust,ignore
use waterui::prelude::*;
use waterui_core::animation::Animation;

let angle = Binding::f64(0.0).animated();
let x_scale = Binding::f64(1.0).animated();
let y_scale = Binding::f64(1.0).animated();

text("Hello")
    .rotation(angle)
    .scale(x_scale, y_scale);
```

When `angle` or the scale bindings change, the rotation and scale transforms
will animate smoothly to their new values.

> **Try it yourself:** Create a button that toggles between `angle = 0.0` and
> `angle = 3.14`. Watch it spin smoothly each time you tap.

## Summary

| API | Purpose |
|---|---|
| `signal.animated()` | Default ease-in-out animation (250 ms) |
| `signal.with_animation(anim)` | Custom animation configuration |
| `Animation::linear(dur)` | Constant velocity |
| `Animation::ease_in(dur)` | Slow start |
| `Animation::ease_out(dur)` | Slow end |
| `Animation::ease_in_out(dur)` | Slow start and end |
| `Animation::spring(stiff, damp)` | Physics-based spring |
| `Animation::bezier(dur, x1, y1, x2, y2)` | Custom cubic bezier |
| `anim.interpolate(from, to, elapsed)` | Manual value interpolation |
| `anim.progress(elapsed)` | Eased progress `[0, 1]` |
| `anim.is_complete(elapsed)` | Check if animation finished |
| `EasingCurve::ease(t)` | Low-level easing calculation |
| `Interpolatable::lerp(other, t)` | Linear interpolation trait |

## What's Next

Your app now moves smoothly, but users interact with more than taps. In the
[next chapter](02-gestures.md), you will learn how to recognize gestures --
taps, drags, pinches, and rotations -- and pair them with the animations you
just learned.

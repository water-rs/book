# Animation

> **In this chapter, you will:**
> - Attach animation metadata to a signal so the renderer interpolates it
> - Choose between bezier curves and spring physics
> - Animate transforms, shape geometry, and collection membership
> - Implement `Animatable` for your own types
> - Drive a timeline manually with `AnimationTrack`

WaterUI has no "start animation from A to B" call. You attach an `Animation`
to a reactive value, and every later change to that value is interpolated
instead of applied instantly. Because the metadata rides on the signal, any
modifier that already accepts a signal animates for free.

## Attaching an animation to a signal

`SignalExt::with` attaches metadata to a signal's emissions. Pass an
`Animation` and the value becomes animated:

```rust,ignore
use waterui::animation::Animation;
use waterui::prelude::*;

let scale = Binding::f32(1.0);
let animated_scale = scale.with(Animation::spring(300.0, 15.0));

Blue.size(80.0, 80.0)
    .scale(animated_scale.clone(), animated_scale)
```

Setting `scale` to `1.5` now springs the box to its new size. The binding
itself is unchanged — `with` returns a wrapper, so the same binding can feed
several views with different timings.

For the common case, `.animated()` applies WaterUI's default timing
(ease-in-out over 250 ms):

```rust,ignore
use waterui::prelude::*;

let opacity = Binding::f64(1.0);
let fade = opacity.animated();
```

> **Note:** `.animated()` comes from the prelude. Importing
> `waterui::animation::AnimationExt` additionally brings
> `.with_animation(animation)`, a named alias for `.with(animation)`.

## Bezier curves

`Animation::Bezier` is a timed curve through two control points, running from
`(0, 0)` to `(1, 1)`. Four constructors match the CSS easing keywords:

| Constructor | Control points | Behavior |
|---|---|---|
| `Animation::linear(duration)` | `(0.0, 0.0, 1.0, 1.0)` | Constant velocity |
| `Animation::ease_in(duration)` | `(0.42, 0.0, 1.0, 1.0)` | Starts slow, accelerates |
| `Animation::ease_out(duration)` | `(0.0, 0.0, 0.58, 1.0)` | Starts fast, decelerates |
| `Animation::ease_in_out(duration)` | `(0.42, 0.0, 0.58, 1.0)` | Slow start and end |

`Animation::bezier` takes the control points directly:

```rust,ignore
use core::time::Duration;
use waterui::animation::Animation;

let bounce = Animation::bezier(Duration::from_millis(400), 0.25, 0.1, 0.25, 1.0);
```

`x1` and `x2` must lie in `[0.0, 1.0]`; `y1` and `y2` are unclamped so curves
can overshoot. Out-of-range or non-finite values panic inside
`Animation::bezier`.

## Spring physics

Drag releases and toggles feel wrong on a fixed-duration curve. Springs take
stiffness (how hard the spring pulls) and damping (how fast oscillation dies):

```rust,ignore
use waterui::animation::Animation;

let springy = Animation::spring(100.0, 10.0);
```

The damping ratio `damping / (2 * sqrt(stiffness))` decides the character:
below `1.0` the value overshoots and oscillates, at `1.0` it arrives as fast as
possible without overshoot, above `1.0` it eases in slowly. Start at
`(100.0, 10.0)`, lower the damping for more bounce, raise the stiffness to make
it snappier. `Animation::spring` panics on a non-positive stiffness or a
negative damping.

Springs have no natural end time. `Animation::duration()` reports 600 ms for
scheduling purposes, but how long the motion *looks* like it lasts is decided
by the physics parameters.

## Different curves, one state change

Each signal carries its own metadata, so a single state change can drive
several properties on different timings:

```rust,ignore
use core::time::Duration;
use waterui::animation::Animation;
use waterui::prelude::*;

fn card(revealed: &Binding<bool>) -> impl View {
    let opacity = revealed.select(1.0, 0.0)
        .with(Animation::ease_in_out(Duration::from_millis(300)));
    let offset_y = revealed.select(0.0, 100.0)
        .with(Animation::spring(100.0, 10.0));
    let scale = revealed.select(1.0, 0.8)
        .with(Animation::ease_out(Duration::from_millis(250)));

    text("Now you see me")
        .padding()
        .opacity(opacity)
        .offset(0.0, offset_y)
        .scale(scale.clone(), scale)
}
```

Flipping `revealed` starts all three at once, and each settles on its own
schedule. A derived signal animates the same way — attach the metadata after
the `map` or `zip`:

```rust,ignore
use core::time::Duration;
use waterui::animation::Animation;
use waterui::prelude::*;

let count = Binding::i32(0);
let opacity = count.map(|n: i32| if n > 5 { 1.0 } else { 0.5 }).animated();

let width = Binding::f32(0.0);
let height = Binding::f32(0.0);
let area = width
    .zip(&height)
    .map(|(w, h)| w * h)
    .with(Animation::ease_in_out(Duration::from_millis(250)));
```

## What animates, and what only updates

Transforms and opacity are compositor properties: `.scale()`, `.rotation()`,
`.offset()`, and `.opacity()` hand the animation metadata to the platform
animator, so the value is interpolated frame by frame.

Layout parameters are different. Stack spacing and the `Frame` dimensions
accept signals and re-run layout when the signal changes, but the change is
applied in one step — the layout invalidation carries no animation metadata:

```rust,ignore
use waterui::prelude::*;

fn toolbar(compact: &Binding<bool>) -> impl View {
    // Reactive: the stack re-lays out when `compact` flips.
    // Not interpolated: the gap jumps from 20 to 4.
    hstack((text("Cut"), text("Copy"), text("Paste")))
        .spacing(compact.select(4.0, 20.0))
}
```

To animate a size change, animate a transform on top of a fixed layout rather
than animating the layout itself.

## Animating collection membership

Rows appearing and disappearing in a `ForEach` or `List` pop in by default.
Wrap the subtree in `collection_transition` to fade and grow entering items and
fade and collapse exiting ones:

```rust,ignore
use core::time::Duration;
use waterui::animation::Animation;
use waterui::prelude::*;

collection_transition(
    List::for_each(rows.clone(), row_view),
    Animation::ease_out(Duration::from_millis(220)),
)
```

The transition is scoped through the environment, so every reactive collection
inside `content` picks it up. Backends without support render the collection
correctly, just without the transition.

## Shape morphing

Geometry morphing is not a native transform, so WaterUI renders it on the GPU
through its own interpolation pipeline:

```rust,ignore
use core::time::Duration;
use waterui::prelude::*;
use waterui::shape::{Capsule, Circle, Rectangle, RoundedRectangle, ShapeExt};

hstack((
    Circle
        .morph_to(RoundedRectangle::new(0.22), Color::srgb_hex("#3B82F6"))
        .duration(Duration::from_millis(1100))
        .size(90.0, 90.0),
    Rectangle
        .morph_to(Capsule, Color::srgb_hex("#10B981"))
        .duration(Duration::from_millis(900))
        .autoreverse(true)
        .size(128.0, 72.0),
))
```

Morphing supports the SDF-backed built-ins: `Rectangle`, `Circle`, `Ellipse`,
`RoundedRectangle`, `UnevenRoundedRectangle`, and `Capsule`.

## Making your own types animatable

Interpolation is defined by the `Animatable` trait. A type exports a payload
the animation system already knows how to blend, and reconstructs itself from
the blended payload:

```rust,ignore
use waterui::animation::Animatable;

#[derive(Clone)]
struct Rgb {
    r: f32,
    g: f32,
    b: f32,
}

impl Animatable for Rgb {
    type AnimatableData = (f32, f32, f32);

    fn animatable_data(&self) -> Self::AnimatableData {
        (self.r, self.g, self.b)
    }

    fn from_animatable_data(data: Self::AnimatableData) -> Self {
        Self { r: data.0, g: data.1, b: data.2 }
    }
}
```

WaterUI ships `Animatable` for `f32`, `f64`, tuples up to four elements, and
`[T; N]` where `T: Animatable + Copy`. Pick whichever of those shapes matches
your field layout as `AnimatableData` and you never have to write
interpolation math.

## Driving a timeline yourself

Custom renderers sometimes own their frame loop. `AnimationTrack<T>` holds one
value plus its in-flight animation, and you advance it by a frame delta:

```rust,ignore
use core::time::Duration;
use waterui::animation::{Animation, AnimationTrack};

let mut track = AnimationTrack::new(0.0_f32);
track.set_target(1.0, Some(Animation::ease_in_out(Duration::from_millis(120))));

// Once per frame:
let still_running = track.advance(Duration::from_millis(16));
let value = track.value();
```

`advance` returns `false` once the animation has landed on its target, and
`set_target` with `None` (or a zero-duration animation) applies the value
immediately. For a one-off sample without a track, `Animation::interpolate`,
`Animation::progress`, and `Animation::is_complete` take the elapsed time
directly:

```rust,ignore
use core::time::Duration;
use waterui::animation::Animation;

let anim = Animation::ease_in_out(Duration::from_millis(300));
let value = anim.interpolate(&0.0_f32, &100.0_f32, Duration::from_millis(150));
let progress = anim.progress(Duration::from_millis(150));
let done = anim.is_complete(Duration::from_millis(300));
```

## What's next

Animation reacts to state; gestures produce it. In the
[next chapter](02-gestures.md) you will recognize taps, drags, pinches, and
rotations, and feed them into the signals you just learned to animate.

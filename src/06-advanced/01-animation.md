# Animation

> **In this chapter, you will:**
> - Attach an `Animation` to a signal so the renderer interpolates every change
> - Choose between bezier curves and spring physics, and switch animation off
> - Animate collection rows and conditional content with `.transition(...)`
> - Morph one built-in shape into another
> - Implement `Animatable` and drive an `AnimationTrack` yourself

WaterUI has no "start animation from A to B" call. You attach an `Animation`
to a reactive value, and every later change to that value is interpolated
instead of applied instantly. Because the metadata rides on the signal, every
modifier or control that reads it animates the change with no further code.
Structural changes (a row appearing, a view being replaced) are the second
half of the story: they animate through a declared `Transition`, covered
further down.

## Attaching an animation to a signal

`SignalExt::with` attaches metadata to a signal's emissions. Pass an
`Animation` and the value becomes animated:

```rust
use waterui::animation::Animation;
use waterui::prelude::*;

fn pulsing_box(scale: &Binding<f32>) -> impl View {
    let animated = scale.with(Animation::spring(300.0, 15.0));

    Blue.size(80.0, 80.0).scale(animated.clone(), animated)
}
```

Setting `scale` to `1.5` now springs the box to its new size. The binding
itself is unchanged: `with` returns a wrapper, so the same binding can feed
several views with different timings. `Animation` is not in the prelude;
import it from `waterui::animation`.

For the common case, `.animated()` attaches `Animation::Default`, the system
timing. For a property change it is a 250 ms ease-in-out, which is what
`Animation::duration()` and `Animation::curve()` report:

```rust
use waterui::prelude::*;

fn fading(opacity: &Binding<f32>) -> impl View {
    text("Hello").opacity(opacity.animated())
}
```

`.animated()` comes from `AnimationExt`, which the prelude re-exports. It is
the framework's only animation extension trait, so `waterui::animation::AnimationExt`
and the prelude name the same thing, and `.with(animation)` is the one spelling
for attaching an explicit animation.

### Switching animation off

A signal that names no animation leaves the choice to the receiver, and a
control that owns its motion (a toggle, a slider, a progress bar) may still
animate the change with its own default. `NoAnimation` states the opposite: the
change must apply in the next frame.

```rust
use waterui::animation::NoAnimation;
use waterui::prelude::*;

fn snapping(angle: &Binding<f32>) -> impl View {
    Blue.size(60.0, 60.0).rotation(angle.with(NoAnimation))
}
```

`NoAnimation` wins over an animation attached anywhere else on the same
change, so `angle.animated().with(NoAnimation)` snaps too. Backends read the
three cases (`Animated`, `Unspecified`, `Disabled`) through
`waterui::animation::ChangeAnimation::of`; app code rarely needs it.

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

```rust
use core::time::Duration;
use waterui::animation::Animation;

let gentle = Animation::bezier(Duration::from_millis(400), 0.25, 0.1, 0.25, 1.0);
```

`x1` and `x2` must lie in `[0.0, 1.0]`, every value must be finite, and `y1`
and `y2` are otherwise unclamped so curves can overshoot. The constructors are
plain data: a debug build asserts the contract, and a consumer that takes an
`Animation` from app code calls `Animation::validated`, which panics in debug
builds and logs and falls back to `Animation::Default` in release builds.

## Spring physics

Drag releases and toggles feel wrong on a fixed-duration curve. Springs take
stiffness (how hard the spring pulls) and damping (how fast oscillation dies):

```rust
use waterui::animation::Animation;

let springy = Animation::spring(100.0, 10.0);
```

The damping ratio `damping / (2 * sqrt(stiffness))` decides the character:
below `1.0` the value overshoots and oscillates, at `1.0` it arrives as fast as
possible without overshoot, above `1.0` it eases in slowly. Start at
`(100.0, 10.0)`, lower the damping for more bounce, raise the stiffness to make
it snappier. Stiffness must be finite and positive and damping finite and not
negative.

Springs have no natural end time. `Animation::duration()` reports 600 ms for
scheduling purposes, but how long the motion looks like it lasts is decided by
the physics parameters.

## Curves as values

`Animation::curve()` returns the `EasingCurve` behind an animation, and
`EasingCurve` is also usable on its own (`waterui::easing::EasingCurve`). It has
exactly two variants, `CubicBezier` and `Spring`, plus the constants `LINEAR`,
`EASE_IN`, `EASE_OUT`, `EASE_IN_OUT` and the CSS `EASE`:

```rust
use core::time::Duration;
use waterui::animation::Animation;
use waterui::easing::EasingCurve;

let curve = Animation::ease_in(Duration::from_millis(200)).curve();
assert_eq!(curve, EasingCurve::EASE_IN);

// Ease-out is ahead of linear at the halfway mark.
assert!(EasingCurve::EASE_OUT.ease(0.5) > 0.5);
```

## Different curves, one state change

Each signal carries its own metadata, so a single state change can drive
several properties on different timings:

```rust
use core::time::Duration;
use waterui::animation::Animation;
use waterui::prelude::*;

fn card(revealed: &Binding<bool>) -> impl View {
    let opacity = revealed
        .select(1.0_f32, 0.0_f32)
        .with(Animation::ease_in_out(Duration::from_millis(300)));
    let offset_y = revealed
        .select(0.0_f32, 100.0_f32)
        .with(Animation::spring(100.0, 10.0));
    let scale = revealed
        .select(1.0_f32, 0.8_f32)
        .with(Animation::ease_out(Duration::from_millis(250)));

    text("Now you see me")
        .padding()
        .opacity(opacity)
        .offset(0.0, offset_y)
        .scale(scale.clone(), scale)
}
```

Flipping `revealed` starts all three at once, and each settles on its own
schedule. A derived signal animates the same way: attach the metadata after
the `map` or `zip`.

```rust
use core::time::Duration;
use waterui::animation::Animation;
use waterui::prelude::*;

let count = Binding::i32(0);
let opacity = count.map(|n: i32| if n > 5 { 1.0_f32 } else { 0.5 }).animated();

let width = Binding::f32(0.0);
let height = Binding::f32(0.0);
let area = width
    .zip(&height)
    .map(|(w, h)| w * h)
    .with(Animation::ease_in_out(Duration::from_millis(250)));
```

## What reads animation metadata

The transform modifiers `.scale()`, `.rotation()` and `.offset()` and the
`.opacity()` modifier take a signal and hand its animation to the platform
animator, so the value is interpolated frame by frame. `.rotation()` is in
degrees. Controls that own a value, such as `progress`, toggles and sliders,
read the metadata of their bound signal as well. The example in
`examples/animation` uses buttons to drive the scale, rotation and offset
transforms and a progress bar.

Layout parameters such as a stack's `spacing` or a `Frame`'s `width` accept
signals and re-run layout when they change. To move something smoothly, animate a transform
on top of a stable layout instead of animating the layout itself.

## Animating structure with transitions

A signal animation interpolates a value. It cannot animate a view that is
being created or destroyed. For that, `View::transition` declares how a view
enters and leaves:

```rust
use core::time::Duration;
use waterui::Identifiable;
use waterui::PropertyTransition;
use waterui::animation::Animation;
use waterui::layout::stack::VStack;
use waterui::prelude::*;
use waterui::reactive::collection::List as ReactiveList;

#[derive(Clone, Identifiable)]
struct Message {
    #[id]
    id: u64,
    title: Str,
}

fn inbox(rows: &ReactiveList<Message>) -> impl View {
    VStack::for_each(rows.clone(), |message: Message| {
        text(message.title)
            .transition(PropertyTransition::opacity())
            .animation(Animation::ease_out(Duration::from_millis(220)))
    })
    .spacing(4.0)
}
```

`.transition(...)` returns metadata whose `.animation(...)` sets the timing
shared by the whole transition (the default is `Animation::Default`). Value
updates never trigger it; only structural events do. Applied to a collection
row it follows that row's membership; applied to a `Dynamic` (the view behind
`watch` and `when`, see [the conditional chapter](../03-ui/06-conditional.md))
it decorates the content that replaces the old content.

Removal has a defined shape, specified in section 4.5 of
[the layout spec](https://github.com/water-rs/waterui/blob/dev/docs/layout-spec.md):

- The leaving view exits the accessibility tree, hit testing and focus
  immediately. What remains is a non-interactive visual.
- A row leaving a collection keeps a layout slot at the size it last measured
  until the transition completes; then the siblings move into the freed space
  using the container's animation.
- Content a `watch` or `when` replaces holds no slot: the enclosing layout sizes
  the live content alone, and the old visual paints at its last frame beneath it
  until its transition completes.
- An inserted row gets its slot at once at its normal size and animates only its
  content. Re-inserting an item that is still leaving creates a new row; the old
  visual finishes on its own clock.

The declaration is read from the row's outermost wrapper, so put `.transition(...)`
last in the modifier chain. A row that declares no transition is inserted and
removed at once, and its slot is released immediately.

### The transition vocabulary

`PropertyTransition` builds the native-property effects. Each is a progress
between the presented view and an absent endpoint, played backwards on
insertion:

| Constructor | Absent state |
|---|---|
| `PropertyTransition::opacity()` | Fully transparent |
| `PropertyTransition::scale(factor)` | Scaled about the center |
| `PropertyTransition::move_in(edge)` | One slot extent toward an `Edge` |
| `PropertyTransition::blur(radius)` | Blurred by a radius in logical pixels |
| `PropertyTransition::rotation(radians)` | Rotated clockwise |
| `PropertyTransition::slide()` | Enters from leading, leaves toward trailing |

`Edge` has four logical cases, `Top`, `Bottom`, `Leading` and `Trailing`;
leading and trailing flip in right-to-left layouts. Transitions compose with
the `Transition` trait, and `AnyTransition::asymmetric` gives insertion and
removal different effects:

```rust
use core::time::Duration;
use waterui::animation::Animation;
use waterui::prelude::*;
use waterui::{AnyTransition, Edge, PropertyTransition, Transition};

fn toast_transition() -> impl Transition {
    AnyTransition::asymmetric(
        PropertyTransition::move_in(Edge::Bottom).combined(PropertyTransition::opacity()),
        PropertyTransition::scale(0.9).combined(PropertyTransition::opacity()),
    )
}

fn toast(message: Str) -> impl View {
    text(message)
        .padding()
        .transition(toast_transition())
        .animation(Animation::ease_out(Duration::from_millis(200)))
}
```

Combined effects share one timing and one progress signal.

### Reduced motion

Every transition declares its reduced form through `Transition::reduced`,
which defaults to a plain opacity fade. When the `ReduceMotion(true)` flag is
in the environment, each transition site plays the reduced form instead:
movement, scale, rotation and blur collapse to a fade, and a combined or
asymmetric transition reduces each part.

The Apple and Hydrolysis backends read the flag from the environment, but
neither sets it from the system accessibility preference, so you install it
yourself for the subtree it covers:

```rust
use waterui::ReduceMotion;
use waterui::prelude::*;

fn settings(reduce_motion: bool) -> impl View {
    vstack((text("Notifications"), text("Privacy"))).with(ReduceMotion(reduce_motion))
}
```

A test puts it in the `Environment` it mounts with:

```rust
use waterui::Environment;
use waterui::ReduceMotion;

let mut env = Environment::new();
env.insert(ReduceMotion(true));
```

Pixel-effect transitions (those that capture the view and draw through a GPU
effect) are a backend-facing part of the `Transition` trait (`body` and
`captures_content`); the `PropertyTransition` family covers app code.

## Scrolling with an animation

`ScrollController::animate_to` takes the same `Animation`, so a scripted
scroll moves with the curve you choose; `Animation::Default` there plays the
backend's own scroll motion. A later request, or the user scrolling, takes
over a flight still in progress:

```rust
use waterui::animation::Animation;
use waterui::layout::{Point, scroll::ScrollController};

let controller = ScrollController::new(Point::zero());
controller.scroll_to(Point::new(0.0, 2_400.0));
controller.animate_to(Point::zero(), Animation::spring(100.0, 10.0));
```

Attach the controller to a scroll view with `ScrollView::scroll_controller`.
See [the lists chapter](../03-ui/05-lists.md) and
[the layout chapter](../03-ui/02-layout.md) for the scroll views it drives.

## Shape morphing

A shape is not a value a modifier interpolates, so `waterui::shape` provides
morphing directly through `ShapeExt::morph_to`. It draws the in-between
silhouettes live through a `Canvas`, so every backend produces the same outline
at every progress, in square and non-square boxes alike:

```rust
use core::time::Duration;
use waterui::prelude::*;
use waterui::shape::{Capsule, Circle, Rectangle, RoundedRectangle, ShapeExt};

fn morphs() -> impl View {
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
}
```

By default a morph repeats and reverses every other cycle (900 ms per cycle,
ease-in-out). `MorphShape` configures that with `.duration`, `.easing`,
`.repeat` and `.autoreverse`, or takes a whole `MorphAnimation`; `MorphAnimation::once`
builds a one-shot. To drive the morph yourself, pass a progress signal in
`[0, 1]`, which takes precedence over the clock:

```rust
use waterui::prelude::*;
use waterui::shape::{Circle, Rectangle, ShapeExt};

fn manual_morph(progress: &Binding<f32>) -> impl View {
    Circle
        .morph_to(Rectangle, Color::srgb_hex("#EF4444"))
        .progress(progress.clone())
        .size(100.0, 100.0)
}
```

Morphing supports the built-in shapes: `Rectangle`, `Circle`, `Ellipse`,
`Capsule`, `RoundedRectangle`, `UnevenRoundedRectangle`,
`FixedRoundedRectangle` and `FixedUnevenRoundedRectangle`. A custom `Path` has
no parameters to interpolate, so `morph_to` panics when either end is one.

## Making your own types animatable

Interpolation is defined by two small traits. `Interpolatable` (in
`waterui::easing`) blends two values at a progress `t`; `Animatable` (in
`waterui::animation`) maps a type to a payload that is `Interpolatable` and back:

```rust
use core::time::Duration;
use waterui::animation::{Animatable, Animation};
use waterui::easing::Interpolatable;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Rgb {
    r: f32,
    g: f32,
    b: f32,
}

impl Interpolatable for Rgb {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            r: self.r.lerp(&other.r, t),
            g: self.g.lerp(&other.g, t),
            b: self.b.lerp(&other.b, t),
        }
    }
}

impl Animatable for Rgb {
    type AnimatableData = Self;

    fn animatable_data(&self) -> Self::AnimatableData {
        *self
    }

    fn from_animatable_data(data: Self::AnimatableData) -> Self {
        data
    }
}

let red = Rgb { r: 1.0, g: 0.0, b: 0.0 };
let blue = Rgb { r: 0.0, g: 0.0, b: 1.0 };
let mid = Animation::linear(Duration::from_millis(100)).interpolate(
    &red,
    &blue,
    Duration::from_millis(50),
);
```

The framework implements `Interpolatable` for `f32`, `f64`, tuples up to four
elements and arrays, and `Animatable` for the same shapes. If your type is a
bag of those, export a tuple or array as `AnimatableData` and you never write
the blend yourself.

## Driving a timeline yourself

Custom renderers sometimes own their frame loop. `AnimationTrack<T>` holds one
value plus its in-flight animation, and you advance it by a frame delta:

```rust
use core::time::Duration;
use waterui::animation::{Animation, AnimationTrack};

let mut track = AnimationTrack::new(0.0_f32);
track.set_target(1.0, Some(Animation::ease_in_out(Duration::from_millis(120))));
assert!(track.is_active());

// Once per frame, with the frame's delta:
while track.advance(Duration::from_millis(16)) {
    let _sample = track.value();
}
assert_eq!(track.value(), 1.0);
```

`advance` returns `true` while the animation is still running and lands the
value exactly on the target when it completes. `set_target` with `None`, or
with a zero-duration animation, applies the value immediately. For a one-off
sample without a track, `Animation::interpolate`, `Animation::progress` and
`Animation::is_complete` take the elapsed time directly.

## Testing animation

The animation clock is virtual in `waterui-testing`: it advances when frames are
pumped, never with wall-clock time. A test that sleeps freezes the clock and
captures the end state while still passing, so tests never call
`std::thread::sleep`. Sample a phase with `OffscreenApp::pump_for`:

```rust,ignore
use std::time::Duration;
use waterui_testing::{Role, TestStyle, ui};

let mut rows = ReactiveList::from(vec![
    Message { id: 1, title: "Welcome".into() },
    Message { id: 2, title: "Invoice".into() },
]);
let view_rows = rows.clone();
let mut app = ui()
    .viewport(240, 160)
    .theme(TestStyle::new())
    .mount_offscreen(move || inbox(&view_rows));

// The removed row leaves the accessibility tree on the next frame...
let _ = rows.remove(0);
app.pump_for(Duration::from_millis(16));
assert_eq!(app.query().role(Role::LABEL).all().len(), 1);

// ...while its 220 ms removal holds the slot; sample mid-exit, then settled.
app.pump_for(Duration::from_millis(90));
app.pump_for(Duration::from_millis(200));
```

The fragment reuses `inbox` and `Message` from the transition example above
and needs `waterui-testing` as a dev-dependency.
`pump_until(timeout, condition)` pumps frames until a condition holds. See
[the testing chapter](../07-tools/03-testing.md) for the query and snapshot
APIs.

## What's next

Animation reacts to state; gestures produce it. In the
[next chapter](02-gestures.md) you will recognize taps, drags, pinches and
rotations, and feed them into the signals you just learned to animate. For the
signals themselves, see [the reactive chapter](../02-core/02-reactive.md), and
for the modifiers the transforms come from, [the modifiers chapter](../02-core/04-modifiers.md).

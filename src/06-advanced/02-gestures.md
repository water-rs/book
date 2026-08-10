# Gestures and haptics

> **In this chapter, you will:**
> - Predict which view receives a touch under WaterUI's hit-testing model
> - Attach tap, long-press, drag, pinch, and rotation gestures to views
> - Inject reactive state and pull it back with the `State<T>` extractor
> - Compose gestures sequentially, simultaneously, and with priority
> - Add haptic feedback, and know what it costs in dependencies

Gesture descriptors in WaterUI are plain data. You describe what should be
recognized; each backend translates that into a platform gesture recognizer.
Nothing in the descriptor knows about touch coordinates or timers.

## Hit-testing model

WaterUI passes touches *through* views that have no interaction of their own:

- Non-interactive views (plain `Text`, `Spacer`, layout containers) never
  intercept a touch. It falls through to whatever is behind them in Z-order.
- Interactive views (`Button`, anything carrying a `GestureObserver`) capture
  touches inside their bounds.

So in a `ZStack` or an overlay, the topmost *interactive* view at the touch
point wins — not simply the topmost view.

```rust,ignore
use waterui::prelude::*;

zstack((
    video_player(url).show_controls(true),
    vstack((
        spacer(),                                 // transparent to touch
        button("Play").action(|| { /* ... */ }),  // captures touch
    )),
))
```

If a tap is not reaching the view you expect, look for an interactive element
in the overlay above it.

## Gesture types

The descriptors live in `waterui::gesture`.

```rust,ignore
use waterui::gesture::{
    DragGesture, LongPressGesture, MagnificationGesture, RotationGesture, TapGesture,
};

let single = TapGesture::new();                  // one tap
let double = TapGesture::repeat(2);              // two consecutive taps
let press = LongPressGesture::new(500);          // hold for 500 time units
let drag = DragGesture::new(5.0);                // 5pt minimum travel
let pinch = MagnificationGesture::new(1.0);      // initial scale factor
let rotation = RotationGesture::new(0.0);        // initial angle, radians
```

`LongPressGesture`'s duration is a raw `u32` that each backend interprets in
its own time unit — in practice, milliseconds. The `.on_long_press_gesture()`
modifier below documents its argument as milliseconds explicitly.

Every descriptor converts into the `Gesture` enum, which also holds the
composition variants:

```rust,ignore
use waterui::gesture::{Gesture, TapGesture};

let gesture: Gesture = TapGesture::new().into();
```

## Event payloads

When a backend recognizes a gesture it puts a payload into the environment,
which a handler reads with the `Use<T>` extractor:

| Event type | Fields |
|---|---|
| `TapEvent` | `location: GesturePoint`, `count: u32` |
| `LongPressEvent` | `location: GesturePoint`, `duration: f32` |
| `DragEvent` | `phase`, `location`, `translation`, `velocity` |
| `MagnificationEvent` | `phase`, `center`, `scale`, `velocity` |

`GesturePhase` is `Started`, `Updated`, `Ended`, or `Cancelled`.

Rotation is the asymmetry here: `RotationGesture` is recognized and fires its
handler, but there is no `RotationEvent` payload type yet, so a handler cannot
read the current angle. Track rotation with a `Binding` you update yourself
until that payload lands.

## Attaching a gesture

`ViewExt::gesture` is the general form. Its first argument is anything that
implements `Into<Gesture>`; its second is any `Handler<Args, ()>` — a bare
closure, or one that names extractors like `State<T>`, as described in
[Resolvers and hooks](08-resolvers.md).

```rust,ignore
use waterui::gesture::TapGesture;
use waterui::prelude::*;

text("Tap me").gesture(TapGesture::new(), || tracing::info!("tapped"))
```

The shorthands cover the common cases:

| Modifier | Equivalent |
|---|---|
| `.on_tap(action)` | `.gesture(TapGesture::new(), action)` |
| `.on_tap_gesture(action)` | alias for `.on_tap` |
| `.on_tap_gesture_count(n, action)` | `.gesture(TapGesture::repeat(n), action)` |
| `.on_long_press_gesture(ms, action)` | `.gesture(LongPressGesture::new(ms), action)` |

`.simultaneous_gesture(g, action)` and `.high_priority_gesture(g, action)`
exist for SwiftUI-shaped code, but both currently forward straight to
`.gesture(...)` — they do not yet change recognition precedence. Compose the
gestures explicitly (below) when precedence actually matters.

To reuse a descriptor plus its handler, build a `GestureObserver` and attach it
with `.gesture_observer(...)`:

```rust,ignore
use waterui::gesture::{GestureObserver, TapGesture};
use waterui::prelude::*;

let counter = Binding::i32(0);

text("Count taps")
    .state(&counter)
    .gesture_observer(GestureObserver::new(
        TapGesture::repeat(2),
        |State(counter): State<Binding<i32>>| *counter.get_mut() += 1,
    ))
```

## Getting state into a handler

Handlers are resolved from the environment, not from captured variables. Inject
a value with `ViewExt::state`, then name it in the handler signature through
`State<T>`. Values are keyed by type, so one injection serves every handler in
the subtree:

```rust,ignore
use waterui::gesture::TapGesture;
use waterui::prelude::*;

let count = Binding::i32(0);

text!("Tapped {count} times")
    .padding()
    .background(Color::srgb(200, 220, 255))
    .state(&count)
    .gesture(
        TapGesture::new(),
        |State(count): State<Binding<i32>>| *count.get_mut() += 1,
    )
```

Stack `.state(...)` calls to inject several values, and name one extractor per
value:

```rust,ignore
use waterui::prelude::*;

let count = Binding::i32(0);
let status = Binding::container(Str::from("Ready"));

text("Interact")
    .state(&count)
    .state(&status)
    .on_tap(
        |State(count): State<Binding<i32>>, State(status): State<Binding<Str>>| {
            *count.get_mut() += 1;
            status.set(Str::from("Tapped"));
        },
    )
```

Keep this to three injected values or fewer. Beyond that, bundle the state in
one `#[derive(Clone)]` struct and inject that instead — a handler with four
`State<Binding<T>>` parameters is a sign the state belongs together.

## Combining gestures

Every descriptor carries the three composition methods, and each produces a
`Gesture::Then`, `Gesture::Simultaneous`, or `Gesture::Exclusive`.

```rust,ignore
use waterui::gesture::{DragGesture, LongPressGesture, TapGesture};

// Sequential: the long press only starts after the tap completes.
let chained = TapGesture::new().then(LongPressGesture::new(300));

// Simultaneous: both may be recognized at once.
let combined = TapGesture::new().simultaneously_with(DragGesture::new(8.0));

// Exclusive: the tap has priority, the long press is the fallback.
let exclusive = TapGesture::new().exclusively_before(LongPressGesture::new(500));
```

`.sequenced_before()` is an alias for `.then()`. Compositions nest, so
`a.then(b).simultaneously_with(c)` is valid.

## Haptic feedback

Haptics are behind the non-default `std` feature, which pulls in the
`waterkit-haptic` crate:

```toml
waterui = { version = "0.2", features = ["std"] }
```

With that enabled, `.on_tap_haptic_default(action)` fires a medium impact
before running the action:

```rust,ignore
use waterui::prelude::*;

text("Save").on_tap_haptic_default(|| tracing::info!("saved"))
```

`.on_long_press_haptic_default(ms, action)` is the long-press equivalent. The
explicit-intensity forms, `.on_tap_haptic(intensity, action)` and
`.on_long_press_haptic(ms, intensity, action)`, take a
`waterkit_haptic::Intensity` (`LOW`, `MEDIUM`, `HIGH`, `MAX`, or
`Intensity::new(value)`). That type is not re-exported through `waterui`, so
using them means adding `waterkit-haptic` to your own `Cargo.toml`.

Haptics are implemented for iOS, macOS, Android, Windows, and Linux. Where the
platform cannot deliver one, the failure is logged at debug level and the
action still runs — a gesture never silently stops working because a device has
no haptic engine.

## Putting it together

```rust,ignore
use waterui::gesture::{DragGesture, LongPressGesture, TapGesture};
use waterui::prelude::*;

fn gesture_demo() -> impl View {
    let taps = Binding::i32(0);
    let presses = Binding::i32(0);
    let drags = Binding::i32(0);
    let chained = Binding::container(Str::from("Waiting"));

    scroll(vstack((
        text("Gesture demo").title(),

        text!("Taps: {taps}")
            .padding()
            .background(Color::srgb(33, 150, 243).with_opacity(0.3))
            .state(&taps)
            .gesture(
                TapGesture::new(),
                |State(c): State<Binding<i32>>| *c.get_mut() += 1,
            ),

        text!("Long presses: {presses}")
            .padding()
            .background(Color::srgb(255, 152, 0).with_opacity(0.3))
            .state(&presses)
            .gesture(
                LongPressGesture::new(500),
                |State(c): State<Binding<i32>>| *c.get_mut() += 1,
            ),

        text!("Drags: {drags}")
            .padding()
            .size(200.0, 100.0)
            .background(Color::srgb(156, 39, 176).with_opacity(0.3))
            .state(&drags)
            .gesture(
                DragGesture::new(5.0),
                |State(c): State<Binding<i32>>| *c.get_mut() += 1,
            ),

        text!("{chained}")
            .padding()
            .background(Color::srgb(244, 67, 54).with_opacity(0.3))
            .state(&chained)
            .gesture(
                TapGesture::new().then(LongPressGesture::new(300)),
                |State(s): State<Binding<Str>>| s.set(Str::from("Chain complete")),
            ),
    )))
}
```

Every counter is displayed with `text!`, so the label updates from the binding
without rebuilding the view that owns the gesture.

> **Try it yourself:** give the first box a double tap as well, using
> `TapGesture::repeat(2).exclusively_before(TapGesture::new())`, and watch how
> the single-tap fallback waits for the double-tap window to expire.

## What's next

A gesture that starts a network request needs somewhere to show progress. In
the [next chapter](03-suspense.md) you will handle async work with `Suspense`
and render a loading state while the data is in flight.

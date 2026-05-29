# Gestures and Haptics

> **In this chapter, you will:**
> - Understand WaterUI's hit-testing model for touch events
> - Attach tap, long-press, drag, pinch, and rotation gestures to views
> - Inject reactive state and pull it back through the `State<T>` extractor
> - Compose gestures sequentially, simultaneously, and with priority
> - Add haptic feedback to make interactions feel tangible

A button click is just the beginning. Real apps need drag-to-reorder, pinch-to-zoom,
long-press context menus, and double-tap shortcuts. WaterUI provides a declarative
gesture system where gesture descriptors are lightweight data structures that
backends translate into platform-native gesture recognizers. You describe *what*
you want to recognize, and the platform handles the *how*.

## Hit-Testing Model

Before diving into gestures, it is important to understand how WaterUI decides
which view receives a touch event.

WaterUI uses a **pass-through** model:

- Non-interactive views (plain `Text`, `Spacer`, layout containers) are
  transparent to touch events. Touches fall through to views behind them in the
  Z-order.
- Interactive views (`Button`, views with a `GestureObserver` attached) capture
  touches within their bounds.

In a `ZStack` or `overlay`, only the topmost *interactive* view at a touch
location receives the event.

```rust,ignore
use waterui::prelude::*;

zstack((
    VideoPlayer::new(url).show_controls(true),
    vstack((
        spacer(),  // non-interactive: touches pass through to VideoPlayer
        button("Play").action(|| { /* ... */ }),  // captures touches
    )),
))
```

> **Note:** If you find that taps are not reaching a view behind an overlay,
> check whether the overlay contains any interactive elements that might be
> capturing the touch.

## Gesture Types

All gesture descriptors live in `waterui::gesture`. Each type captures the
minimum configuration required for a backend to register the interaction.

### TapGesture

Recognizes one or more consecutive taps:

```rust,ignore
use waterui::gesture::TapGesture;

let single = TapGesture::new();       // single tap
let double = TapGesture::repeat(2);   // double tap
let triple = TapGesture::repeat(3);   // triple tap
```

### LongPressGesture

Activates after the pointer is held for a minimum duration:

```rust,ignore
use waterui::gesture::LongPressGesture;

let press = LongPressGesture::new(500); // 500ms minimum hold
```

The duration unit is interpreted by each backend (typically milliseconds).

### DragGesture

Begins after the pointer moves beyond a minimum distance:

```rust,ignore
use waterui::gesture::DragGesture;

let drag = DragGesture::new(5.0); // 5pt minimum travel
```

### MagnificationGesture

Recognizes pinch-to-zoom interactions:

```rust,ignore
use waterui::gesture::MagnificationGesture;

let pinch = MagnificationGesture::new(1.0); // initial scale factor
```

### RotationGesture

Recognizes two-finger rotation:

```rust,ignore
use waterui::gesture::RotationGesture;

let rotation = RotationGesture::new(0.0); // initial angle in radians
```

All of these types can be converted into the unified `Gesture` enum, which also
contains composition variants (`Then`, `Simultaneous`, `Exclusive`) that we will
explore later in this chapter.

```rust,ignore
use waterui::gesture::{Gesture, TapGesture};

let gesture: Gesture = TapGesture::new().into();
```

## Gesture Event Payloads

When a backend recognizes a gesture, it creates an event payload and places it
in the environment. The payload types carry interaction details:

| Event Type | Fields |
|---|---|
| `TapEvent` | `location: GesturePoint`, `count: u32` |
| `LongPressEvent` | `location: GesturePoint`, `duration: f32` |
| `DragEvent` | `phase`, `location`, `translation`, `velocity` |
| `MagnificationEvent` | `phase`, `center`, `scale`, `velocity` |

`GesturePhase` tracks the lifecycle: `Started`, `Updated`, `Ended`, or
`Cancelled`.

## Attaching Gestures to Views

Now that you know the gesture types, let's attach them to views. WaterUI
offers several approaches, from general-purpose to convenient shorthand.

### The `.gesture()` Method

The most general way to attach a gesture is through `ViewExt::gesture`:

```rust,ignore
use waterui::prelude::*;
use waterui::gesture::TapGesture;

text("Tap Me!")
    .gesture(TapGesture::new(), || {
        tracing::info!("Tapped!");
    })
```

The first argument is anything that implements `Into<Gesture>`. The second is
any type that implements `Handler<Args, ()>` -- a plain closure with no
arguments, a closure that pulls injected state via `State<T>`, or any combination
of extractors documented in [Resolvers and Hooks](08-resolvers.md).

### `.on_tap()`

A convenience shorthand for single-tap gestures:

```rust,ignore
use waterui::prelude::*;

text("Click me")
    .on_tap(|| tracing::info!("Clicked!"))
```

### `.on_tap_gesture()` and `.on_tap_gesture_count()`

`.on_tap_gesture()` is an alias for `.on_tap()`. Use
`.on_tap_gesture_count(n, action)` for multi-tap gestures:

```rust,ignore
use waterui::prelude::*;

text("Double-tap me")
    .on_tap_gesture_count(2, || {
        tracing::info!("Double tapped!");
    })
```

### `.on_long_press_gesture()`

Attach a long-press handler with a minimum duration in milliseconds:

```rust,ignore
use waterui::prelude::*;

text("Long press me")
    .on_long_press_gesture(500, || {
        tracing::info!("Long pressed!");
    })
```

### `.gesture_observer()`

For full control, construct a `GestureObserver` directly. The action is any
type that implements `Handler<Args, ()>`, so you can pull captured state out of
the environment with the `State<T>` extractor:

```rust,ignore
use waterui::prelude::*;
use waterui::gesture::{GestureObserver, TapGesture};
use waterui::reactive::binding;

let counter = binding(0i32);

text("Count taps")
    .state(&counter)
    .gesture_observer(GestureObserver::new(
        TapGesture::new(),
        |State(counter): State<Binding<i32>>| counter.set(counter.get() + 1),
    ))
```

## Stateful Gesture Handlers

Most interactions need access to reactive state -- counting taps, tracking drag
positions, or toggling a boolean. WaterUI keeps the handler ergonomic without a
custom builder: inject state into the subtree's environment with
`ViewExt::state`, then ask for it back through the `State<T>` extractor in the
handler signature.

### Injecting State

`ViewExt::state` clones a binding (or any cloneable value) into the subtree's
environment. The injected value is keyed by type, so every handler in scope can
extract it:

```rust,ignore
use waterui::prelude::*;
use waterui::gesture::TapGesture;
use waterui::reactive::binding;

let count = binding(0i32);

text("Tap to count")
    .padding()
    .background(Color::srgb(200, 220, 255))
    .state(&count)
    .gesture(
        TapGesture::new(),
        |State(count): State<Binding<i32>>| count.set(count.get() + 1),
    )
```

Stack multiple `.state(...)` calls to inject several values. Each extractor in
the handler tuple pulls one out:

```rust,ignore
use waterui::prelude::*;
use waterui::reactive::binding;

let count = binding(0i32);
let label = binding(String::from("Ready"));

text("Interact")
    .state(&count)
    .state(&label)
    .on_tap(
        |State(count): State<Binding<i32>>,
         State(label): State<Binding<String>>| {
            count.set(count.get() + 1);
            label.set(format!("Tapped {} times", count.get()));
        },
    )
```

### Inside a GestureObserver

The same `State<T>` extractor works when you build a `GestureObserver`
manually -- the action signature is identical to the one you would pass to
`.gesture(...)`:

```rust,ignore
use waterui::gesture::{GestureObserver, TapGesture};
use waterui::prelude::*;
use waterui::reactive::binding;

let counter = binding(0i32);

let observer = GestureObserver::new(
    TapGesture::repeat(2),
    |State(counter): State<Binding<i32>>| counter.set(counter.get() + 1),
);
```

Attach the observer with `view.state(&counter).gesture_observer(observer)` so
the binding is available when the handler fires.

## Combining Gestures

Single gestures are useful, but real interactions often involve combinations.
WaterUI supports three composition modes, mirroring SwiftUI.

### Sequential: `.then()`

The second gesture starts only after the first completes:

```rust,ignore
use waterui::gesture::{TapGesture, LongPressGesture};

let chained = TapGesture::new()
    .then(LongPressGesture::new(300));
// User must tap, then long-press
```

`.sequenced_before()` is an alias for `.then()`.

### Simultaneous: `.simultaneously_with()`

Both gestures can be recognized at the same time:

```rust,ignore
use waterui::gesture::{TapGesture, DragGesture};

let combined = TapGesture::new()
    .simultaneously_with(DragGesture::new(8.0));
```

### Exclusive: `.exclusively_before()`

The first gesture has recognition priority; the second is a fallback:

```rust,ignore
use waterui::gesture::{TapGesture, LongPressGesture};

let exclusive = TapGesture::new()
    .exclusively_before(LongPressGesture::new(500));
```

These composition methods can be chained to build arbitrarily complex gesture
graphs. Each produces a `Gesture::Then`, `Gesture::Simultaneous`, or
`Gesture::Exclusive` variant.

### Priority Modifiers

`ViewExt` provides SwiftUI-style naming for attaching composed gestures
directly to views:

```rust,ignore
use waterui::prelude::*;
use waterui::gesture::DragGesture;

text("Drag or tap")
    .simultaneous_gesture(DragGesture::new(5.0), || {
        tracing::info!("Drag detected");
    })
    .high_priority_gesture(
        waterui::gesture::TapGesture::new(),
        || tracing::info!("Tap wins")
    )
```

## Haptic Feedback

On platforms that support it (iOS, Android), WaterUI integrates with the
`waterkit-haptic` crate to trigger tactile feedback alongside gestures. Haptics
make interactions feel real -- a subtle vibration on a successful action, a
heavier pulse on a destructive one.

### `.on_tap_haptic()`

Combines a tap gesture with haptic impact:

```rust,ignore
use waterui::prelude::*;
use waterkit_haptic::Intensity;

text("Haptic Tap")
    .on_tap_haptic(Intensity::MEDIUM, || {
        tracing::info!("Felt that!");
    })
```

`Intensity` provides constants for common feedback levels. The haptic fires
before the action closure runs.

### `.on_tap_haptic_default()`

Uses `Intensity::MEDIUM` as a sensible default:

```rust,ignore
use waterui::prelude::*;

text("Default Haptic")
    .on_tap_haptic_default(|| {
        tracing::info!("Medium haptic fired");
    })
```

> **Note:** Both haptic methods require the `std` feature flag and are no-ops
> on platforms without haptic hardware.

## Complete Example

Let's put it all together. This example demonstrates multiple gesture types,
state handling, and gesture composition in a single view:

```rust,ignore
use waterui::prelude::*;
use waterui::gesture::{DragGesture, LongPressGesture, TapGesture};
use waterui::reactive::binding;

fn main() -> impl View {
    let tap_count = binding(0i32);
    let long_press_count = binding(0i32);
    let drag_count = binding(0i32);
    let chained_status = binding(String::from("Waiting..."));

    fn bump(c: &Binding<i32>) {
        c.set(c.get() + 1);
    }

    scroll(vstack((
        text("Gesture Demo").title(),

        // Single tap
        text("Tap Me!")
            .padding()
            .background(Color::srgb(33, 150, 243).with_opacity(0.3))
            .state(&tap_count)
            .gesture(
                TapGesture::new(),
                |State(c): State<Binding<i32>>| bump(&c),
            ),

        // Long press
        text("Long Press Me!")
            .padding()
            .background(Color::srgb(255, 152, 0).with_opacity(0.3))
            .state(&long_press_count)
            .gesture(
                LongPressGesture::new(500),
                |State(c): State<Binding<i32>>| bump(&c),
            ),

        // Drag
        text("Drag Here")
            .padding()
            .width(200.0).height(100.0)
            .background(Color::srgb(156, 39, 176).with_opacity(0.3))
            .state(&drag_count)
            .gesture(
                DragGesture::new(5.0),
                |State(c): State<Binding<i32>>| bump(&c),
            ),

        // Chained: tap then long press
        text("Tap then Long Press")
            .padding()
            .background(Color::srgb(244, 67, 54).with_opacity(0.3))
            .state(&chained_status)
            .gesture(
                TapGesture::new().then(LongPressGesture::new(300)),
                |State(s): State<Binding<String>>| {
                    s.set(String::from("Chained gesture completed!"));
                },
            ),
    )))
}
```

> **Try it yourself:** Add a double-tap gesture to one of the views above using
> `TapGesture::repeat(2)`. Can you make it coexist with the single tap using
> `.exclusively_before()`?

## Summary

| API | Purpose |
|---|---|
| `TapGesture::new()` | Single tap |
| `TapGesture::repeat(n)` | Multi-tap |
| `LongPressGesture::new(ms)` | Long press with minimum duration |
| `DragGesture::new(distance)` | Drag with minimum distance |
| `MagnificationGesture::new(scale)` | Pinch-to-zoom |
| `RotationGesture::new(angle)` | Two-finger rotation |
| `.gesture(g, action)` | Attach any gesture to a view |
| `.on_tap(action)` | Single-tap shorthand |
| `.on_tap_gesture_count(n, action)` | Multi-tap shorthand |
| `.on_long_press_gesture(ms, action)` | Long-press shorthand |
| `.gesture_observer(observer)` | Full-control gesture attachment |
| `.state(&binding)` | Inject cloneable state into the subtree environment |
| `State<T>` extractor | Pull injected state into a handler |
| `.then()` | Sequential composition |
| `.simultaneously_with()` | Parallel composition |
| `.exclusively_before()` | Priority composition |
| `.on_tap_haptic(intensity, action)` | Tap with haptic feedback |
| `.on_tap_haptic_default(action)` | Tap with medium haptic |

## What's Next

Your app now responds to rich touch interactions. But what happens when a
gesture triggers a network request? In the [next chapter](03-suspense.md), you
will learn how to handle async operations gracefully with `Suspense`, showing
loading states while data arrives.

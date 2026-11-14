# Gestures, hover, and haptics

> **In this chapter, you will:**
> - Predict which view receives a touch under WaterUI's pass-through hit-testing model
> - Attach tap, long-press, drag, pinch, and rotation gestures, and read their event payloads
> - Compose gestures sequentially, simultaneously, and exclusively
> - React to pointer hover and set the pointer cursor
> - Drive pressed and hover visuals from the interaction state, and add haptic feedback

A gesture in WaterUI is a plain descriptor. You say what should be recognized
and what to run when it is; each backend feeds the same recognizers the same
input, so the thresholds, timing, and the way competing or composed gestures
resolve are identical on every backend. A backend whose native recognizer
differs is non-conforming and is fixed in the backend, not worked around in
your view code.

The gesture types live in `waterui::gesture`, the cursor types in
`waterui::cursor`, and the interaction-state types in `waterui::interaction`.
The prelude brings `GestureObserver` and the `gesture` and `cursor` modules
into scope; the examples below import the other types explicitly.

## Hit-testing model

WaterUI passes touches *through* views that have no interaction of their own:

- Non-interactive views (plain `Text`, `Spacer`, layout containers) never
  intercept a touch. It falls through to whatever is behind them in Z-order.
- Interactive views (`Button`, anything carrying a gesture handler) capture
  touches inside their bounds.

In a `ZStack` or an overlay, the topmost *interactive* view at the touch point
wins, not simply the topmost view. If a tap is not reaching the view you
expect, look for an interactive element in the overlay above it.

`.hittable(enabled)` overrides this per view. It takes
`impl IntoComputed<bool>`, so a plain `bool` or a signal both work. With
`false`, touches pass through the view and everything inside it to whatever is
behind; its appearance does not change. `true` is the default: it does not make
a non-interactive view capture touches.

```rust
use waterui::prelude::*;

fn dismissible_banner() -> impl View {
    let visible: Binding<bool> = binding(true);

    zstack((
        button("Underneath").action(|| waterui::log::info!("reached")),
        // Once dismissed, the banner fades out and stops catching taps,
        // so they reach the button underneath.
        button("Dismiss banner")
            .action(|State(visible): State<Binding<bool>>| visible.set(false))
            .state(&visible)
            .opacity(visible.select(1.0, 0.0))
            .hittable(visible),
    ))
}
```

`.disabled(true)` is the heavier tool: it also disables every control in the
subtree and stops hit-testing there. See
[Controls](../03-ui/03-controls.md).

## Tap, long press, and drag

`ViewExt::gesture` is the general form. Its first argument is anything that
implements `Into<Gesture>`; its second is a handler, written the way
`Button::action` is (see [Controls](../03-ui/03-controls.md)). A bare closure
is a handler.

```rust
use waterui::gesture::TapGesture;
use waterui::prelude::*;

fn tappable() -> impl View {
    text("Tap me").gesture(TapGesture::new(), || waterui::log::info!("tapped"))
}
```

The descriptors and their constructors:

```rust
use waterui::gesture::{
    DragGesture, LongPressGesture, MagnificationGesture, RotationGesture, TapGesture,
};

let single = TapGesture::new();               // one tap
let double = TapGesture::repeat(2);           // two consecutive taps
let press = LongPressGesture::new(500);       // hold for 500 ms
let drag = DragGesture::new(5.0);             // 5 pixels of travel before it starts
let pinch = MagnificationGesture::new(1.0);   // initial scale factor
let rotation = RotationGesture::new(0.0);     // initial angle, radians
```

`LongPressGesture::new` takes a `u32`, not a `Duration`. The type leaves the
unit to the backend; the Apple and Hydrolysis backends both read it as
milliseconds, which is what `.on_long_press_gesture` below documents.
`DragGesture::new` takes the `f32` distance, in logical pixels, the pointer
must travel before the drag begins. A drag *recognizer* only detects the
motion; it moves no data. Moving data between views is
[drag and drop](10-drag-drop.md).

The shorthands cover the common cases:

| Modifier | Equivalent |
|---|---|
| `.on_tap(action)` | `.gesture(TapGesture::new(), action)` |
| `.on_tap_gesture(action)` | alias for `.on_tap` |
| `.on_tap_gesture_count(n, action)` | `.gesture(TapGesture::repeat(n), action)` (a count of 0 becomes 1) |
| `.on_long_press_gesture(ms, action)` | `.gesture(LongPressGesture::new(ms), action)` |

Use `.on_tap` for tappable *content*. Use `button(..)` when the thing is
semantically a button: the button brings platform chrome and the button
accessibility role.

### Pointer buttons

Tap, long-press, and drag respond to the primary button by default (a touch or
pen contact counts as primary). `.buttons(..)` takes a `PointerButtons` set
(`PRIMARY`, `SECONDARY`, `MIDDLE`, `BACK`, `FORWARD`). A gesture that accepts
`SECONDARY` competes with a `.context_menu` at the same point, and the context
menu wins where one is declared.

```rust
use waterui::gesture::{PointerButtons, TapGesture};
use waterui::prelude::*;

fn closable_tab() -> impl View {
    text("Tab").gesture(
        TapGesture::new().buttons(PointerButtons::MIDDLE),
        || waterui::log::info!("middle click closes the tab"),
    )
}
```

## Getting state into a handler

A handler's parameters are extracted from the environment. Inject a value with
`ViewExt::state`, then name it in the handler signature through `State<T>`.
Chain `.state(..)` after the modifier that carries the handler,
`view.gesture(..).state(&value)`, or install it on an ancestor. Values are
keyed by type, so one injection serves every handler in the subtree.

```rust
use waterui::gesture::TapGesture;
use waterui::prelude::*;

fn counter() -> impl View {
    let count: Binding<i32> = binding(0);

    text!("Tapped {count} times")
        .padding()
        .gesture(
            TapGesture::new(),
            |State(count): State<Binding<i32>>| *count.get_mut() += 1,
        )
        .state(&count)
}
```

Because the label is a `text!`, it updates from the binding without rebuilding
the view that owns the gesture. See [Reactive state](../02-core/02-reactive.md).

To reuse a descriptor together with its handler, build a `GestureObserver` and
attach it with `.gesture_observer(..)`. `GestureObserver::new(gesture, action)`
is the constructor, and `GestureObserver::builder(gesture).action(action)` is
the builder form.

```rust
use waterui::gesture::{GestureObserver, TapGesture};
use waterui::prelude::*;

fn double_tap_counter() -> impl View {
    let count: Binding<i32> = binding(0);

    text!("Double-tap count: {count}")
        .gesture_observer(GestureObserver::new(
            TapGesture::repeat(2),
            |State(count): State<Binding<i32>>| *count.get_mut() += 1,
        ))
        .state(&count)
}
```

## Event payloads

When a backend recognizes a gesture it puts a payload into the environment.
A handler reads it with the `Use<T>` extractor, which is in the prelude.

| Event type | Accessors |
|---|---|
| `TapEvent` | `location()`, `count()`, `button()` |
| `LongPressEvent` | `location()`, `duration()`, `button()` |
| `DragEvent` | `phase()`, `location()`, `translation()`, `velocity()`, `button()` |
| `MagnificationEvent` | `phase()`, `center()`, `scale()`, `velocity()` |
| `RotationEvent` | `phase()`, `center()`, `angle()`, `velocity()` |

Points are `GesturePoint { x, y }` in logical pixels, in the view's own
coordinate space. `LongPressEvent::duration()` is the hold the gesture
required, in the same unit as `LongPressGesture::new`. Angles are in radians.
`GesturePhase` is `Started`, `Updated`, `Ended`, or `Cancelled`: a drag, pinch,
or rotation runs its handler at every phase, while a tap or long press runs it
once, on recognition.

```rust
use waterui::gesture::{DragEvent, DragGesture, GesturePhase};
use waterui::prelude::*;

fn drag_readout() -> impl View {
    let dx: Binding<f32> = binding(0.0);

    vstack((
        text!("Horizontal travel: {dx}"),
        text("Drag me").padding().gesture(
            DragGesture::new(5.0),
            |Use(event): Use<DragEvent>, State(dx): State<Binding<f32>>| {
                if event.phase() == GesturePhase::Cancelled {
                    dx.set(0.0);
                } else {
                    dx.set(event.translation().x);
                }
            },
        ),
    ))
    .state(&dx)
}
```

Here `.state(&dx)` wraps the whole stack, an ancestor of the gesture, which is
also valid. Two injected values of the same type bind to same-typed parameters
in `.state()` call order; bundling them in a struct of your own is clearer.

## Combining gestures

Every descriptor, and `Gesture` itself, carries four composition methods. Each
returns the erased `Gesture` enum, which is itself `Into<Gesture>`, so a
composition nests and goes straight into `.gesture(..)`:

| Method | Result |
|---|---|
| `.then(g)` (alias `.sequenced_before(g)`) | `Gesture::Then`: `g` is armed once the first is recognized, and the handler runs when `g` is |
| `.simultaneously_with(g)` | `Gesture::Simultaneous`: the handler runs whenever either is recognized; neither blocks the other |
| `.exclusively_before(g)` | `Gesture::Exclusive`: whichever is recognized first runs the handler, and any recognition in the next 50 ms is ignored |

The handler of a composition receives the event of the child that completed
it, so a tap-then-long-press reads a `LongPressEvent`.

```rust
use waterui::gesture::{DragGesture, LongPressGesture, TapGesture};
use waterui::prelude::*;

fn composed() -> impl View {
    let status: Binding<&'static str> = binding("Waiting");

    vstack((
        text!("{status}"),
        text("Tap, then hold").padding().gesture(
            TapGesture::new().then(LongPressGesture::new(300)),
            |State(status): State<Binding<&'static str>>| status.set("Chain complete"),
        ),
        text("Hold or tap").padding().gesture(
            // Whichever is recognized first runs the handler.
            LongPressGesture::new(500).exclusively_before(TapGesture::new()),
            || waterui::log::info!("hold or tap"),
        ),
        text("Tap or drag").padding().gesture(
            TapGesture::new().simultaneously_with(DragGesture::new(8.0)),
            || waterui::log::info!("tap or drag"),
        ),
    ))
    .state(&status)
}
```

An exclusive pair does not make the second gesture wait for the first to fail.
`TapGesture::repeat(2).exclusively_before(TapGesture::new())` runs on the
first tap of a double tap, because the single tap is recognized first.

There is deliberately no `.simultaneous_gesture(..)` or
`.high_priority_gesture(..)` modifier. Compose the descriptors explicitly when
precedence matters.

## Hover

`.on_hover_enter(handler)` and `.on_hover_exit(handler)` fire as a pointer
enters and leaves the view's bounds. They apply where a pointer exists, such
as a mouse or a trackpad; touch input produces no hover. Hover may therefore
*enhance* an interaction but must never be the only way to reach it.

Drive the visual from state rather than rebuilding the view: stack both looks
and cross-fade with complementary opacity signals. `.select(a, b)` on a `bool`
signal picks `a` when true and `b` otherwise. `.animated()` rides the
platform's default animation on a signal, and `.with(Animation)` rides one you
choose. See [Animation](01-animation.md).

```rust
use waterui::animation::Animation;
use waterui::prelude::*;

fn hover_card() -> impl View {
    let hovered: Binding<bool> = binding(false);
    let scale = hovered.select(1.05, 1.0).with(Animation::spring(400.0, 15.0));

    zstack((
        Color::srgb(33, 150, 243)
            .with_opacity(0.3)
            .opacity(hovered.select(0.0, 1.0).animated()),
        Color::srgb(76, 175, 80)
            .with_opacity(0.5)
            .opacity(hovered.select(1.0, 0.0).animated()),
        text("Hover me").padding(),
    ))
    .width(200.0)
    .height(80.0)
    .on_hover_enter(|State(hovered): State<Binding<bool>>| hovered.set(true))
    .on_hover_exit(|State(hovered): State<Binding<bool>>| hovered.set(false))
    .state(&hovered)
    .scale(scale.clone(), scale)
}
```

## Pointer cursor

`.cursor(style)` takes `impl IntoComputed<CursorStyle>`: a plain style or a
signal of one. The style applies within the view's bounds and reverts to the
parent's cursor or the system default on exit.

```rust
use waterui::cursor::CursorStyle;
use waterui::gesture::{DragEvent, DragGesture, GesturePhase};
use waterui::prelude::*;

fn cursors() -> impl View {
    let dragging: Binding<bool> = binding(false);

    vstack((
        text("Open the docs").cursor(CursorStyle::PointingHand),
        text("Drag handle")
            .cursor(dragging.select(CursorStyle::ClosedHand, CursorStyle::OpenHand))
            .gesture(
                DragGesture::new(0.0),
                |Use(event): Use<DragEvent>, State(dragging): State<Binding<bool>>| {
                    dragging.set(matches!(
                        event.phase(),
                        GesturePhase::Started | GesturePhase::Updated
                    ));
                },
            )
            .state(&dragging),
    ))
}
```

`CursorStyle` has `Arrow` (the default), `PointingHand`, `IBeam`, `Crosshair`,
`OpenHand`, `ClosedHand`, `NotAllowed`, `ResizeLeft`, `ResizeRight`,
`ResizeUp`, `ResizeDown`, `ResizeLeftRight`, `ResizeUpDown`, `Move`, `Wait`,
and `Copy`. The enum is `#[non_exhaustive]`. Per the `Cursor` documentation, a
style a platform lacks typically falls back to the arrow, and cursors exist
only where there is a pointer, so they are another enhancement rather than a
requirement.

## Reactive pressed and focus visuals

`.on_hover_enter` reports the pointer of *that* view. When chrome lives around
a control it does not own (an outline, an offset, a shadow), ask the backend
for the whole interaction state instead. `.interaction_state(&binding)` writes
the `InteractionState` of the outermost interactive control at or inside the
view it is applied to, every time it changes. The binding stays at
`InteractionState::empty()` while no interactive control is there.

`InteractionState` is a bitflags set: `HOVERED`, `FOCUSED` (focus that should
be shown, like `:focus-visible`: keyboard focus, never a click), `PRESSED`,
`DRAGGED`, `SELECTED`, and `DISABLED`. States combine, so test with
`.contains(..)` and never `==`.

```rust
use waterui::interaction::InteractionState;
use waterui::prelude::*;

fn pressable_chip() -> impl View {
    let state: Binding<InteractionState> = binding(InteractionState::empty());
    let pressed = state.map(|s: InteractionState| s.contains(InteractionState::PRESSED));
    // The whole chip sinks two pixels while its button is pressed.
    let sink = pressed.select(2.0_f32, 0.0_f32);

    vstack((
        text("Filter"),
        button("Apply").action(|| waterui::log::info!("applied")),
    ))
    .offset(0.0, sink)
    .interaction_state(&state)
}
```

Theme styles describe per-state values with `StateValue<T>`:
`StateValue::new(resting).when(InteractionState::HOVERED, hovered_value)`, where
the order of `.when(..)` calls is their precedence and `.resolve(state)`
returns the first override whose states are all present. `.selected(signal)`
marks the control it modifies as selected, adding `SELECTED` to its state.

## Haptic feedback

Haptic modifiers require the `std` facade feature, which is not a default
feature:

```toml
waterui = { version = "0.5", features = ["std"] }
```

With it enabled, `waterui::Intensity` (re-exported from `waterkit_haptic`) has
the constants `LOW`, `MEDIUM`, `HIGH`, and `MAX`. The modifiers fire an impact
before running your handler:

```rust,ignore
use waterui::Intensity;
use waterui::prelude::*;

fn save_button() -> impl View {
    vstack((
        text("Save").on_tap_haptic_default(|| waterui::log::info!("saved")),
        text("Delete").on_tap_haptic(Intensity::HIGH, || waterui::log::info!("deleted")),
        text("Hold to arm").on_long_press_haptic(600, Intensity::MAX, || {
            waterui::log::info!("armed");
        }),
    ))
}
```

`.on_tap_haptic_default(action)` uses `Intensity::MEDIUM`, and
`.on_long_press_haptic_default(ms, action)` is the long-press equivalent.

## Where next

Gestures recognize input on one view. [Drag and drop](10-drag-drop.md) moves
typed data between views and applications, and
[Keyboard and focus](09-keyboard-focus.md) covers key handling. For how
handler parameters such as `State<T>` and `Use<T>` are extracted, read
[Controls](../03-ui/03-controls.md) and
[the environment chapter](../02-core/03-environment.md).

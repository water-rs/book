# Gestures and Haptics

Many interactions start with a gesture—tap, drag, long press—and finish with tactile feedback. The
`waterui::gesture` module lets you describe platform-agnostic gestures, observe them, and react with
handlers that can trigger animations, network work, or custom haptics.

## Observing Gestures

Attach `GestureObserver` to any view via `ViewExt::event` or by wrapping the view in helper widgets.
The observer describes the gesture to track plus a handler that executes when it fires.

```rust
use waterui::prelude::*;
use waterui::gesture::{Gesture, GestureObserver, TapGesture};

pub fn tappable_card() -> impl View {
    let gesture = Gesture::from(TapGesture::new());

    text("Favorite")
        .padding()
        .metadata(GestureObserver::new(gesture, || println!("Tapped!")))
}
```

Gesture handlers support extractors just like buttons. For example, extract the `TapEvent` payload to
read the tap location:

```rust
use waterui::core::extract::Use;
use waterui::gesture::TapEvent;

GestureObserver::new(TapGesture::new(), |Use(event): Use<TapEvent>| {
    println!("Tapped at {}, {}", event.location.x, event.location.y);
})
```

## Combining Gestures

Gestures compose via `.then(...)`. The following snippet waits for a successful long press before
enabling drag updates:

```rust
use waterui::gesture::{DragGesture, Gesture, LongPressGesture};

let gesture = LongPressGesture::new(500)
    .then(Gesture::from(DragGesture::new(5.0)));
```

Backends recognise the combined structure and only feed drag events once the long press completes.

## Drag, Magnification, and Rotation

Drag-related gestures surface `DragEvent` payloads. Magnification (`pinch-to-zoom`) and rotation
behave similarly with `MagnificationEvent` / `RotationGesture`.

```rust
use waterui::core::extract::Use;
use waterui::gesture::{DragEvent, DragGesture, GesturePhase};

GestureObserver::new(DragGesture::new(5.0), |Use(event): Use<DragEvent>| {
    match event.phase {
        GesturePhase::Started => println!("Drag started"),
        GesturePhase::Updated => println!("Translation {:?}", event.translation),
        GesturePhase::Ended => println!("Released"),
        GesturePhase::Cancelled => println!("Cancelled"),
    }
})
```

Store the translation in a binding to build sortable lists, draggable cards, or zoomable canvases.

## Integrating Haptics

WaterUI deliberately keeps haptic APIs in user space so you can tailor feedback per platform. Expose
a `Haptics` service through the environment and trigger it inside gesture handlers:

```rust
use waterui::env::Environment;
use waterui::core::extract::Use;

pub trait Haptics: Clone + 'static {
    fn impact(&self, style: ImpactStyle);
}

#[derive(Clone)]
struct ImpactStyle;

pub fn haptic_button() -> impl View {
    button("Favorite")
        .action(|Use(haptics): Use<impl Haptics>| {
            haptics.impact(ImpactStyle);
        })
}
```

Install a platform-specific implementation (e.g., using UIKit’s `UIImpactFeedbackGenerator` or
Android’s `Vibrator`) near your entry point:

```rust
Environment::new().with(MyHaptics::default());
```

Because gesture observers share the same extractor system, you can fire haptics when a drag completes
or a long press begins without additional glue.

## Best Practices

- Keep gesture handlers pure and fast—hand off async work to `task`.
- Use `.then` to avoid gesture conflicts (e.g., drag vs. tap) so backends receive deterministic
  instructions.
- Provide fallbacks for platforms that lack certain gestures; wrap gesture-sensitive views in
  conditionals that present alternative affordances.
- Treat haptics as optional. If no provider is registered, default to no-op implementations rather
  than panicking.

With declarative gestures and environment-driven haptics you can build nuanced, platform-appropriate
interaction models without sacrificing portability.

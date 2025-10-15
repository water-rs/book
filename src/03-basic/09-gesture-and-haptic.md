# Gesture and haptic

WaterUI's gesture system lives in the `waterui::gesture` module. Gestures describe high-level input
patterns—taps, long presses, drags, magnification, and rotation—and let you attach handler closures
through the `ViewExt::gesture` and `ViewExt::on_tap` helpers.

## Gesture descriptors

Each gesture variant captures the minimum configuration the backend needs to install recognisers:

- `TapGesture` — configure tap counts via `TapGesture::repeat` or rely on the single-tap `Default`.
- `LongPressGesture` — specify the minimum press duration before the handler runs.
- `DragGesture` — set a `min_distance` threshold to avoid accidental drags.
- `MagnificationGesture` — start zoom interactions from an `initial_scale` value.
- `RotationGesture` — initialise rotation with an `initial_angle` in radians.

Wrap these descriptors in the `Gesture` enum when you call `ViewExt::gesture`. Backends mirror the
interaction by storing a payload (for example `TapEvent`, `DragEvent`, or `MagnificationEvent`) in
the environment so your handler can read additional context.

```rust,ignore
use waterui::gesture::{Gesture, LongPressEvent, LongPressGesture, TapGesture};
use waterui::prelude::*;
use waterui_core::extract::{Extractor, Use};

fn attach_gestures() -> impl View {
    text!("Hold to favourite")
        .gesture(LongPressGesture::new(450), |event: LongPressEvent| {
            println!("Pressed at {:?} for {:?}", event.location, event.duration);
        })
        .gesture(Gesture::Tap(TapGesture::repeat(2)), |Use(profile): Use<UserProfile>| {
            profile.toggle_favourite();
        })
}

#[derive(Clone)]
struct UserProfile;

impl UserProfile {
    fn toggle_favourite(&self) {
        println!("Favourite toggled");
    }
}
```

The handler receives whatever you specify in the closure signature. Passing `Use<T>` triggers the
extractor pipeline, cloning `T` from the environment. Other payloads—such as `LongPressEvent` in the
example above—are provided directly by the backend.

## Inspecting gesture payloads manually

If you need access to the raw environment rather than binding `Use<T>` in the signature, request the
`Environment` and extract the payload yourself.

```rust,ignore
use waterui::gesture::{DragEvent, DragGesture, GesturePhase};
use waterui::prelude::*;
use waterui_core::extract::{Extractor, Use};

fn draggable_avatar() -> impl View {
    text!("🙂")
        .font_size(48.0)
        .gesture(DragGesture::new(5.0), |env: Environment| {
            if let Ok(Use(event)) = Extractor::extract::<Use<DragEvent>>(&env) {
                if event.phase == GesturePhase::Updated {
                    println!("translation = {:?}", event.translation);
                }
            }
        })
}
```

`DragEvent` is stored in the environment for the duration of the gesture. Because the extractor
returns `Result<Use<_>, Error>`, you can gracefully handle platforms that do not supply a specific
payload.

## Triggering haptics

WaterUI does not hard-code vibration APIs. Instead, install a haptic service in the environment and
use `Use<T>` inside gesture handlers.

```rust,ignore
use waterui::gesture::TapGesture;
use waterui::prelude::*;
use waterui::task::spawn_local;
use waterui_core::extract::Use;

#[derive(Clone)]
struct Haptics;

impl Haptics {
    async fn impact(&self) {
        println!("trigger haptic feedback");
    }
}

fn haptic_button() -> impl View {
    button("Capture photo")
        .on_tap(|Use(haptics): Use<Haptics>| {
            spawn_local(async move {
                haptics.impact().await;
            });
        })
        .with(Haptics)
}
```

The handler receives the `Haptics` service through the extractor pipeline and then uses
`spawn_local` to kick work onto the configured executor. Each backend is free to translate the
`impact` method to its native vibration APIs, keeping application code platform-agnostic.

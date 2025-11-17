# Canvas

WaterUI does not yet ship a first-party immediate-mode canvas, but you can integrate custom
renderers today using the same primitives the framework uses internally (`Native`, `Metadata`, and
platform hooks). This chapter outlines the approach so you can render charts, maps, or drawing
surfaces until the dedicated `waterui_canvas` crate lands.

## Wrap a Native View

Backends expose a `Native<T>` view that hands control to the renderer. Create a minimal wrapper that
stores your model and drive it with reactive bindings:

```rust
use waterui::prelude::*;
use waterui::reactive::binding;
use waterui::widget::suspense::Suspense;
use waterui_core::{Environment, Native, View};
use waterui::Binding;

pub struct Chart {
    pub points: Vec<(f32, f32)>,
}

impl View for Chart {
    fn body(self, _env: &Environment) -> impl View {
        Native(self)
    }
}

async fn stream_points(points: Binding<Vec<(f32, f32)>>) -> impl View {
    // Replace with actual async updates (WebSocket, sensor data, etc.)
    Chart { points: points.get() }
}

pub fn realtime_chart() -> impl View {
    let points = binding(vec![(0.0, 0.0)]);
    Suspense::new(stream_points(points.clone()));
    Chart { points: points.get() }
}
```

Each backend implements the trait that turns `Native<Chart>` into the appropriate drawing surface
(SwiftUI `Canvas`, HTML `<canvas>`, GTK4 `DrawingArea`, etc.). The binding keeps the view reactive,
and `Suspense` handles asynchronous updates.

## Pointer Input

Combine `GestureObserver` with your native view to capture pointer coordinates, then forward them to
the backing renderer via bindings or environment services.

## Looking Ahead

The upcoming `waterui_canvas` crate will package these patterns with a cross-platform immediate-mode
API (paths, fills, gradients) plus hit-testing utilities. Until then, native wrappers provide a
bridge for teams that need advanced drawing today.

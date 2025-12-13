# Canvas

WaterUI ships with a first-party immediate-mode canvas API via `waterui::graphics::Canvas`. It
uses Vello for high-performance GPU rendering of 2D vector graphics.

## Basic Usage

The canvas API provides a drawing context that lets you stroke and fill shapes:

```rust
use waterui::prelude::*;
use waterui::graphics::Canvas;
use waterui::graphics::kurbo::{Circle, Rect, Point};
use waterui::graphics::peniko::Color;

fn custom_drawing() -> impl View {
    Canvas::new(|ctx| {
        let center = ctx.center();
        
        // Fill a circle
        ctx.fill(
            Circle::new(center, 50.0),
            Color::RED,
        );

        // Stroke a rectangle
        ctx.stroke(
            Rect::from_origin_size(Point::new(10.0, 10.0), (100.0, 50.0)),
            Color::BLUE,
            2.0,
        );
    })
    .height(200.0) // Constrain height if needed
}
```

The drawing closure runs every frame (or when the view needs redrawing). `waterui::graphics`
re-exports types from `kurbo` (geometry) and `peniko` (colors/brushes).

## Reactive Drawing

To make the canvas reactive, capture bindings in the closure or use `Dynamic::watch` to
rebuild the canvas when state changes.

```rust
use waterui::prelude::*;
use waterui::graphics::Canvas;
use waterui::graphics::kurbo::Circle;
use waterui::graphics::peniko::Color;

fn reactive_circle(radius: Binding<f64>) -> impl View {
    // Rebuild the canvas when radius changes
    Dynamic::watch(radius, |r| {
        Canvas::new(move |ctx| {
            ctx.fill(
                Circle::new(ctx.center(), r),
                Color::GREEN,
            );
        })
    })
}
```

## Advanced Rendering

For more complex needs, `waterui::graphics::GpuSurface` allows direct access to `wgpu`
for custom render pipelines, while `ShaderSurface` simplifies using WGSL fragment shaders.
See the Shaders chapter for details.
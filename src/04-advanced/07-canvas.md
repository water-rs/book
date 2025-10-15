# Canvas rendering

WaterUI includes a vector-graphics canvas backed by `tiny-skia`. The canvas records drawing commands
into a `GraphicsContext` and renders them into a `RendererView`, allowing backends to present the
result on either CPU or GPU surfaces.

## Drawing paths

The `canvas` helper wraps a closure that receives a mutable `GraphicsContext`. Use `PathBuilder` to
construct shapes and submit them with `DrawStyle::Fill` or `DrawStyle::Stroke`.

```rust,ignore
use waterui::prelude::*;
use waterui::component::graphics::{DrawStyle, GraphicsContext, PathBuilder, canvas};

fn water_drop() -> impl View {
    canvas(|ctx: &mut GraphicsContext| {
        let mut path = PathBuilder::new();
        path.move_to(100.0, 20.0);
        path.quad_to(60.0, 120.0, 100.0, 160.0);
        path.quad_to(140.0, 120.0, 100.0, 20.0);

        ctx.draw_path(path.finish().unwrap(), DrawStyle::Fill(Color::from_rgb(0.2, 0.5, 0.9)));
    })
    .width(200.0)
    .height(200.0)
}
```

The closure runs every frame the backend needs to repaint. Because the context resolves colors
through the environment, you can layer WaterUI's theming system on top of canvas primitives.

## Reactive drawing

Combine canvas commands with bindings to animate vector art.

```rust,ignore
fn spinner(progress: Binding<f32>) -> impl View {
    canvas(move |ctx| {
        let angle = progress.get() * core::f32::consts::TAU;
        let mut path = PathBuilder::new();
        path.move_to(50.0, 50.0);
        path.arc(50.0, 50.0, 40.0, 0.0, angle);
        ctx.draw_path(path.finish().unwrap(), DrawStyle::Stroke(Color::from_rgb(0.9, 0.9, 0.9), 8.0));
    })
    .with(progress.with_animation(Animation::linear(core::time::Duration::from_secs(1))))
}
```

The animation metadata on the binding ensures the spinner advances smoothly without manual timer
management.

## Integrating with GPU surfaces

The canvas always renders into a CPU buffer. When a backend only exposes a GPU surface, WaterUI clears
it to transparent so you can composite the resulting texture with other layers. If you need custom GPU
behaviour, hand the surface to `RendererView` directly and issue commands using your graphics API of
choice.

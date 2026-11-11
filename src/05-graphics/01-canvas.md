# Canvas drawing

> **In this chapter, you will:**
> - Record fills, strokes, shadows, images, and text on a `Canvas`
> - Bind any operand to a signal so a change updates one command, not the whole drawing
> - Scope state with `clip`, `transform`, and `layer` closures instead of save and restore
> - Re-record part of a canvas with `scope` and `each`, and reuse constant artwork as a `Drawing`
> - Size a canvas and give it an accessible name, then build a live clock

`Canvas` is WaterUI's 2D vector drawing view. It is not an immediate-mode surface: the closure you hand to `Canvas::new` records a retained list of commands, and whichever renderer draws the screen around it draws that recording (a Core Animation layer tree on Apple platforms, Hydrolysis's own engine elsewhere). The canvas itself has no engine, no `wgpu` handle, and no per-frame callback. If you need your own pipeline, use [`GpuContentView`](02-gpu-surface.md) instead.

The canvas lives in `waterui::canvas`, which requires the `canvas` feature of the `waterui` crate (it is not on by default):

```toml
[dependencies]
waterui = { version = "0.5", features = ["canvas"] }
```

## A first canvas

```rust
use waterui::canvas::{Canvas, Circle};
use waterui::layout::{Point, Rect, Size};
use waterui::prelude::*;

fn badge() -> impl View {
    Canvas::new(|cx| {
        cx.fill(
            Rect::from_size(Size::new(120.0, 120.0)),
            Color::srgb(32, 36, 48),
        );
        cx.fill(
            Circle::new(Point::new(60.0, 60.0), 36.0),
            Color::srgb(59, 130, 246),
        );
    })
    .intrinsic_size(Size::new(120.0, 120.0))
}
```

`Canvas::new` takes a `'static` `FnOnce(&mut Cx)`. `Cx` is the recording context: each method (`fill`, `stroke`, `shadow`, `image`, `text`, `draw`, and the scoping closures) appends one retained command. Coordinates are points, with the origin at the top-left corner of the canvas box.

Three properties define the model:

- **The closure runs once** per mounted instance. What it records lives until the canvas unmounts.
- **Every operand takes a value or a signal.** A `Rect`, a `Color`, a `StrokeStyle`, a `Binding<Circle>`, and `binding.map(...)` are all accepted in the same position (the operand type is `Operand<T>`).
- **A signal stays bound.** When it changes, that one operand of that one command updates in place, animated if the change carries an animation. Nothing re-records and your closure does not run again.

Because the closure runs once, never read a signal with `.get()` to feed a command. Pass the signal, or `signal.map(...)`, and let the recording bind to it. The [reactive chapter](../02-core/02-reactive.md) covers `Binding`, `Computed`, and `map`.

## Shapes

`fill` and `stroke` take a shape. The shapes live in `waterui::canvas`:

| Type | Constructor |
|------|-------------|
| `Rect` (from `waterui::layout`) | `Rect::new(origin, size)`, `Rect::from_size(size)` |
| `RoundedRect` | `RoundedRect::new(rect, radius)` |
| `Circle` | `Circle::new(center, radius)`, `Circle::inscribed(rect)` |
| `Ellipse` | `Ellipse::new(bounds)` |
| `Line` | `Line::new(start, end)` |
| `Path` | `Path::builder()` ... `.build()` |

A `Path` is built by chaining `PathBuilder` calls: `move_to`, `line_to`, `quad_to(control, to)`, `cubic_to(c1, c2, to)`, `arc(center, (rx, ry), start, sweep)`, `close`, and `fill_rule(FillRule::EvenOdd)`. Angles are radians and `sweep` is clockwise; an arc sweeps at most one full turn. A finished `Path` is immutable and cheap to clone.

```rust
use waterui::canvas::{Canvas, Path, StrokeStyle};
use waterui::layout::Point;
use waterui::prelude::*;

fn triangle() -> impl View {
    let outline = Path::builder()
        .move_to(Point::new(60.0, 10.0))
        .line_to(Point::new(110.0, 100.0))
        .line_to(Point::new(10.0, 100.0))
        .close()
        .build();

    Canvas::new(move |cx| {
        cx.fill(outline.clone(), Color::srgb(16, 185, 129));
        cx.stroke(outline, StrokeStyle::new(3.0), Color::srgb(6, 95, 70));
    })
}
```

`Path::contains(point)` and `Path::bounds()` answer hit-testing and layout questions without a canvas.

## Strokes

`stroke(shape, style, paint)` takes a `StrokeStyle`. `StrokeStyle::new(width)` is a solid stroke with butt caps and miter joins; chain methods to change that.

```rust
use waterui::canvas::{Cap, Canvas, Join, Line, StrokeStyle};
use waterui::layout::Point;
use waterui::prelude::*;

fn dashed_rule() -> impl View {
    Canvas::new(|cx| {
        cx.stroke(
            Line::new(Point::new(10.0, 20.0), Point::new(190.0, 20.0)),
            StrokeStyle::new(4.0)
                .cap(Cap::Round)
                .join(Join::Round)
                .dash(vec![8.0, 6.0], 0.0),
            Color::srgb(148, 163, 184),
        );
    })
}
```

`Cap` is `Butt`, `Round`, or `Square`; `Join` is `Miter`, `Round`, or `Bevel`; `miter_limit(limit)` bounds a miter join, and `dash(pattern, offset)` takes alternating dash and gap lengths.

## Paints

The paint operand of `fill` and `stroke` accepts a theme `Color`, a `WorkingColor`, a gradient, or an `ImagePattern`.

- A theme `Color` resolves in the canvas's environment and stays bound to it, so a canvas filled with a theme color follows light and dark mode without any code of yours. `cx.resolve(color)` gives you the same resolution as a `Computed<WorkingColor>` when you need to derive something from it.
- `WorkingColor` is the renderer's own space: linear Display P3 with straight alpha. `WorkingColor::BLACK`, `WHITE`, and `TRANSPARENT` are constants, and `.with_alpha(a)` replaces the alpha. A color such as `Srgb` converts into one with `.into()`.
- `LinearGradient::new(start, end)`, `RadialGradient::new(center, radius)` (or `two_point`), and `ConicGradient::new(center, start_angle, end_angle)` take their geometry in the constructor and their stops from `.stop(offset, color)`, where `color` is anything that converts into a `WorkingColor`; `.extend(Extend)` and `.interpolation(Interpolation)` choose edge and blending behavior. `MeshGradient::new(columns, rows, points, colors)` takes every vertex and its color at once; import it from `waterui::canvas`, because the prelude's `MeshGradient` is the gradient view.
- `Paint::transformed(transform)` places any paint through an `Affine2` without moving the shape it fills. Convert the paint first, for example `Paint::from(gradient).transformed(...)`.

```rust
use waterui::canvas::{Canvas, LinearGradient, RoundedRect, Shadow};
use waterui::color::{Srgb, WorkingColor};
use waterui::layout::{Point, Rect, Size};
use waterui::prelude::*;

fn card_background() -> impl View {
    let card = Rect::new(Point::new(10.0, 10.0), Size::new(180.0, 100.0));
    Canvas::new(move |cx| {
        cx.shadow(
            RoundedRect::new(card, 12.0),
            Shadow::new(6.0, WorkingColor::BLACK.with_alpha(0.4)).offset(0.0, 3.0),
        );
        cx.fill(
            RoundedRect::new(card, 12.0),
            LinearGradient::new(Point::new(10.0, 10.0), Point::new(190.0, 110.0))
                .stop(0.0, Srgb::new(0.36, 0.52, 0.98))
                .stop(1.0, Srgb::new(0.55, 0.30, 0.90)),
        );
    })
    .intrinsic_size(Size::new(200.0, 120.0))
}
```

`cx.shadow(shape, shadow)` is a command of its own: it casts a blurred shadow from a shape, and the shape's own fill is a separate `fill`. `Shadow::new(blur, color)` takes the blur's standard deviation in points and a `WorkingColor`; `.offset(dx, dy)` and `.spread(amount)` displace and grow it. Gradients as views, such as a background, are in [the gradients chapter](06-gradients.md).

## Clips, transforms, and layers

A canvas has no `save` or `restore`. State is scoped by a closure, and it ends where the closure ends.

```rust
use waterui::canvas::{Canvas, Circle, Layer, Line, StrokeStyle};
use waterui::layout::{Affine2, Point, Rect, Size};
use waterui::prelude::*;

fn scoped_state() -> impl View {
    Canvas::new(|cx| {
        // Everything inside is clipped to the rectangle.
        cx.clip(
            Rect::new(Point::new(10.0, 10.0), Size::new(100.0, 100.0)),
            |cx| {
                cx.fill(Circle::new(Point::new(60.0, 60.0), 80.0), Color::srgb(239, 68, 68));
            },
        );

        // Everything inside is moved and rotated together.
        cx.transform(Affine2::translate(160.0, 60.0), |cx| {
            cx.transform(Affine2::rotate(0.5), |cx| {
                cx.stroke(
                    Line::new(Point::new(-30.0, 0.0), Point::new(30.0, 0.0)),
                    StrokeStyle::new(4.0),
                    Color::srgb(37, 99, 235),
                );
            });
        });

        // The layer composites its content as one image at half opacity.
        cx.layer(Layer::opacity(0.5), |cx| {
            cx.fill(Circle::new(Point::new(260.0, 40.0), 30.0), Color::srgb(16, 185, 129));
            cx.fill(Circle::new(Point::new(280.0, 60.0), 30.0), Color::srgb(245, 158, 11));
        });
    })
    .intrinsic_size(Size::new(320.0, 120.0))
}
```

- `clip(shape, body)` draws `body` clipped to any shape, including a `Path`.
- `transform(affine, body)` takes an `Affine2`: `translate(tx, ty)`, `scale(sx, sy)`, `rotate(radians)`, or a hand-built matrix. Nested transforms compose.
- `layer(layer, body)` isolates `body`, then composites it as one image. `Layer::opacity(o)` sets the opacity; `.blend(BlendMode)` and `.blend_space(BlendSpace)` change how it blends onto what lies beneath. In the overlapping circles above, the layer fades the pair as a unit, so the overlap does not show through.

A scope's own operand (the clip shape, the transform, the layer) takes a signal too, so `cx.transform(angle.map(Affine2::rotate), |cx| ...)` rotates its whole body when `angle` changes, without re-recording a single command inside it.

## Images

An image is described, never registered. You build an `ImageSource` from pixels; the target that draws the canvas registers it on first use, holds it while the canvas draws it, and registers it again if its engine is replaced. Cloning an `ImageSource` shares the pixels, and every clone is the same image to the target.

```rust
use waterui::canvas::{Canvas, ImageSource, PixelBuffer, PixelFormat, Sampling};
use waterui::graphics::backend::ResourceError;
use waterui::layout::{Rect, Size};
use waterui::prelude::*;

/// A 4x4 checkerboard of straight-alpha sRGB pixels.
fn checkerboard() -> Result<ImageSource, ResourceError> {
    let pixels: Vec<u8> = (0..16u32)
        .flat_map(|i| {
            let light = (i % 4 + i / 4) % 2 == 0;
            let value = if light { 255 } else { 40 };
            [value, value, value, 255]
        })
        .collect();
    ImageSource::new(PixelBuffer::new(4, 4, PixelFormat::Rgba8, pixels))
}

fn pixel_art(image: ImageSource) -> impl View {
    Canvas::new(move |cx| {
        cx.image(
            image,
            Rect::from_size(Size::new(128.0, 128.0)),
            Sampling::Nearest,
        );
    })
    .intrinsic_size(Size::new(128.0, 128.0))
}
```

`ImageSource::new` fails with a `ResourceError` when a dimension is zero or the buffer length does not match. `PixelBuffer::new` assumes straight-alpha sRGB; its public fields (`color_space`, `premultiplied`) cover the other cases, and `PixelFormat::Rgba16F` carries wide-gamut and HDR data. `Sampling::Nearest` keeps hard pixel edges; `Sampling::Linear` is the default.

The image operand is a signal like any other, and passing a signal of images swaps the picture. What the target made of an image is `ImageSource::status()`, a `Computed<ImageStatus>` that reads `Pending` until a target first draws it, then `Ready`, or `Refused` when the image is too large for the device, over the target's image budget, or unreadable. A refused image draws nothing. An `ImagePattern::new(image)` tiles an image as a paint, with `.transform(...)`, `.extend(x, y)`, and `.sampling(...)` to place it.

`ImageSource` is for pixels you already hold or compute. For bundled files, see [the assets chapter](../04-rich/05-assets.md).

## Text

`cx.text(styled, placement)` draws text that the target lays out with the same text stack every other `Text` view uses, in the canvas's environment: the same fonts, locale, and theme colors. The canvas never measures it. A `TextPlacement` names where the text goes instead:

- `TextPlacement::at(origin)` anchors the text, leading-aligned and unwrapped.
- `.aligned(HorizontalAlignment::Center)` aligns the lines and also decides which edge of the text box sits at the origin: leading, center, or trailing.
- `.width(points)` breaks lines at that width.

The box is `width` wide, or as wide as its longest line when you give no width, and its top is at the origin's `y`.

```rust
use waterui::canvas::{Canvas, TextPlacement};
use waterui::layout::{HorizontalAlignment, Point, Size};
use waterui::prelude::*;
use waterui::text::StyledStr;

fn captions() -> impl View {
    Canvas::new(|cx| {
        let centred = |y: f32| {
            TextPlacement::at(Point::new(100.0, y))
                .aligned(HorizontalAlignment::Center)
                .width(180.0)
        };
        cx.text(StyledStr::from("Time"), centred(10.0));
        cx.text(StyledStr::from("时间 / 時間 / 시간"), centred(36.0));
    })
    .intrinsic_size(Size::new(200.0, 64.0))
}
```

Both operands take signals. A new text, or a placement whose alignment or width changed, lays out again; a placement whose origin alone changed just moves the laid-out text, animated when the change carries an animation. Text whose box layout must know before it runs, such as a label that sizes a button, stays a `Text` view placed over the canvas; see [the Text chapter](../03-ui/01-text.md).

For glyphs you position yourself, `cx.glyphs(GlyphRun::new(font, size, glyphs), paint)` draws a run exactly as given, in a `Font::system(family)` or a `Font::from_bytes(data, index)`.

## Reusable drawings

A `Drawing` is a recording of constants that a canvas draws any number of times. It is `Send` and `Sync`, so a large document can be parsed and recorded off the main thread, and cloning shares the recording. You record it through a `DrawingCx`, whose commands mirror the shape, paint and scope commands of `Cx` but take constants: a shape, any paint that converts into a `Paint` (a `WorkingColor`, a gradient, an `ImagePattern`, but not a theme `Color`), a `&StrokeStyle`, a `&ImageSource` and a `&GlyphRun`. A `DrawingCx` has no `text`, `scope`, `each`, `keep`, clock or size.

```rust
use waterui::canvas::{Canvas, Circle, Drawing, StrokeStyle};
use waterui::color::{Srgb, WorkingColor};
use waterui::layout::{Affine2, Point, Size};
use waterui::prelude::*;

fn star_mark() -> Drawing {
    Drawing::record(Size::new(24.0, 24.0), |cx| {
        cx.fill(
            Circle::new(Point::new(12.0, 12.0), 10.0),
            WorkingColor::from(Srgb::new(0.96, 0.62, 0.04)),
        );
        cx.stroke(
            Circle::new(Point::new(12.0, 12.0), 10.0),
            &StrokeStyle::new(2.0),
            WorkingColor::BLACK,
        );
    })
}

fn row_of_marks() -> impl View {
    let mark = star_mark();
    Canvas::new(move |cx| {
        for index in 0..5 {
            cx.draw(&mark, Affine2::translate(30.0 * index as f32, 0.0));
        }
    })
    .intrinsic_size(Size::new(144.0, 24.0))
}
```

`Drawing::record` panics on a size that is not finite and positive. `Canvas::from(drawing)` shows a drawing as a view of its own size, scaled to fill whatever box layout gives it.

## Changing structure: scope and each

Operand signals cover values that change. When the structure itself depends on a value, a command appears or disappears, two tools re-record locally and nothing else records again.

`cx.scope(signal, body)` records `body` for the current value and records it again, that region only, whenever the signal changes.

```rust
use waterui::canvas::{Canvas, Line, StrokeStyle};
use waterui::layout::{Point, Size};
use waterui::prelude::*;

fn optional_grid(show_grid: Binding<bool>) -> impl View {
    Canvas::new(move |cx| {
        cx.scope(show_grid, |cx, on: &bool| {
            if *on {
                for step in 0..=4 {
                    let x = 25.0 * step as f32;
                    cx.stroke(
                        Line::new(Point::new(x, 0.0), Point::new(x, 100.0)),
                        StrokeStyle::new(1.0),
                        Color::srgb(148, 163, 184),
                    );
                }
            }
        });
    })
    .intrinsic_size(Size::new(100.0, 100.0))
}
```

`cx.each(items, key, body)` records `body` once per item of a `Vec`, keyed by `key`. When the collection changes, a new key records, a removed key drops, and moved items reorder, without re-recording the rest. An item whose value changes under the same key updates through the `Computed<T>` its body received. Two items with the same key panic.

```rust
use waterui::canvas::Canvas;
use waterui::layout::{Point, Rect, Size};
use waterui::prelude::*;

#[derive(Clone, PartialEq)]
struct Bar {
    id: u32,
    height: f32,
}

fn bar_chart(bars: Binding<Vec<Bar>>) -> impl View {
    Canvas::new(move |cx| {
        cx.each(
            bars,
            |bar: &Bar| bar.id,
            |cx, bar| {
                let rect = bar.map(|bar: Bar| {
                    Rect::new(
                        Point::new(bar.id as f32 * 20.0, 100.0 - bar.height),
                        Size::new(16.0, bar.height),
                    )
                });
                cx.fill(rect, Color::srgb(59, 130, 246));
            },
        );
    })
    .intrinsic_size(Size::new(200.0, 100.0))
}
```

Use `cx.keep(value)` to tie a value to the recording's lifetime. It is dropped when the canvas unmounts, when the enclosing `scope` re-records, or when the `each` item is removed. A hardware source opened for a drawing closes then.

## Time

`cx.time()` is a `Computed<Duration>`: the time since the canvas mounted. It advances once per presented frame, and only while something observes it. A live operand derived from it counts, as does a `scope` over it. A canvas that never reads the clock presents no extra frames. `cx.time_until(end)` is the same clock held at `end`. A clock that has reached its end no longer counts as observed, so a canvas whose only clock readers have all reached their ends presents no more frames.

There is no `request_next_frame`. To animate, derive an operand from `cx.time()` or from a `Binding` you update.

## Size, layout, and accessibility

A canvas with no `intrinsic_size` stretches on both axes and draws into whatever box layout gives it. `cx.size()` returns that box as a `Computed<Size>`, which follows layout, so draw against it with `map`. The closure runs at mount, before the first layout pass, so the size signal reads zero until layout places the canvas and then updates.

`intrinsic_size(size)` says what the drawing naturally is: an image's pixel size, a typeset formula's box. With a size, the canvas is content-sized and keeps its aspect ratio when a container names only one axis. The size is itself a signal (`impl IntoComputed<Option<Size>>`), and layout measures again when it changes. Layout panics on a size that is not finite and positive on both axes.

To share the box with code outside the drawing, create a `CanvasSize` first, read its `signal()`, and hand it to the canvas with `Canvas::sized_by(size)`. One `CanvasSize` belongs to one mounted canvas at a time.

A drawing says nothing to a screen reader by itself. Name it:

```rust
use waterui::canvas::{Canvas, Circle};
use waterui::layout::{Point, Size};
use waterui::prelude::*;

fn labelled_dot() -> impl View {
    Canvas::new(|cx| {
        cx.fill(Circle::new(Point::new(20.0, 20.0), 16.0), Color::srgb(239, 68, 68));
    })
    .intrinsic_size(Size::new(40.0, 40.0))
    .offered_label(Str::from("Recording indicator"))
    .offered_value(Str::from("On"))
}
```

`offered_label` and `offered_value` are what the drawing itself offers, such as an SVG's title or a formula's spoken mathematics. An `.a11y_label(...)` or `.a11y_value(...)` that the app puts on the view overrides them. See [the accessibility chapter](../06-advanced/05-accessibility.md).

## A clock face

The face and the ticks are recorded once, inside a transform bound to `cx.size()` that keeps them at the center of the box. The two hands are recorded once too, each under a transform bound to `cx.time()`.

```rust
use core::f32::consts::TAU;
use core::time::Duration;

use waterui::canvas::{Cap, Canvas, Circle, Line, StrokeStyle};
use waterui::layout::{Affine2, Point, Size};
use waterui::prelude::*;

fn clock() -> impl View {
    Canvas::new(|cx| {
        let centre = cx.size().map(|size: Size| {
            Affine2::translate(size.width / 2.0, size.height / 2.0)
        });
        let elapsed = cx.time();

        cx.transform(centre, |cx| {
            let face = Circle::new(Point::new(0.0, 0.0), 90.0);
            cx.fill(face, Color::srgb(24, 26, 34));
            cx.stroke(face, StrokeStyle::new(3.0), Color::srgb(200, 200, 210));

            for tick in 0..12 {
                cx.transform(Affine2::rotate(tick as f32 * TAU / 12.0), |cx| {
                    cx.stroke(
                        Line::new(Point::new(0.0, -80.0), Point::new(0.0, -88.0)),
                        StrokeStyle::new(2.0).cap(Cap::Round),
                        Color::srgb(200, 200, 210),
                    );
                });
            }

            // One turn a minute, advancing every presented frame.
            let second =
                elapsed.map(|time: Duration| Affine2::rotate(time.as_secs_f32() / 60.0 * TAU));
            cx.transform(second, |cx| {
                cx.stroke(
                    Line::new(Point::new(0.0, 8.0), Point::new(0.0, -78.0)),
                    StrokeStyle::new(2.0).cap(Cap::Round),
                    Color::srgb(239, 68, 68),
                );
            });

            // One turn an hour.
            let minute =
                elapsed.map(|time: Duration| Affine2::rotate(time.as_secs_f32() / 3600.0 * TAU));
            cx.transform(minute, |cx| {
                cx.stroke(
                    Line::new(Point::new(0.0, 0.0), Point::new(0.0, -60.0)),
                    StrokeStyle::new(4.0).cap(Cap::Round),
                    Color::srgb(240, 240, 245),
                );
            });
        });
    })
    .intrinsic_size(Size::new(200.0, 200.0))
    .offered_label(Str::from("Clock"))
}
```

The clock counts from the moment it mounts. To show wall-clock time, add the current offset into the day to `time` inside the `map`.

The closure ran once. Three operands change after that: the centering transform when layout resizes the box, and the two hands' transforms every frame.

## Next

When a canvas is not enough, such as when you need custom render pipelines, compute shaders, or instanced draws, continue to [GPU content with `GpuContentView`](02-gpu-surface.md). Shapes as views, filled and used as clips rather than drawn, are in [the styling chapter](../03-ui/11-styling.md).

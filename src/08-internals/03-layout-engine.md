# The layout engine

> **In this chapter, you will:**
>
> - Understand WaterUI's two-phase layout algorithm (propose, then place)
> - Learn how `ProposalSize` lets parents and children negotiate dimensions
> - See how `StretchAxis` controls how views fill available space
> - Measure children in parallel and cache measurements correctly
> - Write a custom layout from scratch

Parents propose sizes, children respond with their preferences, and parents make
the final placement decisions. That two-phase negotiation is the whole layout
system; everything else in this chapter is a consequence of it.

## Logical pixels

All layout values in WaterUI use **logical pixels** (also called "points" or "dp"),
the same unit system design tools use:

- **iOS/macOS**: 1 logical pixel = 1 UIKit/AppKit point (1-3 physical pixels)
- **Android**: 1 logical pixel = 1 dp (converted via `displayMetrics.density`)
- **GTK4**: 1 logical pixel = 1 CSS pixel (scaled by GDK)

A button at 44pt height with 16pt padding in Figma is
`.height(44.0).padding_with(16.0)` in WaterUI, with no conversion step.

## The `Layout` trait

`Layout` lives in `waterui_core::layout` and defines a container's algorithm:

```rust,ignore
pub trait Layout: Debug + Any {
    /// Phase 1: calculate the size this layout wants.
    fn size_that_fits(&self, proposal: ProposalSize, children: &[&dyn SubView]) -> Size;

    /// Phase 2: place children within the given bounds, one `Rect` per child.
    fn place(&self, bounds: Rect, children: &[&dyn SubView]) -> Vec<Rect>;

    /// Which axis this container stretches on.
    fn stretch_axis(&self) -> StretchAxis {
        StretchAxis::None
    }
}
```

Those are the two methods you must write. The trait also carries defaulted hooks
for alignment guides (`explicit_horizontal`, `explicit_vertical`, and their
`*_alignments` companions) and one `#[doc(hidden)]` hook, `watch_invalidation`,
covered under [reactive layout parameters](#reactive-layout-parameters).

The separation matters: during sizing you may probe children with several
different proposals to learn how flexible they are before committing to an
arrangement.

## `ProposalSize`

The parent states its intent through `ProposalSize`:

```rust,ignore
pub struct ProposalSize {
    pub width: Option<f32>,
    pub height: Option<f32>,
}
```

| Value                 | Meaning                             |
|-----------------------|-------------------------------------|
| `None`                | "Tell me your ideal/intrinsic size" |
| `Some(0.0)`           | "Tell me your minimum size"         |
| `Some(f32::INFINITY)` | "Tell me your maximum size"         |
| `Some(value)`         | "I suggest you use this size"       |

Three constants cover the common probes:

```rust,ignore
ProposalSize::UNSPECIFIED  // None, None
ProposalSize::ZERO         // Some(0.0), Some(0.0)
ProposalSize::INFINITY     // Some(INFINITY), Some(INFINITY)
```

A child is never obligated to accept a proposal. `Text` returns its intrinsic
size from the content and font no matter what is proposed.

## The `SubView` proxy

Containers never touch child views directly. They work through `SubView`:

```rust,ignore
pub trait SubView: Send + Sync {
    /// Measure the child for a given proposal. May be called repeatedly.
    fn measure(&self, proposal: ProposalSize) -> ViewDimensions;

    /// Which axis this child stretches on.
    fn stretch_axis(&self) -> StretchAxis;

    /// Layout priority for space distribution; higher wins.
    fn priority(&self) -> i32;

    /// Whether this child's measurement must run on the main thread.
    fn require_main_thread(&self) -> bool { false }
}
```

`measure` returns `ViewDimensions`, not a bare `Size`: the `size` field plus any
explicit alignment guides the child published. Reach for `.size` when guides do
not matter.

Three properties of this trait drive everything else:

- **Measurement is pure.** Every method takes `&self`. Calling `measure` five
  times with five proposals is legal and expected.
- **`SubView` is `Send + Sync`.** Layout may measure independent children on
  worker threads.
- **Priority orders space distribution.** Higher-priority children are measured
  first and claim space before flexible siblings such as spacers.

### Caching is the child's job

The `Layout` trait deliberately has no cache, because containers probe freely
and a container-level cache would have to guess which probes repeat. Caching
belongs to the `SubView` implementation, and expensive measures — text shaping
above all — **must** cache.

Because measurement can run off the main thread, that cache has to be
thread-safe: a lock or a lock-free map, never a `RefCell`.

A leaf whose measurement genuinely must stay on the main thread wraps its
non-`Send` state in `waterui_core::MainThreadBound<T>` (which satisfies
`Send + Sync` at the type level while asserting single-thread access at runtime)
and returns `true` from `require_main_thread`. Returning `false` while touching
main-thread-only state is a bug, and the `MainThreadBound` assertion fails fast
when it happens.

### Measuring children in parallel

`waterui_layout::measure_children` applies one measure closure across a child
slice and returns results in the original order:

```rust,ignore
use waterui_layout::measure_children;

let sizes = measure_children(children, |child| child.measure(proposal).size);
```

With the `parallel` feature enabled (off by default; it pulls in `rayon`),
children that report `require_main_thread() == false` are measured on a worker
pool while the rest are measured on the calling thread. Without the feature — on
`no_std` and embedded targets, for instance — the same call measures serially.
`HStack` and `VStack` already route their measurement through it.

## `StretchAxis`

Every view declares how it wants to fill available space:

```rust,ignore
pub enum StretchAxis {
    None,       // Content-sized
    Horizontal, // Expands width, intrinsic height
    Vertical,   // Intrinsic width, expands height
    Both,       // Greedy, fills all space
    MainAxis,   // Expands along the parent's main axis
    CrossAxis,  // Expands along the parent's cross axis
}
```

`MainAxis` and `CrossAxis` are resolved against the parent: in a `VStack`,
`MainAxis` is vertical; in an `HStack` it is horizontal. This is what lets
`Spacer` push siblings apart in either orientation and `Divider` span the cross
axis in either orientation.

## How the built-in layouts work

### `VStack` and `HStack`

**Sizing:**

1. Separate children into fixed (non-stretchy) and flexible (stretchy) groups.
2. Propose the available size to each fixed child and collect their measurements.
3. Compute the space remaining after fixed children and spacing.
4. Distribute the remainder among flexible children as equal shares.
5. Sum child extents along the main axis, plus spacing.

**Placement:** start at the top (`VStack`) or leading edge (`HStack`), advance by
child extent plus spacing, and align each child on the cross axis.

`VStack` reports `StretchAxis::Horizontal`: it fills available width and takes
its height from its children.

When an `HStack`'s children do not fit, the overflow is resolved by
**water-filling**, not by squeezing children in order. The stack finds the
largest common width cap `T` such that the sum of `min(width, T)` fits the
available space, clamps everything above the cap (with a 20pt floor), and
re-measures the clamped children at the cap so wrapped content reports its true
height. Equal-width children therefore shrink equally instead of the leading ones
absorbing the entire overflow.

### Frames

`.width(...)`, `.height(...)`, `.size(w, h)`, and the `min_*`/`max_*` family each
wrap the view in a `Frame`. The frame proposes the constrained size to its child
and reports the constrained dimensions upward.

### Grids

`GridLayout` arranges children into rows and columns; each column can be
fixed-width, flexible, or adaptive.

### `ScrollView`

`ScrollView` proposes an infinite extent along its scroll axis so content may
exceed the viewport. The backend owns the scrolling behavior itself.

### Padding

```rust,ignore
text("Padded").padding_with(EdgeInsets::all(16.0))
```

Sizing adds the insets to the child's size; placement offsets the child's origin
by the leading and top insets.

## Reactive layout parameters

Layout inputs are signals, not snapshots. `HStackLayout::spacing`,
`VStackLayout::spacing`, `GridLayout::spacing`, and every `FrameLayout`
dimension hold a `Computed` rather than a plain `f32`, so passing a binding
produces a real re-layout when it changes:

```rust,ignore
use waterui::prelude::*;
use waterui::layout::stack::hstack;

let gap = Binding::f64(8.0);

hstack((text("left"), text("right"))).spacing(gap.clone())
```

`.spacing(...)` accepts anything implementing `IntoSignalF32`, which covers
integer and float literals as well as signals.

The mechanism behind this is `Layout::watch_invalidation`. A layout returns
watcher guards for its own reactive fields; the native container holds those
guards for the layout object's lifetime and requests a new layout pass when one
fires. `HStackLayout`, for example, returns a single guard on its spacing signal.
The hook is `#[doc(hidden)]` backend infrastructure with a default empty
implementation — implement it only if your custom layout stores signals.

One consequence to know about: `HStack::new`, `hstack()`, `VStack::new`,
`vstack()`, and `GridLayout::new` are no longer `const fn`, because building a
`Computed` is not a const operation. `ZStack` and `zstack()` remain `const`.

## Writing a custom layout

Implement `Layout`. Here is a flow layout that wraps children to the next line
when they exceed the available width:

```rust,ignore
use waterui_core::layout::{Layout, Point, ProposalSize, Rect, Size, SubView};

#[derive(Debug)]
pub struct FlowLayout {
    pub h_spacing: f32,
    pub v_spacing: f32,
}

impl Layout for FlowLayout {
    fn size_that_fits(&self, proposal: ProposalSize, children: &[&dyn SubView]) -> Size {
        let max_width = proposal.width_or(f32::INFINITY);
        let mut x = 0.0_f32;
        let mut y = 0.0_f32;
        let mut row_height = 0.0_f32;
        let mut total_width = 0.0_f32;

        for child in children {
            let child_size = child.measure(ProposalSize::UNSPECIFIED).size;

            if x + child_size.width > max_width && x > 0.0 {
                y += row_height + self.v_spacing;
                x = 0.0;
                row_height = 0.0;
            }

            x += child_size.width + self.h_spacing;
            row_height = row_height.max(child_size.height);
            total_width = total_width.max(x - self.h_spacing);
        }

        Size::new(total_width, y + row_height)
    }

    fn place(&self, bounds: Rect, children: &[&dyn SubView]) -> Vec<Rect> {
        let max_width = bounds.width();
        let mut rects = Vec::with_capacity(children.len());
        let mut x = 0.0_f32;
        let mut y = 0.0_f32;
        let mut row_height = 0.0_f32;

        for child in children {
            let child_size = child.measure(ProposalSize::UNSPECIFIED).size;

            if x + child_size.width > max_width && x > 0.0 {
                y += row_height + self.v_spacing;
                x = 0.0;
                row_height = 0.0;
            }

            rects.push(Rect::new(
                Point::new(bounds.x() + x, bounds.y() + y),
                child_size,
            ));

            x += child_size.width + self.h_spacing;
            row_height = row_height.max(child_size.height);
        }

        rects
    }
}
```

Both phases measure with the same proposal, so the sizes computed in phase 1
match what phase 2 places. If the two disagree, children render at one size and
are positioned as if they had another.

> **Try it yourself:** arrange children in a circle. Use `size_that_fits` for the
> bounding box and `place` to position each child at an angle around the center.

## Safe area

Safe area handling is deliberately outside the `Layout` trait, because it is
platform state rather than geometry: notch and home indicator on iOS, navigation
bar and cutouts on Android, toolbar and title bar on macOS.

Backends apply safe-area insets themselves. A view opts out with
`.ignore_safe_area(EdgeSet)`, which attaches `IgnoreSafeArea` metadata telling
the backend to extend past the boundary on the listed edges:

```rust,ignore
use waterui::layout::safe_area::EdgeSet;

hero_image().ignore_safe_area(EdgeSet::TOP)
```

## Geometry types

| Type           | Fields                                    | Description                        |
|----------------|-------------------------------------------|------------------------------------|
| `Point`        | `x: f32, y: f32`                          | Position relative to parent origin |
| `Size`         | `width: f32, height: f32`                 | Two-dimensional extent             |
| `Rect`         | `origin: Point, size: Size`               | Positioned rectangle               |
| `ProposalSize` | `width: Option<f32>, height: Option<f32>` | Size negotiation                   |

`Rect` answers the usual geometric questions directly:

```rust,ignore
let rect = Rect::new(Point::new(10.0, 20.0), Size::new(100.0, 50.0));
rect.min_x();  // 10.0
rect.max_x();  // 110.0
rect.mid_x();  // 60.0
rect.center(); // Point(60.0, 45.0)
rect.inset(10.0, 10.0, 20.0, 20.0); // top, bottom, leading, trailing
```

## Layout and the FFI

`Layout` is Rust-only. Backends that delegate to a platform layout system — Apple
through UIKit/AppKit, Android through its view hierarchy — do not call it. They
read each view's stretch axis over the FFI through `waterui_view_stretch_axis()`
and let the host system position widgets.

The Rust trait is the source of truth for backends that lay out themselves:
Hydrolysis, Dew, and any custom backend built on `waterui-backend-core`.

## What's next

Layout decides where views go; backends decide what they are made of. The
[next chapter](04-backends.md) surveys the backend architecture.

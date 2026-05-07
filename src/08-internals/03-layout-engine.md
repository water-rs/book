# The Layout Engine

> **In this chapter, you will:**
>
> - Understand WaterUI's two-phase layout algorithm (propose, then place)
> - Learn how `ProposalSize` lets parents and children negotiate dimensions
> - See how `StretchAxis` controls how views fill available space
> - Write a custom layout from scratch

Every UI framework needs to answer one question: where does each element go on screen? WaterUI answers it with a two-phase layout algorithm inspired by SwiftUI's layout protocol. Parents propose sizes, children respond with their preferences, and then parents make the final placement decisions. If you have ever been frustrated by a view that refuses to fill available space (or one that greedily takes too much), understanding this system will give you the tools to fix it.

## Logical Pixels

Before diving into the layout algorithm, a quick note on units.

All layout values in WaterUI use **logical pixels** (also called "points" or "dp").
This is the same unit system used by design tools like Figma, Sketch, and Adobe XD:

- **iOS/macOS**: 1 logical pixel = 1 UIKit/AppKit point (1-3 physical pixels)
- **Android**: 1 logical pixel = 1 dp (converted via `displayMetrics.density`)
- **GTK4**: 1 logical pixel = 1 CSS pixel (scaled by GDK)

This means `spacing(8.0)` produces the same physical size on a 1x display, a 2x
Retina display, and a 3x mobile display. You can translate designs from Figma to
WaterUI using the exact same numbers.

> **Tip:** If your designer hands you a Figma file with a button at 44pt height and 16pt padding, you can use those exact values in WaterUI: `.frame(height: 44.0).padding(16.0)`.

## The Layout Trait

The `Layout` trait defines a container's layout algorithm:

```rust,ignore
pub trait Layout: Debug {
    /// Phase 1: Calculate the size this layout wants.
    fn size_that_fits(
        &self,
        proposal: ProposalSize,
        children: &[&dyn SubView],
    ) -> Size;

    /// Phase 2: Place children within the given bounds.
    fn place(
        &self,
        bounds: Rect,
        children: &[&dyn SubView],
    ) -> Vec<Rect>;

    /// Which axis this container stretches on.
    fn stretch_axis(&self) -> StretchAxis {
        StretchAxis::None
    }
}
```

Layout happens in two phases:

1. **Sizing** (`size_that_fits`): The parent proposes a size. The layout queries its
   children (possibly multiple times) and returns how big it wants to be.
2. **Placement** (`place`): The parent provides final bounds. The layout positions
   each child within those bounds, returning a `Rect` per child.

This separation is important: during the sizing phase, you can probe children with different proposals to understand their flexibility before committing to a final arrangement.

## ProposalSize

The parent communicates its intentions through `ProposalSize`:

```rust,ignore
pub struct ProposalSize {
    pub width: Option<f32>,
    pub height: Option<f32>,
}
```

Each dimension can be:

| Value              | Meaning                              |
|--------------------|--------------------------------------|
| `None`             | "Tell me your ideal/intrinsic size"  |
| `Some(0.0)`        | "Tell me your minimum size"          |
| `Some(f32::INFINITY)` | "Tell me your maximum size"      |
| `Some(value)`      | "I suggest you use this size"        |

Children are free to return any size they want -- the proposal is just a suggestion.
The layout then decides how to handle the response.

The `ProposalSize` type provides convenience constants:

```rust,ignore
ProposalSize::UNSPECIFIED  // None, None -- ideal size
ProposalSize::ZERO         // Some(0.0), Some(0.0) -- minimum size
ProposalSize::INFINITY     // Some(INF), Some(INF) -- maximum size
```

> **Note:** A child is never obligated to accept a proposal. A `Text` view, for example, always returns its intrinsic size based on the text content and font, regardless of what size is proposed.

## The SubView Proxy

During layout, the container does not have direct access to child views. Instead,
it works through the `SubView` trait, which exposes a `measure()` method (the dual
of the layout's own `size_that_fits`):

```rust,ignore
pub trait SubView {
    /// Measure the child for a given proposal. May be called repeatedly.
    fn measure(&self, proposal: ProposalSize) -> ViewDimensions;

    /// Which axis this child stretches on.
    fn stretch_axis(&self) -> StretchAxis;

    /// Layout priority for space distribution.
    fn priority(&self) -> i32;
}
```

Key design principles:

- **Pure functions**: All `SubView` methods take `&self` with no side effects.
  You can call `measure` multiple times with different proposals to probe
  a child's flexibility.
- **Backend-managed caching**: Measurement results are cached by the native
  backend, not in Rust. The `SubView` proxy simply delegates to the backend's
  cache.
- **Priority**: Higher-priority children are measured first and get space
  preference. This allows important content to claim space before flexible
  elements like spacers.

## StretchAxis

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

`MainAxis` and `CrossAxis` are relative to the parent container:
- In a `VStack`, `MainAxis` = vertical, `CrossAxis` = horizontal.
- In an `HStack`, `MainAxis` = horizontal, `CrossAxis` = vertical.

This allows `Spacer` to always push siblings apart regardless of the container
orientation, and `Divider` to always span the cross axis.

## How Built-in Layouts Work

Now let's see how the theory applies in practice with WaterUI's built-in layout containers.

### VStack and HStack

Stacks are the most common layout containers. Their algorithm:

**Sizing phase:**
1. Separate children into fixed (non-stretchy) and flexible (stretchy) groups.
2. Propose the full available size to each fixed child, collect their sizes.
3. Calculate remaining space after fixed children and spacing.
4. Distribute remaining space among flexible children, proposing equal shares.
5. Sum all child heights (VStack) or widths (HStack) plus spacing.

**Placement phase:**
1. Start at the top (VStack) or leading edge (HStack).
2. Place each child sequentially, advancing by child size plus spacing.
3. Align children on the cross axis according to the stack's alignment parameter.

A `VStack` reports `StretchAxis::Horizontal` by default -- it fills available width
but determines its own height from content.

### Frames

The `frame()` modifier constrains a view to specific dimensions:

```rust,ignore
text("Hello").frame(width: 200.0, height: 100.0)
```

The frame layout proposes the constrained size to its child, then returns exactly
the requested dimensions. If only one dimension is specified, the other uses the
child's intrinsic size.

### Grids

Grid layout arranges children in rows and columns with configurable column
definitions. Each column can be fixed-width, flexible, or adaptive.

### ScrollView

`ScrollView` proposes infinite size along its scroll axis, allowing children to
be larger than the visible area. The native backend handles the actual scrolling
behavior.

### Padding

The `Padding` modifier insets the child by specified amounts on each edge:

```rust,ignore
text("Padded").padding(EdgeInsets::all(16.0))
```

During sizing, it adds the padding to the child's size. During placement, it
offsets the child's origin by the padding amounts.

## Writing a Custom Layout

To create a custom layout, implement the `Layout` trait. Here is a flow layout that wraps children to the next line when they exceed the available width:

```rust,ignore
use waterui_core::layout::*;

#[derive(Debug)]
pub struct FlowLayout {
    pub h_spacing: f32,
    pub v_spacing: f32,
}

impl Layout for FlowLayout {
    fn size_that_fits(
        &self,
        proposal: ProposalSize,
        children: &[&dyn SubView],
    ) -> Size {
        let max_width = proposal.width_or(f32::INFINITY);
        let mut x = 0.0_f32;
        let mut y = 0.0_f32;
        let mut row_height = 0.0_f32;
        let mut total_width = 0.0_f32;

        for child in children {
            let child_size = child.measure(ProposalSize::UNSPECIFIED).size;

            if x + child_size.width > max_width && x > 0.0 {
                // Wrap to next line
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

    fn place(
        &self,
        bounds: Rect,
        children: &[&dyn SubView],
    ) -> Vec<Rect> {
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

The key pattern: call `child.measure()` in both phases with consistent
proposals, so the sizes you computed in phase 1 match what you place in phase 2.

> **Try it yourself:** Implement a custom layout that arranges children in a circle. Use `size_that_fits` to calculate the bounding box, and `place` to position each child at an angle around the center.

## Safe Area

Safe area handling is intentionally **not** part of the `Layout` trait. Safe areas
are a platform-specific concept:
- iOS: Notch, home indicator, status bar
- Android: Navigation bar, status bar, cutouts
- macOS: Toolbar, title bar

Each backend handles safe area insets natively. Views can opt out using the
`IgnoreSafeArea` metadata (exposed via `.ignoring_safe_area()` modifier), which
tells the backend to extend the view beyond the safe area boundaries.

## Geometry Types

The layout module provides four fundamental geometry types:

| Type           | Fields                 | Description                        |
|---------------|------------------------|------------------------------------|
| `Point`       | `x: f32, y: f32`      | Position relative to parent origin |
| `Size`        | `width: f32, height: f32` | Two-dimensional extent          |
| `Rect`        | `origin: Point, size: Size` | Positioned rectangle           |
| `ProposalSize`| `width: Option<f32>, height: Option<f32>` | Size negotiation |

`Rect` provides convenience methods for common geometric queries:

```rust,ignore
let rect = Rect::new(Point::new(10.0, 20.0), Size::new(100.0, 50.0));
rect.min_x()   // 10.0
rect.max_x()   // 110.0
rect.mid_x()   // 60.0
rect.center()  // Point(60.0, 45.0)
rect.inset(10.0, 10.0, 20.0, 20.0)  // Shrink by padding
```

## Layout and the FFI

The `Layout` trait lives entirely in Rust. Backends that delegate to a platform
layout system (the Apple backend through SwiftUI/Auto Layout, the Android backend
through Android's view hierarchy) do not use this trait directly -- they read each
view's `StretchAxis` over FFI through `waterui_view_stretch_axis()` and let the
host system position widgets.

The Rust `Layout` trait is the source of truth for any backend that performs
layout itself, including:

- The **Hydrolysis backend** (the active Rust-side self-drawn renderer).
- Any custom Rust-based backend you write on top of `waterui-backend-core`.

## What's Next

Layout puts views in the right place, but different platforms need different backends to make it all happen. The [next chapter](04-backends.md) surveys WaterUI's backend architecture -- Apple, Android, GTK4, and the experimental Hydrolysis renderer.

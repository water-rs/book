# Layout Internals

WaterUI implements a custom layout protocol that runs in Rust but coordinates with the native backend. This ensures consistent layout logic across all platforms while using native widgets.

## The Protocol

The layout system is based on a two-phase "Propose and Response" model, defined by the `Layout` trait in `waterui-core`.

```rust
pub trait Layout: Debug {
    fn size_that_fits(&self, proposal: ProposalSize, children: &[&dyn SubView]) -> Size;
    fn place(&self, bounds: Rect, children: &[&dyn SubView]) -> Vec<Rect>;
}
```

### 1. Sizing Phase (`size_that_fits`)
The parent container proposes a size to the layout. The layout then negotiates with its children to determine its own ideal size.
*   `ProposalSize` can have `Option<f32>` dimensions. `None` means "unconstrained" (ask for ideal size), while `Some(val)` means "constrained" (fit within this size).

### 2. Placement Phase (`place`)
Once the size is determined, the parent tells the layout its final `bounds`. The layout then calculates the `Rect` (position and size) for each child.

## SubView Proxy

Layouts do not interact with `View`s directly. They interact with `SubView` proxies. This abstracts the differences between Rust views and native widgets.

```rust
pub trait SubView {
    fn size_that_fits(&self, proposal: ProposalSize) -> Size;
    fn stretch_axis(&self) -> StretchAxis;
    fn priority(&self) -> i32;
}
```

The native backend implements `SubView` via FFI. When Rust calls `size_that_fits` on a `SubView`, it triggers a call to the native platform to measure the actual text or widget.

## Stretch Axis

Views declare how they want to behave when there is extra space using `StretchAxis`:

*   `None`: Content-sized (e.g., Text, Button).
*   `Horizontal`: Expands width (e.g., TextField, Slider).
*   `Vertical`: Expands height.
*   `Both`: Fills all space (e.g., Color, ScrollView).
*   `MainAxis` / `CrossAxis`: Context-dependent (e.g., Spacer, Divider).

Containers like `VStack` and `HStack` use this information to distribute space proportionally among flexible children.

## Logical Pixels

All layout calculations in Rust use **Logical Pixels** (points).
*   1 logical pixel = 1 point in iOS/macOS.
*   1 logical pixel = 1 dp in Android.

The native backend handles the conversion to physical pixels for rendering. This ensures that a `width(100.0)` looks the same physical size on all devices.

## Writing Custom Layouts

To create a custom layout:
1.  Implement the `Layout` trait.
2.  Wrap it in a `FixedContainer` (for static children) or `LazyContainer` (for dynamic `ForEach` children).

```rust
struct MyLayout;

impl Layout for MyLayout {
    // ... implement size_that_fits and place
}

pub fn my_layout(content: impl View) -> impl View {
    FixedContainer::new(MyLayout, content)
}
```
# The layout engine

> **In this chapter, you will:**
>
> - Follow the proposal protocol: how a parent asks a child for a size and how the child answers
> - Read the `SubView` and `Layout` contracts, including placement and the response rectangle
> - See how stretch, priority and the stack allocation decide who gets the space
> - Write a custom layout that obeys those contracts
> - Learn what a backend does with the result, and where safe areas fit

A parent proposes a size, the child answers with the size it chooses, and the parent places the child. Everything in this chapter is a consequence of that negotiation. The rules are written down in [`docs/layout-spec.md`](https://github.com/water-rs/waterui/blob/dev/docs/layout-spec.md), which is normative and frozen: a difference between that document and the code is a bug in the code. This chapter explains the contract; the [layout chapter](../03-ui/02-layout.md) shows how to use the containers.

## Logical pixels

Every layout value is a logical pixel: a point on Apple platforms, a dp on Android. Backends convert to device pixels with the display scale, so layout never sees a device pixel. A 44pt-high button with 16pt padding in a design tool is `.height(44.0).padding_with(16.0)` here, with no conversion.

`Rect` origins are physical, left to right and top to bottom. Leading and trailing are resolved by containers when they place children, so leaf and renderer code receives physical coordinates.

Extents are finite and non-negative, with one exception: a maximum probe may answer `f32::INFINITY`, meaning unbounded. NaN, negative infinity and negative extents are contract violations reported as `LayoutError`. A zero extent is real geometry.

## The proposal protocol

A parent negotiates with a child through `ProposalSize`, one `Option<f32>` per axis:

```rust,ignore
pub struct ProposalSize {
    pub width: Option<f32>,
    pub height: Option<f32>,
}
```

| Proposal              | Question                |
|-----------------------|-------------------------|
| `None`                | Ideal (intrinsic) extent |
| `Some(0.0)`           | Minimum extent          |
| `Some(f32::INFINITY)` | Maximum extent          |
| `Some(v)`             | Fit into `v`            |

`ProposalSize::UNSPECIFIED`, `ProposalSize::ZERO` and `ProposalSize::INFINITY` name the three probes on both axes. `with_width` and `with_height` replace one axis, and `width_or` / `height_or` read an axis with a default.

The child answers with `ViewDimensions`: a `Size` plus any explicit alignment guides it published. The answer is the child's choice and the proposal is only advice. `Text` answers with the width of its widest laid-out line whatever you propose; it wraps when the proposal is narrower than its ideal width. On each axis the three probe answers satisfy `min <= ideal <= max`. A view that breaks this makes a container compress it below a size it cannot take or stretch it past one, and the symptom shows up in some other view.

The raw answer to a proposal is never overwritten. A container may *account* for a child at a larger extent (a stack uses `max(answer, offer)` for a stretching child), but that decides the child's allocation, not what the child answered.

## The `SubView` contract

A container never touches its child views. Every child it measures is a `SubView`:

```rust,ignore
pub trait SubView {
    fn measure(&self, proposal: ProposalSize) -> ViewDimensions;
    fn resolve_extent(
        &self,
        bounds: Rect,
        proposal: ProposalSize,
    ) -> Result<ResolvedExtent, LayoutError>;
    fn stretch_axis(&self) -> StretchAxis;
    fn priority(&self) -> i32;
    fn is_empty(&self) -> bool { false }
}
```

- **`measure`** is pure for a given proposal within one layout pass, and may be called many times with different proposals.
- **`resolve_extent`** answers where the view's answer to a proposal sits when it is assigned `bounds`. It is a pure query with the same rules as `measure`: it places nothing, installs no frame and emits nothing. [Response geometry](#response-geometry) below explains what it is for.
- **`stretch_axis`** declares which axes the view fills when offered more than it measures. It is read before `body` runs and without an environment, so a view with a filling leaf inside declares it here.
- **`priority`** orders the compression bands of a stack. Ties share.
- **`is_empty`** is a semantic question, not a size: it is `true` only when the whole subtree renders nothing. A zero-size `Color` or `Spacer` still renders and answers `false`.

Measurement is single-threaded and runs on the thread that drives layout. A `SubView` is neither `Send` nor `Sync`.

### Caching is the child's job

The `Layout` trait deliberately has no cache and never gains one. Containers probe freely, and a container-level cache would have to guess which probes repeat. Caching belongs to the `SubView`, and an expensive measure, text shaping above all, must cache.

`MemoizedSubView` wraps a child for one pass and remembers what each distinct proposal measured. You rarely build one yourself: `measure_layout`, `resolve_layout_extent` and `place_layout` memoize their children for the call. Driving a `Layout` directly means owning that boundary with `with_memoized_children`.

## The `Layout` trait

A container is a `Layout`. It is defined in `waterui_core::layout` and reached as `waterui::layout::Layout`: `waterui::layout` is the `waterui-layout` crate, which re-exports the core layout module beside its containers.

```rust,ignore
pub trait Layout: Debug + Any {
    fn size_that_fits(&self, proposal: ProposalSize, children: &[&dyn SubView]) -> Size;

    fn place(
        &self,
        bounds: Rect,
        proposal: ProposalSize,
        children: &[&dyn SubView],
    ) -> Vec<SubviewPlacement>;

    fn resolve_extent(
        &self,
        input: PlacementInput,
        children: &[&dyn SubView],
    ) -> Result<Rect, LayoutError>;

    // Defaulted hooks:
    fn explicit_horizontal(&self, alignment: HorizontalAlignment, bounds: Rect,
        children: &[PlacedSubview<'_>]) -> Option<f32> { None }
    fn explicit_vertical(&self, alignment: VerticalAlignment, bounds: Rect,
        children: &[PlacedSubview<'_>]) -> Option<f32> { None }
    fn explicit_horizontal_alignments(&self) -> Vec<HorizontalAlignment> { Vec::new() }
    fn explicit_vertical_alignments(&self) -> Vec<VerticalAlignment> { Vec::new() }
    fn stretch_axis(&self, children: &[StretchAxis]) -> StretchAxis { StretchAxis::None }
    #[doc(hidden)]
    fn watch_invalidation(&self, invalidate: LayoutInvalidationCallback)
        -> Vec<BoxWatcherGuard> { Vec::new() }
}
```

Three methods are required.

- **`size_that_fits(proposal, children)`** is the container's answer to a proposal. It may probe children freely.
- **`place(bounds, proposal, children)`** returns one `SubviewPlacement` per child, in order, in the coordinate system of `bounds`: a frame includes the bounds' origin, it is not offset from it. `bounds` is the frame the parent resolved for this container; `proposal` is the proposal the container was measured with, for layouts whose rule needs it. A `SubviewPlacement` carries the child's `frame` and the `proposal` the child is placed with, and the child's own layout pass receives that proposal verbatim.
- **`resolve_extent(input, children)`** positions the container's own response rectangle. `input` is a `PlacementInput`: the assigned `bounds`, the incoming `proposal` (never reconstructed from the bounds) and the container's `answer` to it. The rule is yours and there is no default. A container whose rule derives from a child asks that child's `resolve_extent`, never its placement.

You do not call these three in sequence yourself. The driver owns the pairing: `place_layout(layout, bounds, proposal, children)` measures the answer once, resolves the response and places the children from the same snapshot, and returns a `LayoutPlacement` (the answer, the response rectangle, the child placements). `resolve_layout_extent` is the query alone, and is what a container's `SubView::resolve_extent` returns. Both reject a response whose extent is not the concretized answer.

The hooks cover the rest. A container exposes explicit alignment guides through the `explicit_*` methods; resolving a guide runs `place`, so a guide always agrees with the placement, and guides stay in allocation-local coordinates. `stretch_axis` receives each child's own axis and relays what they claim (see [Stretch](#stretch)). `watch_invalidation` is `#[doc(hidden)]` backend infrastructure, covered under [reactive layout parameters](#reactive-layout-parameters).

### Placement negotiates against the resolved bounds

Placement is not a replay of measurement. When a container is placed, it negotiates with its children again against the bounds it was actually given, on both axes, so children see the real geometry and never a stale probe. A stack allocates its main axis from `bounds` and proposes its resolved cross extent to every child. A `ZStack`, `Overlay`, `Background`, `Absolute` and `Padding` propose the bounds (less insets) they place into. A `Frame` proposes the region it resolved, an `AspectRatio` the ratio-correct box.

A container never places a child under a proposal it did not derive from `bounds`. The one documented exception is `Grid`, whose cells keep the measurement pass's proposals so that a content-sized grid keeps content-sized columns and rows.

### Response geometry

Every placed view has two rectangles. Its *allocation* is the frame its parent assigned; children, drawing and hit testing use it. Its *response rectangle* is its answer to its placement proposal, positioned inside the allocation by its layout's own rule. Accessibility uses the response.

The response obeys a few rules:

- On every axis its extent is the view's concretized answer to its placement proposal. A `+INFINITY` axis takes the assigned extent; every other axis keeps the answer.
- It is the answer, not the content. It does not promise to enclose children or their drawing, and it may extend outside the allocation. Neither implies clipping.
- The rule belongs to the layout or leaf contract. A backend never reconstructs the rectangle from descendants.

A leaf without an explicit content-placement policy answers with `ResolvedExtent::centred(bounds, answer)`: its answer centered in its allocation. A text that is the whole content of a page therefore sits in the middle of the page at its own size. Stacks center their response on the main axis and anchor it by their alignment on the cross axis, a `ZStack` anchors it by its alignment on both axes, `Frame` and `AspectRatio` center theirs, `Absolute` puts it at its bounds' origin and `Grid` at their top-leading corner, `Padding` out-sets its child's response by the insets, and `Background`, `Overlay` and metadata wrappers take their content's (or base's) response. The container's own section of the spec lists each rule.

## Stretch

`stretch_axis` is the only way a view claims space beyond its measured size:

```rust,ignore
pub enum StretchAxis {
    None,       // content-sized
    Horizontal, // expands width, intrinsic height
    Vertical,   // intrinsic width, expands height
    Both,       // fills all space
    MainAxis,   // expands along the parent stack's main axis
    CrossAxis,  // expands along the parent stack's cross axis
}
```

`MainAxis` and `CrossAxis` are resolved against the enclosing stack: in a `VStack` the main axis is vertical, in an `HStack` it is horizontal. That is what lets `Spacer` push siblings apart in either orientation and `Divider` span the cross axis in either orientation.

The rules a container follows:

- A stretching leaf is promoted to `max(measured, offered)` only where the container's allocation rule promotes it (a stack's offers). A `ZStack` proposes the bounds but keeps a stretcher's finite answer. Promotion changes the child's allocation, never its answer: a `Spacer` allocated a 40pt gap still answers its minimum length.
- A container claims nothing of its own. Its stretch is the union of its children's, resolved against its axis. A `VStack` holding a `Spacer` stretches vertically, one holding a `Color` stretches on both axes, and a row of labels stretches on neither. This is why `Layout::stretch_axis` receives the children's axes: a container that stayed silent would be handed its own intrinsic size, and the greedy child inside would fill a box of nothing.
- Modifier containers (`Padding`, `Background`, `Overlay`, `AspectRatio`, metadata wrappers) are transparent and report their content's axis.
- A `Frame` stretches on an axis only when its `max` there is `INFINITY`. `.width(v)` and `.max_width(100.0)` do not stretch; `.max_width(f32::INFINITY)` is how a view opts in to filling.
- `Absolute` is `Both` regardless of its children. A `List` is `Both`. A `ScrollView` stretches on its scrolling axis, and on the other axis only when its content does.
- A lazy stack cannot enumerate its children before they are realized, so it reports `None`. A lazy stack that must fill is placed by a parent that stretches it.
- `Spacer` is `MainAxis`, `Divider` is `CrossAxis`, `Color` and shapes are `Both`, text, buttons and other content controls are `None`, and a text field is `Horizontal`. A toggle is `Horizontal` when it draws the switch style with a visible label, a stepper when its label is visible, and a picker in the segmented style; otherwise each is `None`.

## Stacks

The description is written for an `HStack`; a `VStack` is the same with the axes swapped. The default spacing is 10pt.

### Cross axis: the alignment envelope

Each child answers a cross extent `e` and a guide `g` for the stack's alignment: its explicit guide when it published one, otherwise the alignment's default position. In a stack, guides are not clamped into `[0, e]`; a child may raise its guide above its own top. The stack's cross extent is the envelope those guides form:

```text
above = max over children of g
below = max over children of (e - g)
cross = above + below
```

A stack never widens itself to its proposal: a column of two labels is as wide as the wider label wherever it is placed. At placement the envelope is anchored in the bounds by the alignment (start for `Top` / `Leading`, center for `Center`, end for `Bottom` / `Trailing`). Nothing is clamped or clipped: a child wider than the bounds overflows, and the alignment decides which edges it crosses. A cross-axis stretcher is placed at the bounds' start with extent `max(measured, bounds.cross)`.

### Main axis: one probe, then allocation

With an unspecified main proposal, every child keeps its ideal extent. `0` and `INFINITY` proposals are forwarded unchanged. With a finite, non-zero main proposal the stack negotiates:

1. `available` is the main extent after member spacing, clamped to zero.
2. Each child's `min` is probed at main proposal zero. Its `target` is `max(min, available)` when it stretches on the main axis, otherwise its maximum-probe answer limited to `available` and raised to its `min`.
3. If every target fits, each child is proposed its target. If the minima alone exceed `available`, each child is proposed its `min` and the stack overflows.
4. Otherwise the priority bands negotiate from highest to lowest. Within a band, children negotiate by increasing flexibility (maximum answer minus `min`; a main-axis stretcher counts as unbounded), with ties resolved by logical member order whatever the layout direction. Before each offer, the stack deducts the reported extents of processed children, reserves the `min` of every unprocessed lower-priority child, and divides the band budget among the unprocessed children in the band. The offer is then limited so the band's other unprocessed children keep their `min`, and clamped between the child's `min` and `target`.
5. Each child is measured once at its offer. Its reported extent is its answer, or `max(answer, offer)` when it stretches on the main axis, and that is deducted before the next child. A processed child is never offered space again.

Answers larger than offers overflow; they are not clipped or replaced by the offers. At placement the same negotiation runs again with `main = bounds.main` and `cross = bounds.cross`. The negotiated block is centered in the bounds and the members are placed from its start at their reported extents, reversed physically for a right-to-left `HStack`. Centering moves the block as a whole: it never redistributes free space.

### Priority

`layout_priority(n)` sets a child's band; the default is `0`.

```rust
use waterui::prelude::*;

fn toolbar() -> impl View {
    hstack((
        text("A long document title that keeps its space").layout_priority(1),
        spacer(),
        text("Edit"),
    ))
}
```

The title's band negotiates first, so it is offered everything except the `min` that `Edit` reserves, and only then does `Edit` take its offer. Priority decides the order in which bands receive their offers; higher bands go first, while every unprocessed child keeps its `min` reserved. It decides sizing opportunity only. It does not force a child to accept an offer, and space a child declines stays available to later children, including those in lower bands. A `Spacer`'s default priority is `i32::MIN`, below every ordinary child, so it absorbs only what remains. An explicit `layout_priority` replaces that default outright: a `Spacer` raised to `0` negotiates as an ordinary stretching child.

### Spacing and membership

Spacing is reactive. A child that renders nothing (`is_empty()`) is not a stack member: it takes no slot and no spacing, and contributes to neither the envelope, nor the exported guides, nor the container's stretch. A conditional that flips between nothing and a view is a membership change and invalidates the stack. `Grid` is the exception: every child it is built with keeps a cell.

## Other containers

| Container | Rule |
|-----------|------|
| `ZStack` | Every child is measured with the stack's proposal. The answer is the alignment envelope over all children, not capped by the proposal. Children are placed by the alignment at their own answer and an unbounded answer fills the bounds. |
| `Overlay` (`.overlay`) | Sized by the base alone. Decorations are measured with the bounds and anchored on the envelope their own guide forms; they never influence the parent's sizing. |
| `Background` (`.background`) | Sized by the content alone. Both layers are placed over the whole bounds. `Material` and `Glass` are metadata the backend projects and do not enter layout. |
| `Padding` | Measures the child with the proposal shrunk by the insets and adds them back. Negative insets are legal: the extent is `max(0, child + insets)` and the child sits at the inset offset. Transparent to stretch and guides. |
| `Frame` | An inverted constraint (`min` above `max`) resolves in `min`'s favor. The child hears the proposal clamped into `[min, max]`, `ideal` answers only an unspecified axis, and the frame answers `min(max, max(child, proposal))`. It measures the child a second time under the exact proposal it will place with, so its answer and placement agree. |
| `AspectRatio` | `Fit` is the largest box of the ratio inside the offer, `Fill` the smallest box covering it. A non-finite proposal axis counts as unspecified. The child is placed in the resolved box, centered in the bounds. |
| `Absolute` | Answers the proposal: `0` on an unspecified axis, and the probe itself to a maximum probe (the one place a container answers `INFINITY` without a child doing so). It gives every child the full bounds as frame and proposal. `pin` and `position_in` resolve against the bounds; leading and trailing are logical edges. |
| `Grid` | Fixed column count. Under a finite width each column is `max(0, (width - spacing * (columns - 1)) / columns)` wide and the grid answers the proposal width; otherwise each column is as wide as its widest cell's ideal answer. Cells are proposed `(column width, None)`. A `GridRow` is construction-time grouping and has no size of its own. |
| `Spacer` | `MainAxis`. Answers its minimum length (`spacer_min(n)`, 0 by default) to every measurement on the stack's main axis and zero elsewhere. Reported extent during allocation is `max(minimum, offer)`. |
| `Divider` | Orientation comes from the enclosing stack: 1pt wide in an `HStack`, 1pt high in a `VStack`, and a horizontal rule in a container that establishes no axis. Stretch is `CrossAxis`. |
| Lazy containers (`List`, lazy stacks) | Virtualized along one axis with the same per-child protocol over the realized children. Their response comes from the logical extent state their measurement uses, never from the union of realized children. |

## Reactive layout parameters

Layout inputs are signals, not snapshots. A stack's `spacing` and every `Frame` dimension hold a `Computed`, so passing a binding produces a real re-layout when it changes:

```rust
use waterui::prelude::*;

fn spaced() -> impl View {
    let gap = Binding::f32(8.0);

    hstack((text("left"), text("right"))).spacing(gap)
}
```

`.spacing(...)` accepts anything implementing `IntoSignalF32`, which covers integer and float literals as well as signals.

The mechanism is `Layout::watch_invalidation`. A layout returns watcher guards for its own reactive fields; the native container holds them for the layout's lifetime and requests a new layout pass when one fires. The hook has a default empty implementation, so implement it only if your custom layout stores signals. Because building a `Computed` is not a const operation, `hstack()` and `vstack()` are not `const fn`.

## Writing a custom layout

Implement `Layout`, then wrap it in a `FixedContainer` to get a view. `FixedContainer::new(layout, children)` takes the layout and a tuple of views, and wraps the layout so that it is mirrored under a right-to-left layout direction: your layout computes in left-to-right coordinates and does not handle direction itself. (`LazyContainer` is the equivalent for children produced lazily from a collection.)

Here is a flow layout that wraps children onto new rows when they exceed the available width:

```rust
use waterui::layout::container::FixedContainer;
use waterui::layout::{
    Layout, LayoutError, PlacementInput, Point, ProposalSize, Rect, Size, SubView,
    SubviewPlacement, UnitPoint,
};
use waterui::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct FlowLayout {
    pub h_spacing: f32,
    pub v_spacing: f32,
}

impl FlowLayout {
    /// The proposal each child is measured with: the wrap width when it is
    /// finite, never a maximum probe, and an unspecified height.
    fn probe(width: Option<f32>) -> ProposalSize {
        ProposalSize {
            width: width.filter(|width| width.is_finite()),
            height: None,
        }
    }

    /// Lays children out in rows no wider than the probe, relative to the
    /// origin, and returns the frames with the block's size.
    fn rows(&self, probe: ProposalSize, children: &[&dyn SubView]) -> (Vec<Rect>, Size) {
        let limit = probe.width_or(f32::INFINITY);
        let mut frames = Vec::with_capacity(children.len());
        let (mut x, mut y, mut row_height, mut block_width) = (0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32);

        for child in children {
            // An empty child is not a member: no slot and no spacing.
            if child.is_empty() {
                frames.push(Rect::from_size(Size::zero()));
                continue;
            }

            let size = child.measure(probe).size;
            if x > 0.0 && x + size.width > limit {
                y += row_height + self.v_spacing;
                x = 0.0;
                row_height = 0.0;
            }

            frames.push(Rect::new(Point::new(x, y), size));
            x += size.width + self.h_spacing;
            row_height = row_height.max(size.height);
            block_width = block_width.max(x - self.h_spacing);
        }

        (frames, Size::new(block_width, y + row_height))
    }
}

impl Layout for FlowLayout {
    fn size_that_fits(&self, proposal: ProposalSize, children: &[&dyn SubView]) -> Size {
        self.rows(Self::probe(proposal.width), children).1
    }

    fn place(
        &self,
        bounds: Rect,
        _proposal: ProposalSize,
        children: &[&dyn SubView],
    ) -> Vec<SubviewPlacement> {
        // Placement negotiates against the resolved bounds, not the proposal
        // this container was measured with.
        let probe = Self::probe(Some(bounds.width()));
        let (frames, _) = self.rows(probe, children);

        frames
            .into_iter()
            .map(|frame| SubviewPlacement::new(frame.translated(bounds.x(), bounds.y()), probe))
            .collect()
    }

    fn resolve_extent(
        &self,
        input: PlacementInput,
        _children: &[&dyn SubView],
    ) -> Result<Rect, LayoutError> {
        // The rows are placed from the top-leading corner, so the response
        // sits there at the concretized answer.
        Ok(input.bounds.align(input.extent()?, UnitPoint::TOP_LEADING))
    }
}

pub fn tags() -> impl View {
    FixedContainer::new(
        FlowLayout {
            h_spacing: 8.0,
            v_spacing: 8.0,
        },
        (
            text("Rust"),
            text("Layout"),
            text("中文"),
            text("日本語"),
            text("한국어"),
        ),
    )
}
```

Each part follows a rule from this chapter:

- `size_that_fits` and `place` share one `rows` function, so the measured size and the placed frames cannot drift apart.
- `place` derives its child proposal from `bounds` and hands the same proposal back in each `SubviewPlacement`. The child's own layout pass receives exactly that proposal, so wrapped text lays out at the width it was placed at.
- The probe never forwards a maximum probe to children. Under an `INFINITY` width proposal the layout answers a single row of ideal widths, finite, instead of asking every child how wide it could possibly be.
- `resolve_extent` is mandatory and states where the answer sits. It reads no child, so it works for a lazy container too. The returned rectangle must have exactly the extent `input.extent()` reports; the driver rejects anything else.
- The layout keeps the default `stretch_axis`: its answer comes entirely from its content. A layout whose children's fill should be visible to its parent returns the union of the `children` axes it was given instead of copying them when it is built.

> **Try it yourself:** arrange children in a circle. Use `size_that_fits` for the bounding box and `place` to position each child at an angle around the center, then decide where `resolve_extent` puts the response.

A custom layout stays within the same freeze as the built-in ones. If you find a case the specification does not decide, report it as an issue; do not pick a behavior in code and rely on it.

## Leaf contracts

A backend hosts native leaves inside Rust-driven containers, and a leaf's `measure` is the platform's answer under the same protocol.

- **Text.** The answer's width is the widest laid-out line, never the proposal. A finite width wraps, `0` answers the narrowest wrap, `None` answers the unwrapped line. Height is the line count times the platform line box, snapped to the display scale.
- **Controls.** The platform control's intrinsic size, with the platform's own chrome padding for the selected style. A text field answers the proposal width and its intrinsic height. A switch-style toggle or a stepper with a visible label answers the proposal width and places the label at the leading edge and the control at the trailing edge; with a hidden label, or as a checkbox-style toggle, it answers its intrinsic size. A segmented picker answers the proposal width and divides it into equal segments; an unbounded width proposal gets its intrinsic width. The automatic toggle style resolves to the platform's default (switch on iOS and Android, checkbox on macOS) before this rule applies.
- **`ScrollView`.** On the scrolling axis it claims the whole offer: a finite proposal is answered with the proposal, `None` with the content's intrinsic extent, `0` with `0`. On the other axis it answers the content's own answer, clamped to a finite offer, so a scroll hugs non-filling content. Content is measured with the viewport extent on the non-scrolling axis and `None` on the scrolling axis, and is centered on the non-scrolling axis whether it is narrower or wider than the viewport.
- **Images.** A non-resizable image answers its intrinsic size to an unspecified proposal, scales down (preserving its aspect ratio) under a smaller finite proposal and never scales up. `.resizable()` fills the proposal per `content_mode`.
- **GPU surfaces, shapes, colors.** `Both`.
- **Video.** A `Fit` video answers a finite width with that width and the height its source's aspect ratio gives, and stretches horizontally. `Fill` and `Stretch` answer the proposal on both axes.

A leaf that answers a probe with the proposal instead of its content, on an axis it does not stretch, is a backend bug. It makes every container above it report the proposal, and the failure appears as a stretched card or a leading-parked column far from the leaf.

## Layout on the backends

A backend applies the Rust placement verbatim. The child's frame is the placement rectangle, and its layout pass receives the placement proposal. Backends do not hand container layout to the platform's layout system: both mainline backends drive containers through `place_layout`. Apple does it from its AppKit and UIKit bridge, Hydrolysis from its renderer tree, and a custom backend built on `waterui-backend-core` follows the same contract.

- **Retained geometry.** With each committed node a backend retains its `PlacedGeometry`: the placement `proposal`, the raw `answer`, the `allocation` and the allocation-local `response_rect`. A node's geometry becomes visible atomically with its placement.
- **Bridge wrappers.** A wrapper standing in a child's slot (a metadata wrapper, a clip host) lays its content over the whole slot. It never re-centers, re-measures or gravity-packs the content, and it forwards the response query with the same bounds and proposal however many host views the backend builds for it.
- **The root.** The root proposal is the window's (or scene's) content size, both axes finite. The root is placed at its answer to that proposal and stretched to the window on the axes it declares.
- **Native leaves.** A backend draws a leaf's intrinsic content in its response rectangle, so drawing and accessibility agree. It never shrinks or moves a platform view to make its accessibility frame come out right; it reports the projected rectangle through the platform's accessibility-frame API and leaves the Rust-assigned allocation in place.
- **Threading.** Layout is single-threaded. A backend that measures on another thread is outside the contract.

### Safe area

Safe-area insets are a backend concern applied outside the protocol; they never change a proposal a Rust container sees. The safe area has two regions on each edge: *container* (system bars, display cutouts, the home indicator, window chrome) and *keyboard* (software keyboard and other input-method surfaces).

- **Layout avoids.** A view is laid out clear of every region it does not ignore. `ignore_safe_area` names the regions and edges it ignores, and a stack lays its children out inside what remains.
- **Paint extends for fills.** A background whose content is a fill (a solid color, a gradient or a material) extends past its frame to the window edge on every edge it touches, through both regions, without moving the content it backs. An `ignore_safe_area` on the fill replaces that default with the regions and edges it names. Any other background, and a fill that is not a background, extends only where it ignores the safe area.
- **Scroll surfaces.** A scroll surface touching an edge extends under that edge's regions and insets its content by them. When a text field inside it gains focus, it scrolls the minimum distance that brings the field clear of the keyboard.
- **Chrome.** A navigation, tool or tab bar host extends each bar under the regions of the edge the bar touches. A bar stays docked, so the keyboard covers it rather than lifting it, while the content the host contains is laid out clear of both regions.
- **Keyboard motion.** A change of the keyboard inset follows the platform's keyboard animation frame by frame. A backend never avoids the keyboard by resizing the window or its rendering surface.

To opt out on specific edges:

```rust
use waterui::layout::EdgeSet;
use waterui::prelude::*;

fn backdrop() -> impl View {
    Color::red().ignore_safe_area(EdgeSet::ALL)
}

fn header() -> impl View {
    text("Title").ignore_safe_area(EdgeSet::TOP)
}
```

To ignore only the keyboard region on an edge, pass `SafeAreaRegions::KEYBOARD.on(EdgeSet::BOTTOM)` instead of an `EdgeSet`.

### Accessibility geometry

An accessibility element reports its *geometry owner*'s response rectangle, and that owner can differ from the object that performs its actions. In `button("OK").padding_with(8.0).a11y_label("x")` the padded view owns the geometry and the button the actions, while `button("OK").a11y_label("x").padding_with(8.0)` makes the labeled button the owner. Modifier order decides. See the [accessibility chapter](../06-advanced/05-accessibility.md) for the authoring side.

## What's next

Layout decides where views go; backends decide what they are made of. The [next chapter](04-backends.md) surveys the backend architecture, and the [rendering chapter](01-rendering.md) follows a frame from the view tree to pixels.

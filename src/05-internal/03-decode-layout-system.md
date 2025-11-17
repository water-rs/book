# Layout Internals

The high-level layout chapter introduced stacks, padding, and grids. This appendix dives deeper into
`waterui_layout` so you can reason about performance and implement custom layouts when the built-in
ones do not suffice.

## Proposal → Size → Placement

Every layout implements:

```rust
pub trait Layout {
    fn propose(&mut self, parent: ProposalSize, children: &[ChildMetadata]) -> Vec<ProposalSize>;
    fn size(&mut self, parent: ProposalSize, children: &[ChildMetadata]) -> Size;
    fn place(
        &mut self,
        bounds: Rect,
        proposal: ProposalSize,
        children: &[ChildMetadata],
    ) -> Vec<Rect>;
}
```

- `ProposalSize` contains optional width/height hints (`None` = unconstrained).
- `ChildMetadata` includes stretch flags, intrinsic baselines, and previous measurements.
- `Size` / `Rect` use logical pixels (`f32`).

Stacks, grids, and scroll views you use every day are thin wrappers around these methods.

## Container and FixedContainer

Declarative layout constructors (`vstack((…))`) ultimately call into `FixedContainer::new(layout,
tuple_of_children)`; dynamic lists (`ForEach`, diffing) use `Container` with `AnyViews` so children
can be replaced incrementally.

When building custom layouts, decide which wrapper you need:

- Use `FixedContainer` for small, static tuples (tabs with three children, for example).
- Use `Container` when the layout should host arbitrary `AnyView` collections.

## Stretch Semantics

Stretch is controlled by `ChildMetadata::stretch`. Stacks mark `Spacer` and any view wrapped with
`.frame().max_width(...)` accordingly. Layout implementors should:

- Distribute leftover space proportionally across stretch children.
- Leave non-stretch children at their measured size.

This is how `spacer()` works without special-casing: it simply sets `stretch = true`.

## Writing Your Own Layout

1. Define a struct to hold per-render state (spacing, alignment, caches).
2. Implement `Layout` using the rules above.
3. Expose an ergonomic constructor returning `impl View` by wrapping the layout in a container.

```rust
pub fn badge_stack(children: (impl View, impl View)) -> impl View {
    FixedContainer::new(BadgeLayout::default(), children)
}
```

Refer to `waterui/components/layout/stack/*.rs` for concrete examples—the stack layouts are heavily
commented and cover alignment math, stretch, and baseline propagation.

Armed with these internals you can debug spacing issues, reason about measurement order, and extend
the layout system confidently.

# How WaterUI renders

> **In this chapter, you will:**
>
> - Follow a view from a Rust struct to a platform widget or a recorded draw command
> - Tell leaves, composite views and metadata apart, and know what a backend does with each
> - Choose between mandatory and ignorable metadata when you define your own
> - See why a signal change touches one widget or one recorded value and never diffs a tree
> - Know how stretch-axis negotiation and measurement fit into the render path

You do not need this chapter to build apps. You need it to debug a view that renders wrong, to write a backend or a component port, or to understand why a metadata type you invented panics with "not caught by your renderer".

Your application code produces a tree of Rust values implementing `View`. A backend turns that tree into something that draws. Everything between your code and the backend, the dispatch, the leaf types and the metadata wrappers, is Rust, in one process, with no C ABI in the middle.

## The pipeline

```text
Rust view tree  (your View types)
     |
     |  body() expansion, until a leaf or a metadata wrapper
     v
Native<T> leaves  +  Metadata<T> / IgnorableMetadata<T> wrappers
     |
     +-- Apple backend ------> AppKit / UIKit views, built from Rust through objc2
     |
     +-- Hydrolysis ---------> retained render tree
                                  |
                                  +-- Cherenkov GPU painter
                                  +-- HWUI painter on Android: RenderNode display lists
```

The tree is the same for every backend; backends differ in what they build from it. Per the repository's supported-platform policy, Apple (macOS, iOS) and Android are the tier-1 platforms, Windows and Linux are tier 2, and the GTK, WinUI and TUI backends are experimental and live in their own repositories. See [Platforms](../01-getting-started/05-platforms.md) and [Backends](04-backends.md) for what ships where.

| Backend | Lives in | Realizes a leaf as |
|---------|----------|--------------------|
| Apple | `backends/apple` | An AppKit or UIKit view, composed in Rust through objc2 |
| Hydrolysis | `backends/hydrolysis` | A node of a retained tree, painted by Cherenkov or, on Android, HWUI |

`backends/core` (`waterui-backend-core`) holds the infrastructure backends share: the `ViewDispatcher`, gesture recognition, scroll math, input events, frame triggers and the main-loop executor contract. The Swift package under the Apple backend (`Sources/WaterUI/Embedding.swift`) is a thin host that binds the backend's exported entry points; component rendering and layout stay in Rust.

## Three kinds of node

Expansion stops at one of three things.

### Composite views

A composite view has a `body()` that returns another view:

```rust
use waterui::prelude::*;

struct Greeting {
    name: Binding<String>,
}

impl View for Greeting {
    fn body(self, _env: &Environment) -> impl View {
        let name = self.name;
        vstack((text!("Hello, {name}"), spacer()))
    }
}
```

The backend calls `body()` and looks at what comes back, again and again, until the result is a leaf or a metadata wrapper. `body` consumes the view (`self`, not `&self`), and the `&Environment` it receives is the one in scope at that position of the tree. See [the View chapter](../02-core/01-view.md) and [the Environment chapter](../02-core/03-environment.md).

### Native leaves

A leaf is a view whose type a backend claims. Components declare themselves with `raw_view!` (a plain leaf) or `configurable!` (a view plus a config struct). Both end with the view's `body()` returning `Native<T>`:

```rust,ignore
// waterui-core, core/src/components/native.rs
pub struct Native<T: NativeView> { /* native: T, fallback: Option<AnyView> */ }

pub trait NativeView {
    fn stretch_axis(&self) -> StretchAxis { StretchAxis::None }
}
```

`Native<T>` is the sentinel that says "stop here, hand this payload to the backend". A backend registers a handler for `Native<TextConfig>`, `Native<Color>`, `Native<Spacer>` and so on, and never sees `Text` or `Button` themselves, only the config they resolved to. The payload carries the signals the widget reads (`Computed`, `Binding`), so the backend keeps the live inputs rather than a snapshot of them.

`Native<T>::body()` returns the fallback attached with `with_fallback`. Without one, reaching it is a contract violation that names the type: a debug build panics, and a release build logs the error and renders nothing. A leaf nobody claimed fails loudly at the first render instead of silently drawing nothing.

`configurable!` adds one step before `Native`: when the environment holds a `Hook<Config>`, the hook receives the config and returns the view to use instead. That is how a style package or an application re-renders a component without forking it. Without a hook, the config becomes `Native<Config>`, optionally rewritten against the environment first. [Hooks in the Environment chapter](../02-core/03-environment.md#hooks-intercepting-component-configuration) shows them from the application side.

### Metadata wrappers

Modifiers that do not add a widget, such as opacity, scale, offsets, accessibility attributes and environment overrides, wrap their content instead:

```rust,ignore
// waterui-core, core/src/components/metadata.rs
pub struct Metadata<T: MetadataKey> {
    pub content: AnyView,
    pub value: T,
}
pub struct IgnorableMetadata<T: MetadataKey> {
    pub content: AnyView,
    pub value: T,
}
```

A backend claims `Metadata<Opacity>` the way it claims a leaf: it applies the value to the child it renders from `content` and continues into it. Metadata is transparent to layout. Both wrappers forward `stretch_axis()` to their content, so wrapping a view never changes how a stack sizes it.

## Mandatory and ignorable metadata

The two wrappers differ in what happens when no backend claims them.

| Wrapper | When a backend does not catch it |
|---------|----------------------------------|
| `Metadata<T>` | `body()` reports a contract violation: a debug build panics with "is not caught by your renderer", a release build logs the error and renders nothing |
| `IgnorableMetadata<T>` | `body()` returns the content unchanged, so the effect silently disappears |

Choose per modifier. Dropping an offset or a scale would draw the wrong thing, so those are mandatory. A platform-specific nicety, such as a material background that only some backends can draw, is ignorable: the content still renders everywhere. Define your own through the `waterui::metadata` module:

```rust
use waterui::metadata::{IgnorableMetadata, MetadataKey};
use waterui::prelude::*;

/// A hint backends may use. Not drawing it is harmless.
struct DebugLabel(&'static str);

impl MetadataKey for DebugLabel {}

fn labelled(content: impl View) -> impl View {
    IgnorableMetadata::new(content, DebugLabel("profile card"))
}
```

If you swapped `IgnorableMetadata` for `Metadata` here, every backend would hit the contract violation, because none of them knows `DebugLabel`. Making the key meaningful means adding a handler in a backend; see [Library authoring](05-library-authoring.md) and [Backends](04-backends.md). `Retain` is a built-in mandatory key that only keeps a value, such as a watcher guard, alive for as long as the view lives.

## Dispatch is by Rust type

A backend identifies what it holds by its `TypeId`. There is no hashed name and no table of exported ID functions. The unit of dispatch is the concrete type in the tree: `Native<TextConfig>`, `Metadata<Opacity>`, a composer a port takes before `body()`.

`waterui-backend-core` provides the reusable form of this loop as `ViewDispatcher<T, C, R>`: a table from `TypeId` to handler, plus state `T`, a context `C` handlers receive (a parent widget, say) and a result `R` they return.

```rust,ignore
let mut dispatcher: ViewDispatcher<(), Ctx, Widget> = ViewDispatcher::new();

dispatcher.register::<Native<TextConfig>>(|state, ctx, native, env| {
    // build a widget from native.as_inner()
});

let widget = dispatcher.dispatch(view, &env, ctx);
```

`dispatch` looks up the view's `TypeId`. A hit invokes the handler; a miss calls `body()` and dispatches the result. A concrete view stays on the stack and the handler receives it through a typed slot; an `AnyView` is unwrapped and dispatched by its inner type. Set `WATERUI_DISPATCH_DEBUG` in the environment and the dispatcher logs an indented trace of every type it walks through `tracing::debug!`, which is the fastest way to learn what your view expanded to.

At this revision neither in-tree backend instantiates `ViewDispatcher`; it is the shared building block for Rust-side backends, and each in-tree backend runs the same idea in its own form:

- **Apple** owns its dispatch in `backends/apple/src/dispatch.rs`. A `Dispatcher` is built once from `registry.rs`, which installs one module per component from `components/` (`text`, `button`, `slider`, `list`, `map` and many more), each behind its own Cargo feature so an unused component compiles out. Handlers come in two shapes: `register_native::<C>` claims `Native<C>` and receives the payload, and `register_view::<T>` claims a type as it appears in the tree, which is how metadata wrappers are taken. The loop is the one you would write: if a handler claims the view's type, run it, otherwise replace the view with its `body()` and look again. A handler answers with a `NativeLeaf`, a platform view together with the layout face a container lays out with. Registering a second handler for one type is an assertion failure, so no two ports can silently fight over a type.
- **Hydrolysis** builds a retained tree, described below. Its builder downcasts each view onto a closed set of node kinds (`Native<Color>`, `Native<TextConfig>`, containers, `Metadata<Opacity>` and so on) and expands anything else through `body()` once.

Both treat a claimed type as the end of that branch and expansion as the default.

## Fine-grained updates

The walk above runs once per subtree. After that, WaterUI does not compare trees. A reactive value in a leaf's payload is subscribed to by the node that draws it:

```rust
use waterui::prelude::*;

fn counter() -> impl View {
    let count = Binding::i32(0);
    let add = count.clone();

    vstack((
        text!("Count: {count}"),
        button("Add").action(move || *add.get_mut() += 1),
    ))
}
```

When `count` changes, the one text node that reads it is told; nothing above it re-runs, and `counter` itself is not called again. That is the reason for the golden rule in [the Reactive chapter](../02-core/02-reactive.md): a snapshot taken in a body has no subscription, so the node never hears about the change.

How "the node is told" plays out depends on the backend:

- **Apple.** A handler subscribes to the signals of its payload and keeps the watcher guards in the `KeepAlive` that belongs to the leaf. A change sets one property on one AppKit or UIKit object. Dropping the leaf drops its guards, in reverse order of insertion, so a watcher stops before the object it writes to is released.
- **Hydrolysis.** A node stores the live `Computed` or `Binding`, not a copy, and watches it. A signal that affects size, such as a text node's content, marks the node's layout dirty. No `body()` runs again; the next frame re-reads the node's inputs and re-measures and re-places from the retained tree. Some widgets already go further: a toggle records its parts once with every animated value as a live operand, so its press, hover and drag retarget those operands and the engine animates the recorded commands in place. The window pump itself has not caught up yet. At this revision `refresh_window_scene` in `src/runner/window.rs` still re-reads every reactive input, runs layout and re-encodes the whole retained tree on every awake frame. The Hydrolysis engine contract names that as the state being replaced. Its target is that a paint-only change (a color, an opacity, a transform, the content of a text run) reaches the engine as a live operand of the recorded command, with no tree walk and no layout pass. Treat that as a goal, not a guarantee, and read the crate's `AGENTS.md` for the current state.

Structural change is handled where it happens. A `Dynamic` host rebuilds only its own child subtree, and a collection such as `ForEach` or `List` reconciles membership by item id rather than rebuilding every row. That is why a reactive collection keeps per-row state such as focus, animation and accessibility, and why wrapping a `Vec` in `watch(...)` does not. See [Lists](../03-ui/05-lists.md) and [Conditional views](../03-ui/06-conditional.md).

## Hydrolysis: a retained tree over an engine

Hydrolysis is the self-drawn backend, and it is the Android backend. It does not use the platform's widgets: it draws its components itself, and a style package decides what a button or a slider looks like. Platform objects it cannot draw, such as a system web view, are hosted beside the drawn content rather than imitated. See [Styling](../03-ui/11-styling.md).

Its render path has three layers.

1. **The retained render tree** (`src/renderer/tree`). `RenderNode::build` runs once for the window's root view and keeps the result for the window's lifetime. Each frame the window flush runs three steps: `patch` applies pending structural changes, `layout` re-reads signals, re-measures and re-places what changed, caching each container's child frames, and `flush` records each node into its program from the cached placements. A value change that alters a leaf's size therefore reflows its ancestors without calling any `body()` again.
2. **Mounts** (`src/renderer/mount`). A node that draws, places children or carries an effect owns one engine layer for its lifetime, so a frame touches only what changed. Each node records its content into a program, and the window commits the layers through one `Mount`.
3. **A painter.** The mount commits through a layer target: the Cherenkov GPU engine, or HWUI on Android.

The two painters:

| Painter | Selected by | What it does |
|---------|-------------|--------------|
| Cherenkov GPU (`cherenkov-gpu`) | always on every target but Android; on Android, the `gpu` feature (on by default) without `hwui` | The 2D engine's GPU backend, on `wgpu`, for phones, tablets and desktops |
| HWUI | the `hwui` feature, Android only | Records the layer tree into Android `RenderNode` display lists, which a Kotlin decoder replays, one `RenderNode` per layer (two for a scrolled layer) |

The painter is chosen at build time, per target, never at run time. On Android, enabling `hwui` together with `gpu` keeps HWUI as the painter while the GPU crates stay linked for hosted GPU content; an Android build with neither feature has no painter, and the build script warns about it. Cherenkov also has a CPU backend (`cherenkov-cpu`, a banded rasterizer that the `waterui-graphics` raster path uses), but at this revision Hydrolysis has no CPU layer target: the CPU and microcontroller design points in the engine contract are not yet a Hydrolysis painter. The self-drawn path is a deliberate backend, not a runtime fallback for the Apple one, and an Apple build never switches to it on failure.

Hydrolysis never draws a stand-in. A leaf nobody realized, such as a `Native<MapConfig>` with no map hook installed, is a programmer error that panics at measure or node build with a message naming the missing piece. If you see that panic, install the realization the message names.

Hosts own the event loop: the winit and browser runners, the headless runner used by tests and offscreen rendering, and the Android host each hand their own executor to the framework. A host that owns its loop must install an executor bound to that loop rather than calling a blocking `start_main_executor`; the module docs of `waterui-backend-core` explain why. [Testing](../07-tools/03-testing.md) uses the headless runner.

## Stretch axis and measurement

Every view says how it fills space with a `StretchAxis`, read from `View::stretch_axis()` or `NativeView::stretch_axis()` before `body()` runs and without an environment:

| Value | Meaning | Example |
|-------|---------|---------|
| `None` | Content-sized | Text, buttons |
| `Horizontal` | Fills width, intrinsic height | Text field |
| `Vertical` | Fills height, intrinsic width | A vertically scrolling container |
| `Both` | Fills all offered space | `Color`, shapes, `List` |
| `MainAxis` | Fills along the parent stack's main axis | `Spacer` |
| `CrossAxis` | Fills along the parent stack's cross axis | `Divider` |

Because the answer is static, a composite view whose `body` produces a stretching leaf has to declare that axis itself; a backend cannot find it out by expanding. A container claims nothing of its own and takes the union of its children's axes. Wrappers (padding, background, overlay, metadata) report their content's axis.

A parent negotiates with each child through a `ProposalSize`, per axis: `None` asks for the ideal extent, `Some(0.0)` for the minimum, `Some(f32::INFINITY)` for the maximum and `Some(v)` for "fit into `v`". The child answers with `ViewDimensions`. The three probes satisfy `min <= ideal <= max`, and the container decides allocation from them. A stretch axis never changes the answer a child gives. It only allows a container's allocation rule to give the child more than it measured, and each container applies that rule for itself.

A container is a `Layout` (`size_that_fits` and `place`). It sees each child as a `SubView`, whose `measure(proposal)` may be called many times in one pass, with different proposals, and is pure within the pass. Because of that, expensive measurements, text shaping above all, must be cached, and the cache belongs to the `SubView` implementation, never to the `Layout`. `SubView` is neither `Send` nor `Sync`: measurement runs on the thread that drives layout.

Layout semantics are frozen and specified in `docs/layout-spec.md` of the framework repository. The proposal protocol, the stack allocation rules and the leaf contracts every backend implements all live there. A backend that disagrees with the document has a bug. For the container-by-container tour, read [the Layout chapter](../03-ui/02-layout.md) and [the layout engine](03-layout-engine.md).

## Reading a render problem

A few checks cover most surprises:

- **A panic naming `Metadata<...>` or `Native<...>`**, or in a release build the same message logged and nothing drawn. No backend claimed that type. Install the hook or feature that realizes it, or use `IgnorableMetadata` if the effect is optional.
- **A value that never updates.** Something snapshotted a signal in a body. Pass the signal itself, as in `text!("{count}")`.
- **A view that stretches or collapses wrongly.** Check the `stretch_axis` of the leaf and of any composite that wraps it, then check the spec's rule for the container involved.
- **Seeing what your view expanded to.** The [inspector](../07-tools/05-inspector.md) shows the live tree; `WATERUI_DISPATCH_DEBUG` traces a `ViewDispatcher` walk.

## Next

[Layout engine](03-layout-engine.md) goes through the proposal protocol and container rules in detail, and [Backends](04-backends.md) covers what a backend has to provide.

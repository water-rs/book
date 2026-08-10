# How WaterUI renders

> **In this chapter, you will:**
>
> - Trace a view from Rust struct all the way to pixels on screen
> - Understand the difference between raw views, composite views, and configurable views
> - Learn how 128-bit type IDs enable efficient cross-language dispatch
> - See why WaterUI's signal-based reactivity avoids tree diffing entirely

You do not need this chapter to build apps. You need it to debug a view that renders wrong, to write a backend, or to understand why a modifier you invented panics with "not caught by your renderer".

Your application code produces a tree of Rust structs implementing `View`. A backend walks that tree at runtime and maps each node to something it can draw — a UIKit view, an Android `View`, a GTK widget, or a Vello scene.

## The rendering pipeline

```text
Rust view tree
     |
     v
FFI layer (C ABI / JNI)        or        Rust-side backend
     |                                          |
     v                                          v
Native backend (Swift / Kotlin)         ViewDispatcher
     |                                          |
     v                                          v
Platform UI framework                   GTK widgets, or Vello scenes
(UIKit / AppKit / Android Views)                |
     |                                          v
     v                                   GPU (wgpu) or CPU raster
Pixels on screen
```

WaterUI ships several backends, and they differ in *how* they consume the tree, never in the tree itself:

| Backend | Crate / directory | Consumes the tree via | Draws with |
|---------|-------------------|------------------------|------------|
| Apple | `backends/apple` (submodule) | C ABI | UIKit / AppKit |
| Android | `backends/android` (submodule) | JNI | Android Views |
| GTK | `backends/gtk` (`waterui-gtk`) | `ViewDispatcher` | GTK4 widgets |
| Hydrolysis | `backends/hydrolysis` | `ViewDispatcher` | Vello on `wgpu`, GPU-required |
| Dew | `backends/dew` | `ViewDispatcher` | `vello_cpu`, CPU-first, MCU-class targets |

`backends/core` (`waterui-backend-core`) holds the plumbing every Rust-side backend shares: the dispatcher, widget metrics, input, gestures, and frame signals. `backends/hydrolysis_m3` is a Material 3 theme package for Hydrolysis, not a backend of its own.

Hydrolysis and Dew are both self-drawn, and they sit at deliberately opposite design points: Hydrolysis redraws the full scene every frame on the GPU and targets 120fps-class hardware; Dew re-rasterizes only dirty bands on the CPU so peak pixel memory is one band, and targets microcontrollers with no GPU and no full-resolution framebuffer. Neither is a fallback for the other.

## View categories

Every view falls into one of three categories. These categories *are* the render loop.

### Raw views (leaf nodes)

A **raw view** maps directly to something the backend draws. It is declared with the `raw_view!` macro in `waterui-core`:

```rust,ignore
// Default stretch axis (None) -- content-sized
raw_view!(MyLeaf);

// With explicit stretch axis
raw_view!(Color, StretchAxis::Both);
raw_view!(Spacer, StretchAxis::MainAxis);
raw_view!(ScrollView, StretchAxis::Both);
```

The macro implements two traits:

1. **`NativeView`** — marks the type as a leaf, and declares its stretch axis.
2. **`View`** — implements `body()` to return `Native::new(self)`, a sentinel wrapper meaning "stop recursing, extract my data."

`Native<T>` is where recursion is *supposed* to stop. Its own `body()` panics with the type name unless a fallback view was attached with `.with_fallback(...)`. A backend that reaches `Native<T>::body()` has failed to handle a leaf it was expected to handle, and it finds out immediately.

### Composite views

A **composite view** has a `body()` returning other views. The backend evaluates `body()` and keeps walking the result until it bottoms out at a raw view.

```rust,ignore
pub trait View: 'static {
    fn body(self, env: &Environment) -> impl View;
}
```

Any struct, function, or closure implementing `View` is composite unless it went through `raw_view!` or `configurable!`. `Divider` is a good example: it has no config struct and no FFI type. Its `body()` reads the parent stack's axis out of the environment and returns a one-point `Frame`, so a backend gets a correct divider for free — though a backend is still free to recognize `Divider` and draw a native rule instead.

### Configurable views

`configurable!` bridges the two. It splits a view into a public type and a config struct, and lets a `Hook<Config>` installed in the `Environment` intercept the config before it reaches the backend:

```rust,ignore
configurable!(Button, ButtonConfig);
configurable!(Slider, SliderConfig, StretchAxis::Horizontal);
```

When the view's `body()` runs, it looks for `Hook<ButtonConfig>` in the environment. If one is present, the hook may alter or completely replace the view. If not, the config falls through to `Native::new(config)` and behaves like a raw view.

A `resolve |config, env| ...` clause can additionally rewrite the config against the environment before it becomes `Native` — that is how a config resolves theme tokens or locale-dependent labels without the backend knowing anything about either.

> **Note:** The configurable pattern is what makes theming work without forking. A library defines `Button`; downstream code replaces its rendering by installing a hook.

## View identification

The backend needs a fast way to tell what it is holding. The FFI layer uses 128-bit type IDs:

```rust,ignore
#[repr(C)]
pub struct WuiTypeId {
    pub low: u64,
    pub high: u64,
}
```

The ID is a 128-bit FNV-1a hash of the type's *name*, not its `std::any::TypeId`. This is deliberate: `TypeId` is not stable across dynamic library boundaries, but `type_name()` is. Since the preview system loads user code as a dylib, the IDs must agree across that boundary.

The backend keeps a table mapping IDs to handlers and compares in constant time:

```text
view_id == waterui_text_id()         --> create UILabel / TextView / GtkLabel
view_id == waterui_button_id()       --> create UIButton / MaterialButton / GtkButton
view_id == waterui_metadata_env_id() --> extract new environment, continue
...
otherwise                            --> call waterui_view_body(), recurse
```

## Data extraction

Once a raw view is identified, the backend extracts its data through type-specific FFI functions generated by `ffi_view!`:

```rust,ignore
ffi_view!(TextConfig, WuiText, text);
// Generates:
//   waterui_text_id()       -> WuiTypeId
//   waterui_force_as_text() -> WuiText
```

`waterui_force_as_*` performs an unchecked downcast — it trusts that the caller already compared the ID. The returned C struct carries everything the backend needs: content signals, alignment, colors, action handlers.

Note the type parameter: `waterui_text_id()` returns the ID of `Native<TextConfig>`, not of `Text`. The backend never sees `Text`; it sees the config the view resolved to.

## Metadata and modifiers

Modifiers like `.padding()`, `.opacity()`, or `.on_appear()` do not introduce new widget types. They wrap the inner view in a `Metadata<T>` node:

```text
Metadata<Opacity> {
    content: AnyView,   // the wrapped view
    value: Opacity { value: Computed<f32> }
}
```

Metadata nodes carry their own type IDs (from `ffi_metadata!`). The backend extracts the value and the inner content, applies the modifier to the platform widget, and continues into the content.

Metadata is **mandatory by default**. `Metadata<T>::body()` panics:

```text
The metadata `...::Metadata<...::Opacity>` is not caught by your renderer.
If the metadata is not essential, use `IgnorableMetadata<T>`.
```

Optional modifiers use `IgnorableMetadata<T>` instead, whose `body()` simply returns the content. That is how a platform-specific feature such as `MaterialBackground` degrades to plain content elsewhere rather than crashing. The distinction is a design decision per modifier: silently dropping a padding would be a bug, silently dropping a blur-material is not.

## The render loop

Per node, the backend does:

1. Call `waterui_view_id(view)` for the 128-bit type ID.
2. Look it up in the handler table.
3. **Handler found** (raw view or metadata) — call `waterui_force_as_*`, create or update the platform widget, and for metadata also render the `content` child.
4. **No handler** — call `waterui_view_body(view, env)` and go back to step 1 with the result.

Rust-side backends get this loop from `ViewDispatcher` in `waterui-backend-core`:

```rust,ignore
// Simplified shape of ViewDispatcher::dispatch.
pub fn dispatch<V: View>(&mut self, view: V, env: &Environment, context: C) -> R {
    if let Some(entry) = self.handlers.get(&TypeId::of::<V>()) {
        // Registered handler: extract the typed view and run it.
        return entry.invoke(&mut self.state, context, view, env);
    }
    // No handler: expand body() and recurse.
    self.dispatch(view.body(env), env, context)
}
```

Handlers register against the type the tree actually contains, which for a leaf is the `Native<Config>` wrapper:

```rust,ignore
dispatcher.register::<Native<TextConfig>>(|state, ctx, native, env| {
    // build a platform label from native.as_inner()
});
```

For concrete types the view stays on the stack and dispatch allocates nothing; only `AnyView` takes the boxed path.

> **Tip:** Set `WATERUI_DISPATCH_DEBUG=1` to have the dispatcher log an indented trace of every view type it walks. It is the fastest way to find out why your view resolved to something you did not expect.

## Reactivity and updates

The initial walk is only half the story. When data changes, WaterUI does not diff view trees. Each signal is wired directly to the one widget property it feeds:

```text
Binding<String> --> Computed<Str> --> Watcher --> UILabel.text
```

The watcher callback updates that single property. No reconciliation, no virtual DOM, no re-walk.

Collections are the same idea one level up. The `Views` trait exposes `get_id(index)`, `len()`, `get_view(index)`, and `watch(range, watcher)`; `AnyViews` erases it for the FFI. The backend receives id-level change notifications for a range and patches the platform list — inserting, removing, and reusing rows by id rather than rebuilding all of them. That is why `ForEach`/`List` over a reactive collection preserves per-row animation, focus, and accessibility state, and why `watch(...)` over a `Vec` does not.

## Stretch axis negotiation

Every view declares how it wants to fill space through `StretchAxis`:

| Value        | Behavior                                     | Example           |
|--------------|----------------------------------------------|-------------------|
| `None`       | Content-sized, uses intrinsic dimensions     | Text, Image       |
| `Horizontal` | Expands width, intrinsic height              | TextField, Slider |
| `Vertical`   | Intrinsic width, expands height              | (rare)            |
| `Both`       | Greedy, fills all available space            | Color, GpuSurface |
| `MainAxis`   | Expands along the parent stack's main axis   | Spacer            |
| `CrossAxis`  | Expands along the parent stack's cross axis  | Divider           |

Stacks use this to distribute space: a `VStack` gives leftover height to children that report `Vertical` or `MainAxis` after content-sized children are measured.

`waterui_view_stretch_axis()` exposes the value to native backends so they can lay out without evaluating the view's body.

## Measurement is parallel, and caching belongs to the leaf

Layout containers probe children through the `SubView` trait, which requires `Send + Sync` so independent children can be measured on worker threads. A leaf whose measurement genuinely must touch main-thread-only state confines that state in `waterui_core::MainThreadBound` and returns `true` from `SubView::require_main_thread()`; the executor then keeps it on the calling thread. `waterui_layout::measure_children` splits children along exactly that line, under the crate's `parallel` feature (off by default, so embedded builds stay serial).

Because containers probe the same child many times with different proposals, expensive measurements — text shaping above all — must cache. That cache belongs to the `SubView` implementation, never to the `Layout`, and because measurement can run on worker threads it must be thread-safe, not a `RefCell`.

## Performance characteristics

- **No tree diffing.** A changed signal updates its own property; cost is independent of tree size.
- **No virtual DOM.** Views are consumed by `body()`, not cloned.
- **Constant-time dispatch.** A 128-bit hash comparison, not string matching.
- **Lazy evaluation.** `body()` runs only when a backend needs that subtree.

The main cost is the initial walk, proportional to the number of visible views. After that, cost tracks the number of changed signals.

## Next: the FFI bridge

The [next chapter](02-ffi.md) opens the layer this one kept behind a curtain — how a Rust `AnyView` becomes a Swift object or a Kotlin class, and what contract a backend has to honor at startup.

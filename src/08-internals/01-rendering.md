# How WaterUI Renders

> **In this chapter, you will:**
>
> - Trace a view from Rust struct all the way to pixels on screen
> - Understand the difference between raw views, composite views, and configurable views
> - Learn how 128-bit type IDs enable efficient cross-language dispatch
> - See why WaterUI's signal-based reactivity avoids tree diffing entirely

You do not need to understand the rendering pipeline to build great apps with WaterUI. But if you have ever wondered what actually happens between writing `text("Hello")` in Rust and seeing pixels appear on an iPhone screen, this chapter is for you. Understanding these internals will help you debug rendering issues, write more efficient views, and contribute to the framework itself.

WaterUI takes a fundamentally different approach from frameworks that draw their own pixels.
Instead of maintaining a virtual DOM or a custom render tree, WaterUI compiles your
Rust view declarations into a tree that native backends walk at runtime, mapping each node
to a platform widget.

## The Rendering Pipeline

The high-level data flow looks like this:

```text
Rust View Tree
     |
     v
FFI Layer (C ABI / JNI)        or        Rust-side backend
     |                                          |
     v                                          v
Native Backend (Swift / Kotlin)        Hydrolysis renderer
     |                                          |
     v                                          v
Platform UI Framework                  Vello + wgpu on GPU
(UIKit / AppKit / Android Views)
     |
     v
Pixels on Screen
```

Your application code produces a tree of Rust structs that implement the `View` trait.
For the Apple and Android backends, the FFI layer exposes this tree through a stable C ABI
(or JNI on Android), and Swift or Kotlin walks the tree to create UIKit/AppKit/Android
widgets. For Rust-side backends like Hydrolysis, the dispatcher walks the same tree without
crossing a language boundary.

## View Categories

Every view in WaterUI falls into one of three categories. Understanding these categories is the key to understanding the render loop.

### Raw Views (Leaf Nodes)

A **raw view** (also called a *native view* or *leaf view*) maps directly to a platform
widget. Examples include `Text`, `Button`, `Toggle`, `Slider`, and `Color`. These
views are marked with the `raw_view!` macro in `waterui-core`:

```rust,ignore
// Default stretch axis (None) -- content-sized
raw_view!(Text);

// With explicit stretch axis
raw_view!(Color, StretchAxis::Both);
raw_view!(Spacer, StretchAxis::MainAxis);
```

The macro implements two traits:

1. **`NativeView`** -- marks the type as a leaf that backends should handle directly.
2. **`View`** -- implements `body()` to return `Native::new(self)`, a sentinel wrapper
   that tells the FFI layer "stop recursing, extract my data."

When the backend encounters a `Native<T>` wrapper, it knows to call the corresponding
`waterui_force_as_*` function to extract the view's data and create a platform widget.

### Composite Views

A **composite view** has a `body()` method that returns other views. When the backend
encounters a composite view, it calls `waterui_view_body()` to evaluate `body()` and
then continues walking the result. This recursion bottoms out when it reaches a raw
view.

```rust,ignore
pub trait View: 'static {
    fn body(self, env: &Environment) -> impl View;
}
```

Any closure, struct, or function that implements `View` is a composite view unless it
uses `raw_view!` or `configurable!`.

### Configurable Views

A third category bridges the gap. The `configurable!` macro creates views whose
configuration can be intercepted by `Hook`s installed in the `Environment`:

```rust,ignore
configurable!(Button, ButtonConfig);
configurable!(Slider, SliderConfig, StretchAxis::Horizontal);
```

When a configurable view's `body()` runs, it checks the environment for a matching
`Hook<Config>`. If found, the hook can alter or replace the view entirely. If no hook
is present, the view falls through to `Native::new(config)` -- behaving like a raw view.

> **Note:** The configurable pattern is what makes WaterUI's theming system so powerful. A library can define a `Button`, and downstream code can completely replace its rendering -- without forking the library.

## View Identification

With three categories of views in play, the backend needs a fast way to identify what it is looking at. The FFI layer solves this with 128-bit type IDs.

The function `waterui_view_id()` returns a `WuiTypeId` for any `AnyView` pointer:

```rust,ignore
#[repr(C)]
pub struct WuiTypeId {
    pub low: u64,
    pub high: u64,
}
```

The ID is computed from the type's name using a 128-bit FNV-1a hash. This choice is
deliberate: Rust's `std::any::TypeId` is not stable across dynamic library boundaries,
but `type_name()` is. Since the preview system loads user code as a dylib, WaterUI
needs IDs that remain consistent regardless of how the code was loaded.

The backend maintains a lookup table mapping `WuiTypeId` values to handler functions.
When it receives a view, it compares the ID in O(1) time:

```text
view_id == waterui_text_id()       --> create UILabel / TextView / GtkLabel
view_id == waterui_button_id()     --> create UIButton / MaterialButton / GtkButton
view_id == waterui_metadata_env_id() --> extract new environment, continue
...
otherwise                          --> call waterui_view_body(), recurse
```

## Data Extraction

Once a raw view is identified, the backend extracts its data using type-specific FFI
functions. These are generated by the `ffi_view!` macro:

```rust,ignore
ffi_view!(TextConfig, WuiText, text);
// Generates:
//   waterui_text_id()       -> WuiTypeId
//   waterui_force_as_text() -> WuiText
```

The `waterui_force_as_*` function performs an unchecked downcast -- it trusts that the
caller already verified the type ID. The returned C struct contains all the data the
backend needs to create the widget: text content, font, color signals, action handlers,
and so on.

## Metadata and Modifiers

Modifiers like `.padding()`, `.opacity()`, or `.on_appear()` do not create new widget
types. Instead, they wrap the inner view in a `Metadata<T>` node:

```text
Metadata<Opacity> {
    content: AnyView,   // the wrapped view
    value: Opacity { value: Computed<f32> }
}
```

The backend identifies metadata nodes by their own type IDs (generated by
`ffi_metadata!`). When it encounters one, it extracts the metadata value and the
inner content view, applies the modifier to the platform widget, and continues
rendering the content.

Some metadata types are marked as `IgnorableMetadata<T>`. If a backend does not
recognize the metadata, it can safely skip the modifier and render just the inner
content. This allows platform-specific features (like `MaterialBackground` on Apple)
to degrade gracefully on other platforms.

## The Render Loop

Putting it all together, here is the algorithm the backend follows for each view node:

1. Call `waterui_view_id(view)` to get the 128-bit type ID.
2. Look up the ID in the handler table.
3. **If a handler is found** (raw view or metadata):
   - Call the corresponding `waterui_force_as_*` to extract data.
   - Create or update the platform widget.
   - For metadata, also render the `content` child recursively.
4. **If no handler is found** (composite view):
   - Call `waterui_view_body(view, env)` to evaluate `body()`.
   - Go to step 1 with the result.

Rust-side backends formalize this pattern in `ViewDispatcher` from `waterui-backend-core`:

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

> **Tip:** If you are writing a Rust-side backend, `ViewDispatcher` handles this loop for you. You only need to call `register::<MyView>(handler)` for each native view type you support.

## Reactivity and Updates

The initial render is only half the story. What happens when data changes?

WaterUI does not diff entire view trees. Instead, it relies on fine-grained
reactivity exposed through `Binding<T>` and `Computed<T>`. When a binding
changes, only the computed signals that depend on it fire. Each signal is
connected to a specific widget property through a watcher:

```text
Binding<String> --> Computed<Str> --> Watcher --> UILabel.text
```

The watcher callback runs on the main thread, updating the single widget property that
changed. There is no tree reconciliation, no virtual DOM diff, and no full re-render.

For collection views (lists), the `Views` trait provides an `AnyViews` abstraction
with a `watch()` method. The backend receives fine-grained change notifications
(insertions, deletions, moves) and updates the platform list accordingly.

## Stretch Axis Negotiation

Every view declares how it wants to fill available space through `StretchAxis`:

| Value        | Behavior                                     | Example        |
|-------------|----------------------------------------------|----------------|
| `None`      | Content-sized, uses intrinsic dimensions     | Text, Image    |
| `Horizontal`| Expands width, intrinsic height              | TextField, Slider |
| `Vertical`  | Intrinsic width, expands height              | (rare)         |
| `Both`      | Greedy, fills all available space            | Color, GpuSurface |
| `MainAxis`  | Expands along the parent stack's main axis   | Spacer         |
| `CrossAxis` | Expands along the parent stack's cross axis  | Divider        |

Stack layouts use this information to distribute space. In a `VStack`, children with
`StretchAxis::Vertical` or `StretchAxis::MainAxis` share remaining vertical space
after content-sized children are measured.

The FFI function `waterui_view_stretch_axis()` exposes this value to native backends
so they can perform layout calculations without evaluating the full view body.

## Performance Characteristics

Several design decisions contribute to rendering performance:

- **No tree diffing**: Signal-based updates are O(1) per changed property.
- **No virtual DOM**: Views are consumed (moved) during `body()`, not cloned.
- **O(1) type dispatch**: 128-bit hash comparison avoids string matching.
- **Native widgets**: The platform's own compositor handles drawing and compositing.
- **Lazy evaluation**: `body()` is only called when the backend actually needs the
  view tree. Composite views deeper than the visible hierarchy are never evaluated.

The main cost center is the initial view tree walk, which is proportional to the
number of visible views. Subsequent updates are proportional only to the number of
changed signals, not the tree size.

## What's Next

Now that you understand how views become pixels, the [next chapter](02-ffi.md) dives deeper into the FFI bridge -- the layer that makes it possible for Rust structs to become Swift objects and Kotlin classes.

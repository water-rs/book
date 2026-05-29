# The FFI Bridge

> **In this chapter, you will:**
>
> - Understand how the `export!()` macro wires up the application entry points
> - Learn the initialization sequence that every native backend follows
> - See how theme signals are injected across the FFI boundary
> - Know the macros that generate type-safe FFI bindings

WaterUI applications are written in Rust, but they render through platform-native
backends written in Swift, Kotlin, or GTK4. The FFI (Foreign Function Interface) layer
is the bridge between these worlds -- a stable C ABI contract that both sides agree on. If you ever need to add a new native view, debug a cross-language issue, or understand why a type needs `#[repr(C)]`, this chapter has the answers.

## Overview

The `waterui-ffi` crate (`ffi/` directory) serves as the translation layer between
Rust types and C-compatible representations. It defines:

- **`IntoFFI`** -- converts Rust types to FFI-safe representations.
- **`IntoRust`** -- converts FFI types back to Rust (unsafe, ownership transfer).
- **`OpaqueType`** -- marks types as opaque pointers across the boundary.

The crate operates in `#![no_std]` mode to minimize dependencies, using `alloc` for
heap allocations.

## The `export!()` Macro

Every generated FFI companion crate invokes the `export!()` macro exactly once.
Your application crate exposes `pub fn app(env: Environment) -> App`; the CLI
generated companion depends on that crate and owns the C entry points that
native backends call:

```rust,ignore
waterui_ffi::export!();
```

The macro expands to three key functions. Let's look at each one.

### `waterui_init()`

```rust,ignore
pub unsafe extern "C" fn waterui_init() -> *mut WuiEnv
```

Called once at application startup. It:

1. Initializes the platform logging system (`tracing` with OS-specific backends):
   - Apple: `tracing-oslog` with subsystem `dev.waterui`
   - Android: `tracing-android` with tag `WaterUI`
   - Other: `tracing-subscriber` with `fmt` output
2. Sets up a panic hook that forwards panics to `tracing::error!`.
3. Initializes the async executor (`native-executor`).
4. Optionally initializes the shared GPU context.
5. Creates a default `Environment` and returns it as an opaque pointer.

### `waterui_app()`

```rust,ignore
pub unsafe extern "C" fn waterui_app(env: *mut WuiEnv) -> WuiApp
```

Takes ownership of the environment (which the native side has enriched with theme
data) and calls the user's `app(env: Environment) -> App` function. Returns a
`WuiApp` struct containing the window array and the environment pointer.

### `JNI_OnLoad` (Android only)

```rust,ignore
extern "system" fn JNI_OnLoad(vm: *mut c_void, _reserved: *mut c_void) -> i32
```

Initializes the Android NDK context and JNI module. This is generated only when
targeting Android.

## Initialization Sequence

Every native backend follows the same protocol to start a WaterUI application. Understanding this sequence is essential if you are writing or debugging a backend:

```text
1. waterui_init()                      --> *mut WuiEnv
2. waterui_theme_install_color_scheme() --> install light/dark signal
3. waterui_theme_install_color()        --> install color slots (x8)
4. waterui_theme_install_font()         --> install font slots (x6)
5. waterui_app(env)                     --> WuiApp { windows, env }
6. Render loop begins
```

Steps 2-4 inject reactive theme signals that track platform appearance changes.
The environment carries these signals into the view tree where colors and fonts
resolve automatically.

> **Note:** The ordering matters. Theme signals must be installed *before* calling `waterui_app()`, because the user's `app()` function may immediately reference theme tokens such as `theme::color::Foreground` or text presets such as `.body()`, which need the theme to be in place.

## Theme Installation APIs

### Color Scheme

```c
// Create a reactive color scheme signal
WuiComputed_ColorScheme* scheme =
    waterui_computed_color_scheme_constant(WuiColorScheme_Dark);

// Install it into the environment
waterui_theme_install_color_scheme(env, scheme);
```

The `WuiColorScheme` enum has two variants: `Light` (0) and `Dark` (1). Native
backends typically create a callback-driven signal that tracks the system appearance.

### Color Slots

WaterUI defines 8 semantic color slots:

| Slot                | Value | Purpose                        |
|---------------------|-------|--------------------------------|
| `Background`        | 0     | Primary background             |
| `Surface`           | 1     | Elevated surfaces (cards)      |
| `SurfaceVariant`    | 2     | Alternate surfaces             |
| `Border`            | 3     | Borders and dividers           |
| `Foreground`        | 4     | Primary text and icons         |
| `MutedForeground`   | 5     | Secondary/dimmed text          |
| `Accent`            | 6     | Interactive element highlights |
| `AccentForeground`  | 7     | Text on accent backgrounds     |

Each slot is installed individually:

```c
WuiComputed_ResolvedColor* fg = create_foreground_signal();
waterui_theme_install_color(env, WuiColorSlot_Foreground, fg);
```

### Font Slots

WaterUI defines 6 font slots:

| Slot           | Value | Purpose          |
|----------------|-------|------------------|
| `Body`         | 0     | Body text        |
| `Title`        | 1     | Titles           |
| `Headline`     | 2     | Headlines        |
| `Subheadline`  | 3     | Subheadlines     |
| `Caption`      | 4     | Captions         |
| `Footnote`     | 5     | Footnotes        |

```c
WuiComputed_ResolvedFont* body = create_body_font_signal();
waterui_theme_install_font(env, WuiFontSlot_Body, body);
```

### Querying Theme Values

Native code can also read theme values back:

```c
WuiComputed_ResolvedColor* accent = waterui_theme_color(env, WuiColorSlot_Accent);
// Use the signal...
waterui_drop_computed_resolved_color(accent);  // Clean up
```

## View Traversal

Once the app is created, the backend walks the view tree using these functions:

### `waterui_view_id()`

```c
WuiTypeId waterui_view_id(const WuiAnyView* view);
```

Returns the 128-bit type ID of a view. The backend compares this against known IDs
to determine how to render the view.

The `WuiTypeId` uses FNV-1a hashing of the Rust type name, ensuring stability across
dynamic library boundaries (required for the preview/hot-reload system).

### `waterui_view_body()`

```c
WuiAnyView* waterui_view_body(WuiAnyView* view, WuiEnv* env);
```

Evaluates a composite view's `body()` method, consuming the view pointer and
returning a new view. The backend calls this when it encounters a view type it
does not recognize.

### `waterui_force_as_*()` Functions

For each raw view type, the `ffi_view!` macro generates a force-cast function:

```c
WuiText waterui_force_as_text(WuiAnyView* view);
WuiButton waterui_force_as_button(WuiAnyView* view);
```

These functions perform an unchecked downcast. The caller must have already
verified the type ID. The returned C struct contains all data needed to create
the platform widget.

Similarly, `ffi_metadata!` generates functions for metadata types:

```c
WuiMetadataOpacity waterui_force_as_metadata_opacity(WuiAnyView* view);
WuiMetadataBorder waterui_force_as_metadata_border(WuiAnyView* view);
```

And `ffi_ignorable_metadata!` for platform-optional metadata:

```c
WuiIgnorableMetadataMaterialBackground
    waterui_force_as_ignorable_metadata_material_background(WuiAnyView* view);
```

### `waterui_view_stretch_axis()`

```c
WuiStretchAxis waterui_view_stretch_axis(const WuiAnyView* view);
```

Returns the view's stretch axis without evaluating its body. Used by layout
containers to determine how children should fill available space.

## FFI Macros

The FFI layer relies on several code-generation macros to reduce boilerplate and prevent mistakes. Here is a quick reference for each one.

### `ffi_safe!`

Declares types as directly FFI-compatible (identity conversion):

```rust,ignore
ffi_safe!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64, bool);
```

### `opaque!`

Creates an opaque wrapper type with pointer-based transfer:

```rust,ignore
opaque!(WuiEnv, waterui::Environment, env);
// Generates: struct WuiEnv(Environment)
// Also generates: waterui_drop_env() for cleanup
```

### `ffi_view!`

Generates ID and force-cast functions for native view types:

```rust,ignore
ffi_view!(TextConfig, WuiText, text);
// C-API: waterui_text_id(), waterui_force_as_text()
// JNI:   WatcherJni.textId(), WatcherJni.forceAsText()
```

### `ffi_metadata!`

Same pattern for `Metadata<T>` wrappers:

```rust,ignore
ffi_metadata!(Opacity, WuiMetadataOpacity, opacity);
// C-API: waterui_metadata_opacity_id(), waterui_force_as_metadata_opacity()
```

### `into_ffi!`

Derives `IntoFFI` for structs and enums with field-by-field conversion:

```rust,ignore
into_ffi!(ListConfig, pub struct WuiList {
    contents: *mut WuiAnyViews,
});
```

## FFI Boundary Safety

All FFI entry points are wrapped in `ffi_boundary()`, which catches panics and
converts them to tracing errors instead of unwinding across the C boundary:

```rust,ignore
pub fn ffi_boundary<T>(name: &'static str, f: impl FnOnce() -> T) -> Option<T> {
    match std::panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => Some(value),
        Err(_) => {
            tracing::error!(boundary = name, "panic crossing FFI boundary");
            None
        }
    }
}
```

This prevents undefined behavior from Rust panics unwinding through C frames.

> **Warning:** If you see `panic crossing FFI boundary` in your logs, it means a Rust function panicked during an FFI call. Check the surrounding log output for the actual panic message -- it will point you to the root cause.

## C Header Generation

The C header file `ffi/waterui.h` is checked into the WaterUI repository and is
generated automatically. You must never write or edit it by hand. If you are
contributing to WaterUI itself and have modified any FFI function signature,
regenerate the header from inside the upstream `waterui` checkout:

```bash
cargo run --bin generate_header --features cbindgen --manifest-path ffi/Cargo.toml
```

CI verifies that the checked-in header matches the generated output, so a missed
regeneration will fail your pull request rather than slip through. Application
authors who only consume WaterUI never need to run this command.

## Android JNI

On Android, the same macros generate JNI entry points alongside the C API. The
`ffi_view!` macro produces both:

```rust,ignore
// C-API (Apple/GTK)
extern "C" fn waterui_force_as_text(view: *mut WuiAnyView) -> WuiText;
extern "C" fn waterui_text_id() -> WuiTypeId;

// JNI (Android)
extern "system" fn Java_dev_waterui_android_ffi_WatcherJni_textId(...) -> jobject;
extern "system" fn Java_dev_waterui_android_ffi_WatcherJni_forceAsText(...) -> jobject;
```

The JNI module (`ffi/src/jni/`) provides conversion utilities between Rust structs
and Java objects, caching JNI class references for performance.

## Adding a New View to FFI

If you are extending WaterUI itself with a new native view, here is the checklist:

1. Define the Rust view type with `raw_view!` or `configurable!` in its component crate.
2. Define a `#[repr(C)]` FFI struct (e.g., `WuiMyView`) in `ffi/src/components/`.
3. Implement `IntoFFI` for the view type.
4. Call `ffi_view!(MyView, WuiMyView, my_view)` to generate the entry points.
5. Regenerate the C header.
6. Implement the handler in the Apple Swift package and the Android Kotlin runtime so each backend can render the new view.

The header regeneration command will fail if any FFI type is not `#[repr(C)]`
compatible, catching errors at build time rather than runtime.

## What's Next

The FFI bridge gets data across the language boundary, but it does not decide where things go on screen. The [next chapter](03-layout-engine.md) explores WaterUI's two-phase layout engine -- how containers negotiate sizes with their children and place them within the final bounds.

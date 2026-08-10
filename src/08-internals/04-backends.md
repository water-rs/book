# Backend architecture

> **In this chapter, you will:**
>
> - See how each backend maps a Rust view tree onto its platform
> - Understand the shared contract every backend implements
> - Learn why WaterUI ships two self-drawn renderers with opposite designs
> - Follow the steps for adding a new backend

A backend turns the same Rust view tree into whatever the target platform
understands. Apple and Android bridge to native widgets; GTK4 bridges to GTK
widgets; Hydrolysis and Dew draw the pixels themselves.

> **Note:** You do not need any of this to use WaterUI. This chapter is for
> contributors, backend authors, and the curious.

## Native bridge first

WaterUI's rule is that a semantic component gets a **native bridge** on every
platform with a suitable platform primitive, and a shared self-drawn realization
where none exists. "Native" means coupled to the platform's own object model,
lifecycle, accessibility, input, and graphics pipeline — not merely "a library
that happens to ship with the OS." Bundling a portable engine and calling it
native does not qualify.

The self-drawn realization is a deliberate backend, never a runtime fallback. A
failed native bridge is an error to fix, not a cue to silently swap renderers.

## The backend contract

Every backend must:

1. Call `waterui_init()` to initialize the Rust runtime and get an `Environment`.
2. Install theme signals (color scheme, colors, fonts) into that environment.
3. Call `waterui_app(env)` to obtain the application's window tree.
4. Walk the tree, dispatching each node by its `WuiTypeId`.
5. Subscribe to reactive signals and update widgets when values change.
6. Drive the application lifecycle: windows and the event loop.

`waterui_init` and `waterui_app` come from `waterui_ffi::export!()`. That macro
lives in the FFI companion crate the `water` CLI generates — application crates
neither declare `waterui-ffi` nor call `export!()` themselves.

The `waterui-backend-core` crate holds what Rust-side backends share:
`ViewDispatcher` for type-based dispatch, plus animation, gesture, input,
scroll, frame-signal, and time modules.

## Apple backend (Swift)

**Location**: `backends/apple/` (git submodule)

A Swift Package targeting UIKit (iOS/tvOS), AppKit (macOS), and WatchKit
(watchOS). It is the most mature backend and the reference implementation.

```text
Rust library (.dylib / .a)
     |
     C ABI (waterui.h)
     |
Swift package (WaterUI)
     |
     UIKit / AppKit widgets
```

The Swift side walks the Rust view tree and creates the corresponding platform
views:

| Rust view         | iOS                       | macOS                                    |
|-------------------|---------------------------|------------------------------------------|
| `Text`            | `UILabel`                 | `NSTextField` (label mode)               |
| `Button`          | `UIButton`                | `NSButton`                               |
| `Toggle`          | `UISwitch`                | `NSSwitch`                               |
| `TextField`       | `UITextField`             | `NSTextField`                            |
| `ScrollView`      | `UIScrollView`            | `NSScrollView`                           |
| `NavigationStack` | `UINavigationController`  | custom `NSView` stack with `NSWindow` toolbar accessories |
| `GpuSurface`      | `CAMetalLayer`            | `CAMetalLayer`                           |

AppKit has no navigation controller, so the macOS stack is built from `NSView`
containers and drives the window's title, leading, trailing, and search
accessories directly. This is the "asymmetries are documented, not faked"
principle in practice.

### Reactive integration

The backend subscribes to WaterUI signals through FFI watchers. A change in
Rust invokes a C callback that Swift registered:

```text
Binding<Str> changes
  --> Computed<Str> fires
    --> C callback invoked
      --> Swift closure updates UILabel.text
```

Only the bound property is touched, and the update lands on the main thread.

### Theme injection

The backend maps system appearance and typography into WaterUI's theme slots:

```swift
// Pseudocode
let colorSchemeSignal = waterui_computed_color_scheme_new { watcher in
    // Track UITraitCollection.userInterfaceStyle
    // Call waterui_call_watcher_color_scheme(watcher, .dark) on change
}
waterui_theme_install_color_scheme(env, colorSchemeSignal)
```

Each semantic slot (`Foreground`, `Background`, `Surface`, `Accent`, and the
rest) resolves to a platform dynamic color, so ordinary view code adapts to
light/dark mode with no extra work. Backends read these slots rather than
hard-coding `.label` or `.systemBackground`: getting defaults right is the
backend's job, not the view author's.

### Build integration

Drive the backend through the `water` CLI, never `xcodebuild` or `swift build`
directly. `water run --platform ios` cross-compiles the Rust staticlib for the
target triple, hands the path to the Swift package, signs, and deploys to the
chosen simulator or device. If a build step the CLI cannot express turns up,
file an issue against `cli/` rather than scripting around it.

## Android backend (Kotlin/JNI)

**Location**: `backends/android/` (git submodule)

```text
Rust library (.so)
     |
     JNI
     |
Kotlin runtime (dev.waterui.android)
     |
     Android View hierarchy
```

The Kotlin runtime is organized into `components`, `ffi`, `layout`, `reactive`,
and `runtime` packages.

### JNI bridge

On Android, the FFI macros emit JNI entry points beside the C functions. For a
view registered as `ffi_view!(TextConfig, WuiText, text)`:

```rust,ignore
extern "system" fn Java_dev_waterui_android_ffi_WatcherJni_textId(...) -> jobject;
extern "system" fn Java_dev_waterui_android_ffi_WatcherJni_forceAsText(...) -> jobject;
```

The identifier is lower-camelized for the `*Id` function and upper-camelized
after the `forceAs` prefix. The JNI module (`ffi/src/jni/`) caches class
references at `JNI_OnLoad`, converts `#[repr(C)]` structs into Java objects
field by field, and passes Rust pointers as `jlong`.

`export!()` generates `JNI_OnLoad` in the companion crate:

```rust,ignore
extern "system" fn JNI_OnLoad(vm: *mut c_void, _reserved: *mut c_void) -> i32 {
    unsafe { waterui_ffi::__jni_init(vm) }
}
```

Targeting Android without the `android-jni` feature is a compile error rather
than a silent no-op.

### Gradle project

The Gradle project holds the `runtime` Kotlin library and its JNI bindings. You
do not invoke Gradle or `adb`; `water run --platform android` cross-compiles for
the Android targets, copies the `.so` into `jniLibs/`, runs the embedded Gradle
wrapper, and installs and launches the app.

## GTK4 backend

**Location**: `backends/gtk/`

The native Linux bridge, built on `gtk4-rs`. Beyond the widget mapping it hosts
the embedded browser components: the system WebKitGTK view, WaterUI's bundled
WPE runtime, and the CEF-based Chromium runtime, selected through the
`webview_backend` key in `Water.toml`.

Select it with `water run --platform linux --backend gtk4`, or scaffold with
`water create <name> --backends gtk4`.

## Two self-drawn renderers

Hydrolysis and Dew share `waterui-core`, reactivity, layout, and text, and
diverge only in render strategy — deliberately, at opposite ends of the hardware
range. Neither is converging on the other.

| | Hydrolysis | Dew |
|---|---|---|
| Target | High-end desktop, mobile, web | MCU-class embedded (ESP32-S3, ESP32-C3) |
| Rasterization | GPU-required (`vello` on `wgpu`) | CPU-first (`vello_cpu` sparse strips), GPU optional |
| Frame strategy | Full-scene redraw, game-engine style | Dirty regions only, sliced into bands |
| Peak pixel memory | Full frame | One band |
| Frame rate | High refresh, explicitly requested | 30/60fps, power-frugal |
| Dependency graph | Modern GPU + multi-core CPU | Lean, feature-gated for firmware |

### Hydrolysis

**Location**: `backends/hydrolysis/`, with `backends/hydrolysis_m3/` supplying
the Material 3 skin.

```text
Rust view tree
     |
     ViewDispatcher (Rust)        ----> accesskit a11y tree
     |
     Hydrolysis widgets (text, layout, scroll, gestures, ...)
     |
     vello + parley scene
     |
     wgpu device + GPU surface
```

Rules that hold when you author GPU-backed components on top of it:

- **GPU only, no CPU fallback.** There is no software rasterization path.
  Production surfaces reject software and noop `wgpu` adapters; the
  `WATER_HYDROLYSIS_FORCE_FALLBACK_ADAPTER` environment variable exists for
  one-off diagnostics, not for shipping.
- **Never read render targets back to CPU memory** on the runtime render path.
  Offscreen capture for tests has its own entry points.
- **One `GpuView` per `GpuSurface`.** `GpuSurface::new(renderer)` owns that
  `GpuView` for the surface's lifetime, and persistent GPU resources belong in
  `GpuView::setup()` — not in a cache that outlives the surface.
- **No damage tracking.** The scene is redrawn rather than invalidated by
  region. Adding dirty-rectangle logic here would contradict the design.

Starting a Hydrolysis app installs the Material 3 defaults onto the app's
environment after the app is constructed:

```rust,ignore
let env = Environment::new();
let mut app = my_crate::app(env);
hydrolysis_m3::install_defaults(&mut app.env);
hydrolysis::run(app);
```

The CLI generates exactly this for managed Hydrolysis backends.

#### Accessibility as a build output

With the `accessibility` feature enabled, every Hydrolysis component emits an
`accesskit` tree as a first-class artifact rather than a retrofit.
`waterui-testing` consumes that tree directly, so a component that cannot be
covered by an accessibility query is failing its design contract, not just a CI
lint.

### Dew

**Location**: `backends/dew/`

```text
WaterUI view tree
   |  dispatch + waterui-layout measure/place
   v
DisplayList        retained draw commands (kurbo paths, peniko brushes)
   |  diff against previous frame -> dirty rects
   v
BandScheduler      dirty rects -> row slices no taller than band_height
   |
Painter            vello_cpu rasterizes each band into a scratch pixmap
   |
DisplayFlush       the only platform-specific piece: in-memory buffer,
                   simulator window, or RGB565 panel stream
```

The screen never has to exist as a full-resolution framebuffer, which is what
makes the backend viable on a microcontroller driving an SPI/QSPI panel. Dew is
`std`-based through its embedded RTOS rather than bare-metal `no_std`, and
firmware builds strip GPU, widget, and gesture features from the dependency
graph.

The whole flow runs on the desktop without cross-compiling:

```bash
cargo run -p waterui-dew --example watch_sim --features embedded-simulator
```

`waterui_dew::render_view_png(builder, env, width, height)` renders one frame
headlessly for snapshot tests. Views Dew does not yet support panic rather than
render something wrong.

## View dispatch

Rust-side backends route views through `ViewDispatcher` from
`waterui-backend-core`:

```rust,ignore
use waterui_backend_core::ViewDispatcher;
use waterui_core::{Environment, components::Native};

let mut dispatcher: ViewDispatcher<State, RenderContext, Widget> = ViewDispatcher::new();

dispatcher.register::<Native<TextConfig>>(|state, ctx, view, env| {
    // Build the backend's text widget from the config.
});

dispatcher.register::<Native<ButtonConfig>>(|state, ctx, view, env| {
    // Build the backend's button widget.
});

let widget = dispatcher.dispatch(my_view, &env, context);
```

`dispatch` is the render loop:

1. Look up the view's `TypeId` in the handler table.
2. If a handler is registered, run it — the view stays on the stack, no
   allocation.
3. Otherwise evaluate `body()` and recurse on the result.

That is the same algorithm the Apple and Android backends implement in Swift and
Kotlin, minus the language boundary.

### Debug tracing

`WATERUI_DISPATCH_DEBUG=1` logs the dispatch tree as views are matched:

```text
[dispatch] Native<TextConfig>
[dispatch] Metadata<Padding>
[dispatch]   Native<ButtonConfig>
```

Any view that falls through to `body()` and never reaches a registered handler
is a backend gap worth filing.

## Adding a new backend

1. **Create the crate** in `backends/your-backend/`.
2. **Depend on `waterui-backend-core`** for `ViewDispatcher` and the shared
   interaction types.
3. **Register handlers** for each native view type you support:
   ```rust,ignore
   dispatcher.register::<Native<TextConfig>>(|state, ctx, view, env| {
       // Create your platform's text widget
   });
   ```
4. **Handle metadata** the same way:
   ```rust,ignore
   dispatcher.register::<Metadata<Opacity>>(|state, ctx, meta, env| {
       // Apply opacity, then render meta.content
   });
   ```
5. **Implement the lifecycle**: window creation, event loop, signal-driven
   updates.
6. **Install theme signals**, mapping your platform's appearance system onto
   WaterUI's color and font slots.

Two constraints apply throughout. A bridge may only make reachable the platform
code the selected WaterUI features actually need — hiding a whole framework
behind FFI or broad keep rules defeats dead-stripping and inflates every
packaged app. And unused WaterUI features must drop their Rust code, platform
code, resources, and transitive dependencies from the artifact, which means new
backend dependencies are feature-gated and measured.

For FFI backends, you write the view walker in the target language against the
generated C header or JNI functions instead of registering Rust handlers.

## Backend status

| Backend       | Path                      | Notes                                                     |
|---------------|---------------------------|-----------------------------------------------------------|
| Apple         | `backends/apple/`         | Submodule. UIKit/AppKit bridge; reference implementation.  |
| Android       | `backends/android/`       | Submodule. Android View bridge over JNI.                   |
| GTK4          | `backends/gtk/`           | Linux bridge; also hosts the WebKitGTK/WPE/CEF web views.  |
| Hydrolysis    | `backends/hydrolysis/`    | GPU self-drawn renderer; drives `waterui-testing`.         |
| Hydrolysis-M3 | `backends/hydrolysis_m3/` | Material 3 skin for Hydrolysis.                            |
| Dew           | `backends/dew/`           | CPU self-drawn renderer for MCU-class targets.             |
| Backend core  | `backends/core/`          | Shared dispatch, gesture, scroll, animation, frame timing. |

Component coverage moves too fast to freeze in a matrix. Run your view against a
target and read the dispatch trace instead.

## What's next

The [next chapter](05-library-authoring.md) turns from extending the framework
downward to extending it outward: authoring reusable WaterUI component crates.

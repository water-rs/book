# Backend architecture

> **In this chapter, you will:**
>
> - See how the Apple backend maps each realized view to one `NSView` or `UIView`
> - Understand why Hydrolysis is the only self-drawn backend and what draws for it
> - Learn what `waterui-backend-core` shares between backends
> - Know which platforms are tier 1, 2 and 3, and where the experimental backends live

A backend turns the same Rust view tree into pixels and input on one platform. WaterUI has two mainline backends with opposite designs. The Apple backend wraps the platform's own UI frameworks. Hydrolysis draws everything itself. Your view code is identical for both:

```rust
use waterui::prelude::*;

fn greeting() -> impl View {
    text!("Hello, 你好, こんにちは, 안녕하세요")
}
```

> **Note:** You do not need any of this to use WaterUI. This chapter is for contributors, backend authors, and the curious.

## One semantic, two realizations

A component's semantics and API are defined once, in Rust. Each backend realizes that semantic with its platform's look:

- The **Apple backend** uses AppKit and UIKit: the platform's control where it carries the whole semantic, otherwise a composition of AppKit or UIKit views written in Rust inside the backend.
- **Hydrolysis** draws the component through a *style package* that implements the widget and theme contracts.

Style is an attribute, not a separate component, so a backend never grows parallel types such as a checkbox toggle next to a switch toggle. The visual comes from the backend's platform default, the theme tokens in the environment, or a component's `.style(...)` call. The [styling chapter](../03-ui/11-styling.md) covers the author-facing side.

Self-drawn realization is a deliberate backend, never a runtime fallback. A native bridge that fails is an error to fix; it never silently switches renderers.

"Native" means platform-coupled: the implementation projects WaterUI semantics directly into the platform's object model, lifecycle, accessibility, input, graphics or media pipeline. A package that supplies a largely self-contained, portable engine, such as a bundled media player or map engine, is not native even when it ships on that platform.

## The Apple backend

The Apple backend is the `waterui-apple` crate in `backends/apple`. It is Rust on [`objc2`](https://crates.io/crates/objc2) that projects WaterUI semantics directly into AppKit on macOS and UIKit on iOS. There is no C header and no FFI seam between the view tree and the native views: one `NSView` or `UIView` is created per realized view, in Rust.

It has two Rust layers over the platform frameworks, plus a thin Swift package for embedding.

```text
Your Rust app (waterui views)
        |
waterui-apple          component ports, layout bridging, dispatch,
        |              windows and scenes, preview, capture
cocoa-ui               safe Rust API over AppKit / UIKit (objc2)
        |
AppKit / UIKit
```

**`waterui-apple`** holds one port per component under `src/components/`. A dispatch table maps each leaf view type to the port that realizes it. The table is built once at startup and is immutable afterwards. A view with no registered handler expands through its `body()` and is dispatched again, the same rule the shared `ViewDispatcher` in `waterui-backend-core` documents.

**`cocoa-ui`** is a separate crate: a safe Rust API over AppKit and UIKit that knows nothing about WaterUI. It owns the application and its delegate, windows and scenes, a host view whose layout, resizing and hit testing are Rust closures, and the system notifications around them. Its public API contains no `unsafe` function and no `msg_send!`. Main-thread types need a `MainThreadMarker`, native objects are owned through `Retained`, and a panic in a closure the frameworks call is logged and aborts the process rather than unwinding into Objective-C.

**The application entry** is Rust too. The `waterui_apple::export_app!` macro, invoked in a companion crate the `water` CLI generates, expands to the exported `waterui_apple_main` function, and the generated executable's `main` calls it. That function ends in `waterui_apple::entry::run`, which never returns, so process startup, the environment, native services, declared windows and the run loop all stay in Rust. The CLI generates and manages the Apple project around this; see [project structure](../01-getting-started/04-project-structure.md).

**The Swift package** exists only for embedding WaterUI in an existing native app, and is deliberately thin. `backends/apple/Sources/WaterUI/Embedding.swift` binds the backend's exported runtime, mount and scene entry points through `@_extern(c)`, and the repository-root `Package.swift` publishes it as the `WaterUI` library. A SwiftUI, UIKit or AppKit host creates one `WaterUIRuntime` and mounts content with `WaterUIHost` or `WaterUIHostController`.

Apple support is ARM64 only: `aarch64-apple-darwin`, `aarch64-apple-ios` and `aarch64-apple-ios-sim`. Intel Apple targets are retired.

Layout and gestures behave identically on every backend. Layout is specified once (see [the layout engine](03-layout-engine.md)); a native control whose own behavior differs is non-conforming and is fixed in the backend. Typesetting is the exception by design: the text leaf reports its size and each platform decides line breaking and shaping.

## Hydrolysis

Hydrolysis (`backends/hydrolysis`) is WaterUI's only self-drawn backend, and it is also the Android backend. It does not bundle a widget theme. A build selects its painter per target, and nothing switches painters at run time.

| Painter | Target | Notes |
| --- | --- | --- |
| Cherenkov GPU | every target | the default `gpu` feature, on wgpu; off Android the GPU stack is always linked and the feature changes nothing |
| Android HWUI | Android | the `hwui` feature: the retained layer tree is recorded into `RenderNode` display lists, and text is laid out by the platform's `StaticLayout` |

On Android `hwui` and `gpu` may combine, keeping the HWUI painter with the GPU crates present for hosted GPU content, and a pure HWUI build (`--no-default-features --features hwui`) links none of the GPU stack. GPU rendering is Vulkan-only on Linux and Android, DX12-only on Windows and Metal on Apple platforms; the web keeps WebGPU and WebGL. A device without that API fails with an unsupported-device error rather than falling back.

Cherenkov is the 2D engine in `graphics/cherenkov`. Its GPU backend runs on wgpu with shaders precompiled at build time. It also has a CPU backend, `cherenkov-cpu`, a multi-core SIMD rasterizer that can stream 16-row bands instead of keeping a framebuffer, aimed at microcontroller-class devices; Hydrolysis does not link it today. Runtime render paths never read GPU targets back to CPU memory; offscreen snapshot export is a separate verification path.

### Style packages

Hydrolysis draws widgets through a `Style`, the trait a style package implements:

```rust,ignore
pub trait Style: theme::WidgetTheme + 'static {
    fn install_tokens(&self, env: &mut Environment);
}
```

`WidgetTheme` and the `theme` module are the public contract. A style records through the render-target-neutral `hydrolysis::draw` vocabulary (`Recorder`, `Paint`, shapes, materials) and depends on no render engine. The Material 3 implementation is `hydrolysis-m3`, maintained in its own repository. Hydrolysis's own tests mount the in-tree `hydrolysis::testing::TestStyle`, which depends on no style package.

### Unsupported components panic

Hydrolysis never draws a stand-in for a component it cannot realize. A view that reaches the backend with no realization, such as a map whose GPU hook was never installed, panics at the earliest point it is seen, either at measure time or at node build, with a message naming the missing piece and how to install it. A system icon panics for a different reason: self-drawn backends have no OS icon catalog, so you use a packaged icon crate (see [icons](../03-ui/12-icons.md)). A stub that merely looks like the component would hide the gap from you, so there is none.

### Frames and hosts

Reactive closures request frames through `FrameSignals`, a handle shared with the frame pump that drains the requests. An idle window schedules no work: there is no polling and no timer. A change made outside a frame wakes the host once, however many requests arrive before the next frame. Hosts are the winit runner on desktop, the browser window on the web, the Android host library with the Kotlin Gradle project in `backends/hydrolysis/android`, and a headless runner for offscreen rendering. See [rendering](01-rendering.md) for what happens inside a frame and [testing](../07-tools/03-testing.md) for how `waterui-testing` drives the headless runner.

## Shared contracts: `waterui-backend-core`

`backends/core` holds what Rust backends have in common, so no backend re-implements it:

| Module | What it shares |
| --- | --- |
| `dispatcher` | `ViewDispatcher<T, C, R>`: routes a view to a handler registered for its concrete type |
| `frame_signals` | `FrameSignals`: the cloneable handle reactive closures use to request a redraw or a refresh pass, which also tracks the structural rebuild generation |
| `gesture` (feature `gestures`) | gesture recognition state machines fed by pointer phases, so thresholds and timing match everywhere |
| `input` | the platform-agnostic input vocabulary, such as `TouchPhase` |
| `scroll` | scroll offset and viewport math, with a handle per scroll view |
| `overlay` (feature `overlay`) | window-space placement of anchored overlays |
| `time` | a monotonic clock valid on native and web targets |
| `log` (feature `log`) | the `WATERUI_LOG` / `RUST_LOG` filter every backend's subscriber shares |

`ViewDispatcher` registers one handler per view type and falls back to `body()` when none is registered:

```rust,ignore
let mut dispatcher: ViewDispatcher<(), Context, Widget> = ViewDispatcher::new();
dispatcher.register(|state, ctx, view: Native<TextConfig>, env| build_text(state, ctx, view, env));
let widget = dispatcher.dispatch(view, &env, ctx);
```

### The main-loop executor rule

Whoever owns the main loop supplies the `LocalExecutor`. Every host has an executor bound to its loop, and that executor is what you hand to `try_init_local_executor`. Handing it `native_executor::NativeExecutor` on a non-Apple target is a mistake: its main-thread polyfill expects the thread that called `start_main_executor`, an entry point that blocks and never returns. A host that owns its loop must not call it. The error is made at install time but panics only at the first `spawn_local`, so check it whenever a new host or test harness is added. `NativeExecutor` remains correct for the global executor, which needs no main-thread affinity.

## Support tiers and other backends

| Tier | Platforms | Meaning |
| --- | --- | --- |
| 1 | macOS, iOS, Android | tested on real devices |
| 2 | Windows, Linux | built and tested in CI, failures fixed promptly; Windows does not support dynamic linking, so development builds link statically |
| 3 | the GTK, WinUI and TUI backends | experimental |

Apple and Hydrolysis are the mainline backends: their integration checks define the release contract. The GTK, WinUI and TUI backends live in their own repositories, have no stability or completeness guarantee, and never block a framework release. ESP32 is not supported. For the platform list from a user's side, see [platforms](../01-getting-started/05-platforms.md).

## Where to go next

- [Rendering](01-rendering.md) follows a frame from view tree to pixels.
- [The layout engine](03-layout-engine.md) describes the contract every backend implements.
- [Library authoring](05-library-authoring.md) covers shipping a component that works on both backends.

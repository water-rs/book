# Troubleshooting

> **In this chapter, you will:**
>
> - Diagnose an environment problem with `water doctor` before touching anything else
> - Recognize the silent bugs that compile fine and behave wrong, and the compile errors that point at them
> - Read WaterUI's fast-fail messages and act on them
> - Turn on application and native logs with `water run --logs debug` and `--native-logs`

Most failures are environment failures, and the CLI already knows how to check
the environment. Start there, then work down the chapter by symptom.

## Doctor first

```bash
water doctor          # check the environment
water doctor --fix    # install what can be installed automatically
water doctor --json   # the same report, one machine-readable record per item
water clean           # remove this project's build cache and generated backends
```

`water doctor` groups its items under one heading each: the Rust toolchain, host
tools (Git and a C toolchain), Apple, Hydrolysis (desktop and web), WinUI, GTK4,
Android, and build helpers. Each missing item is marked `[fixable]` when `--fix`
can install it and `[manual]` when you have to. Inside a project, every backend
and platform is checked, because the CLI can generate any of them. Outside a
project, only the backends your host builds for are checked (Apple on macOS,
WinUI on Windows, GTK4 on Linux, Hydrolysis everywhere). The rest, Android
included, are listed as optional and do not fail the run.

`--fix` runs up to three passes and attempts every fixable item even if an
earlier one fails. An install that modifies the system outside `~/.water`
(Visual Studio Build Tools, the LLVM MSI) needs your consent: answer the prompt,
or pass `--yes` on a non-interactive run, otherwise it is skipped with a note.

If doctor is green and the app still fails, the problem is in your code or in
the runtime. Jump to [Fast-fail messages](#fast-fail-messages) and
[Silent bugs](#silent-bugs-that-compile). Platform setup is covered in
[Platforms](../01-getting-started/05-platforms.md) and the CLI in
[the CLI chapter](../01-getting-started/01-cli.md).

## Rust toolchain

### Rust version too old

**Symptom**: compilation errors about unstable features or unrecognized syntax.

WaterUI uses Rust edition 2024, and the framework's `rust-version` is **1.99**.
Doctor enforces the highest of the CLI's, the framework's and your project's
`rust-version`.

```bash
rustc --version
rustup update stable
```

If the project pins a toolchain in `rust-toolchain.toml`, doctor checks the
pinned toolchain instead, and `--fix` installs missing targets into it. A pin
below the floor has to be raised in that file.

### Missing target triple

**Symptom**: `error[E0463]: can't find crate for 'std'` when cross-compiling.

`water doctor --fix` installs the targets for the platforms you build. To add
them by hand:

```bash
rustup target add aarch64-apple-ios          # iOS device
rustup target add aarch64-apple-ios-sim      # iOS simulator
rustup target add aarch64-linux-android      # Android device
rustup target add x86_64-linux-android       # Android emulator on an x86 host
```

Apple support is ARM64-only: macOS, iOS devices and iOS simulators use
`aarch64-apple-*` targets. Intel Apple targets are retired, and the CLI rejects
an explicit request for one, such as `water build --platform macos --arch x86-64`,
with an error instead of building.

### Cryptic cargo failures

Update Rust (`rustup update`), then refresh the lockfile (`cargo update`).
`water` builds in a managed cache under `~/.water/build_cache`, not in your
project's `target/`. `water clean` removes this project's part of it. Dependency
artifacts shared by every project on the machine stay, and
`water gc build-cache --shared-target` removes those too, at the price of a full
rebuild.

## Missing features

WaterUI is feature-granular, and a missing feature shows up as an unresolved
path, not a runtime problem.

### `cannot find ... in crate waterui`

**Symptom**: `waterui::svg`, `GpuContent`, `GpuContentView`, `ShaderPaintView`,
`FlowingGradient` or `FilterViewExt` does not resolve.

These live behind the `gpu` feature, which is on by default (`FilterViewExt`
needs only `effects`, which `gpu` implies). If you set
`default-features = false`, typically to cross-compile for an embedded target
without wgpu, name every feature you still use. The defaults are `gpu`,
`assets`, `dialog`, `snackbar`, `highlight`, `locale-datetime` and `locale-list`:

```toml
waterui = { version = "0.5", default-features = false, features = ["gpu", "assets"] }
```

### `waterui::webview` does not resolve

`webview` is not a default feature. Opt in:

```toml
waterui = { version = "0.5", features = ["webview"] }
```

Which engine draws the page follows what your app links, not a `Water.toml`
key: with no engine crate the platform's own web view is used where the backend
bridges one. See [WebView](../04-rich/03-webview.md).

`canvas`, `chart`, `barcode`, `particle`, `media`, `video`, `remote`,
`inspector`, `widgets` and `ts` are opt-in the same way. `asset!("logo.png")`
fails to compile without `media`, because `ImageAsset` does not exist then. Maps
are not a feature: add the `waterui-map` crate. Check the feature list in the
facade's `Cargo.toml` rather than guessing.

## Platform SDKs

### iOS: Xcode not found

```bash
xcode-select -p
# If it points at CommandLineTools instead of Xcode.app:
sudo xcode-select -s /Applications/Xcode.app/Contents/Developer
sudo xcodebuild -license accept
```

### iOS: no simulator found

```bash
water devices              # what water can see
water run --platform ios   # pick a simulator and boot it if needed
```

Without `--device`, `water run` uses the device you used last, then the only
candidate, then asks you to pick.

If `water devices` lists no simulators at all, the problem is the host Xcode
install, and `water doctor` reports it.

### iOS: code signing errors

Simulator builds need no signing. Device builds need an Apple Developer account
configured in Xcode, a development certificate and provisioning profile, and the
right team selected.

### Android: SDK not found

The CLI reads `ANDROID_SDK_ROOT`, then `ANDROID_HOME`, and otherwise looks in
the default location: `~/Library/Android/sdk` on macOS, `~/Android/Sdk` on
Linux, `%LOCALAPPDATA%\Android\Sdk` on Windows. Set the variable only when your
SDK lives elsewhere, and add it to your shell profile:

```bash
export ANDROID_SDK_ROOT="/path/to/Android/sdk"
```

### Android: NDK not found

**Symptom**: cross-compilation fails looking for `aarch64-linux-android*-clang`.

The CLI builds against one pinned NDK release and looks for it under the SDK's
`ndk/` directory. Let doctor install it:

```bash
water doctor --fix
```

### Android: emulator not running

```bash
water devices --platform android
water run --platform android --device Pixel_9   # an AVD name boots that emulator
```

`water devices` lists connected devices and every Android Virtual Device (AVD),
running or not, and `water run` boots a stopped AVD before deploying. If no AVD
is listed, create one in Android Studio's Device Manager.

### Linux: system packages

The CLI's Linux default backend is still the GTK4 backend, which is experimental
(Tier 3). Hydrolysis is the mainline self-drawn backend, so select it
explicitly:

```bash
water run --platform linux --backend hydrolysis
```

Both backends build against distribution development packages (the C toolchain,
`pkg-config`, libclang, Wayland, ALSA, GBM, XCB, fontconfig and the GTK4 and
Pango headers). You do not need to remember the list: `water doctor` names every
package your distribution is missing under the Linux system packages item, and
`water doctor --fix` installs them with your package manager.

### Linux: no usable GPU adapter

**Symptom**: Hydrolysis panics at startup with `this host has no GPU Hydrolysis
can use` or `no surface-compatible adapter was found`.

GPU rendering on Linux is Vulkan-only, so install the Vulkan loader and a
driver. Mesa's packages carry lavapipe, a software Vulkan driver:

```bash
sudo apt install libvulkan1 mesa-vulkan-drivers vulkan-tools         # Ubuntu / Debian
sudo dnf install vulkan-loader mesa-vulkan-drivers vulkan-tools       # Fedora
sudo pacman -S vulkan-icd-loader vulkan-swrast vulkan-tools           # Arch Linux
```

Hardware adapters rank first, and a software adapter is used only when the host
has no hardware GPU. If `vulkaninfo --summary` lists no device at all, there is
nothing to render on and the backend panics at startup. On a virtual machine,
enable 3D acceleration. To reproduce a software-adapter run on a machine that
does have a GPU, set `WATER_HYDROLYSIS_FORCE_FALLBACK_ADAPTER=1` for that run.

## Build failures

### Linking errors

**Symptom**: `undefined reference` or `unresolved external symbol`.

- **Missing system libraries**: run `water doctor` to check SDK installation.
- **Architecture mismatch**: confirm the target is installed with
  `rustup target list --installed`.
- **Stale artifacts**: `water clean`, then rebuild.

### The CLI is out of date

The `water` on your `PATH` is a previously installed binary. `water update`
replaces it with the newest release, and `water update --check` reports what is
available without installing.

## Fast-fail messages

WaterUI refuses to degrade into a plausible-looking default. Most checks inside
the framework are *contract violations*: a **debug build panics** with a
specific message, and a **release build logs the error** through `tracing` and
carries on with a documented fallback. So the same bug is a crash during
development and an error line in a shipped app. Run `--logs debug`, or at least
read the error level, before you conclude a release build is healthy. The
messages below say where a check panics in every build instead.

### Theme tokens: `... is not installed in the environment`

A color slot or color scheme resolved against an environment that nothing
installed it into. The message says what the release fallback is: a theme color
token resolves opaque black, and a missing scheme is the light scheme.

The runtimes install the theme tokens themselves, so an app built through
`water run` does not hit this. It appears when you render a view against a bare
`Environment::new()` that no backend or theme has touched, for example in a
hand-built harness. Render through the testing or preview tooling instead
([Testing](../07-tools/03-testing.md), [Preview](../07-tools/01-preview.md)), or
install the colors, fonts and scheme with a `Theme` yourself. To ask whether a
scheme exists without triggering the violation, use
`waterui::theme::installed_color_scheme(env)`, which returns
`Option<Computed<ColorScheme>>`. See [Environment](../02-core/03-environment.md)
and [Styling](../03-ui/11-styling.md).

Font tokens and the graphics color keys fail harder: a text style whose font
token was never installed panics with `font token is not installed in the
environment`, and a graphics key such as `BackgroundColor` with
`BackgroundColor is not installed in the environment`, in release builds too.

### Floating surfaces

`.floating()` needs nothing installed. It reads a `FloatingStyle` from the
environment if a theme supplied one and otherwise uses the framework default,
built from the same theme tokens, so it has no panic of its own. To restyle
every floating surface, install your own `FloatingStyle` in the environment, or
pass one to a single view with `.floating_with(style)`.

### A `Label` builder panicked

`Label::icon` and the other decoration builders (`system_icon`, `leading`,
`trailing`, `spacing`, `font`) only work on a semantic label built from text.
Calling one on a label built with `Label::new(semantic_text, content)` panics in
every build with a message such as `Label::icon requires a semantic text label;
compose icons inside Label::new content`, instead of silently dropping the
decoration. Build the icon into the custom content.

`LabelDisplayMode::IconOnly` does not panic on a label without an icon. That
label shows its title, so an icon-only toolbar with one text item means that
item has no icon.

### Hydrolysis: unsupported component

Hydrolysis is self-drawn, so a few components need something the platform or an
extra crate has to supply. Reaching one without it panics with a message that
names the fix:

| Message starts with | Cause | Fix |
|---|---|---|
| ``SystemIcon `name` is unsupported on Hydrolysis`` | Self-drawn backends have no OS icon catalog. | Use a packaged icon crate, see [Icons](../03-ui/12-icons.md). |
| `Map is unsupported on Hydrolysis` | No map realization is installed. | Insert `MapGpuOptions` and call `waterui_map_gpu::install` in `app(env)`, see [Maps](../04-rich/02-maps.md). |
| `WebView is unsupported on this Hydrolysis build` | No web engine is bridged. | Link a browser engine crate or enable the system web view features the message lists, see [WebView](../04-rich/03-webview.md). |
| `PlatformView is unsupported on this runner` | The host cannot mount native children. | Only the Android runner embeds native views, so do not use a platform view elsewhere. |

A map that is installed but stays empty on Android, with no panic, usually has
every tile request failing for lack of the internet permission. See
[Android network calls fail](#android-network-calls-fail-with-a-resolution-error).

### Extraction and executor panics

**"failed to extract `T` from environment for action".** A handler parameter
could not be extracted, so the handler did not run. This is a contract
violation: a debug build panics, a release build logs it and drops the action.
The rest of the message names the cause:

- **"Environment state `T` not found at position N".** A handler asked for a
  `State<T>` that nothing injected. Add `.state(&value)` on the button, on an
  ancestor if several handlers need it, or `App::state` for an application-wide
  handler.
- **"Environment value `T` not found".** Same, for `Use<T>`: the value must be
  installed in the environment, typically in `app(env)`.

**"Local executor not set".** `spawn_local` ran before the backend mounted the
app's view, for example inside `main` or a `Store::new`. Move the call into
`.task(..)`, `.on_appear(..)` or a handler body.

**`Image::new` panics at construction.** `waterui::media::Image::new(pixels,
width, height)` asserts that the buffer length is exactly `width * height * 4`
(RGBA8). It needs the `media` feature.

**`Id::try_from(0)` fails.** `Id` is non-zero, so offset 0-based indices by one.

## Silent bugs that compile

The type system cannot catch these. Recognize them by symptom.

### The UI never updates

A `.snapshot()` reached a view-building expression, which turns a signal into a
one-time value. Search the view function for `.snapshot()`. Every hit outside a
handler or a `.map()` closure is suspect.

```rust,ignore
view.opacity(fade.snapshot())   // frozen: read once, never subscribes
view.opacity(fade.clone())      // reactive: the property tracks the binding
```

Text has the same problem. `text(format!("Count: {}", n.snapshot()))` formats
once. Write `text!("Count: {n}")`. Plain `text(format!(..))` over a value that is
not a signal is fine. See [Reactive state](../02-core/02-reactive.md).

### Typed input resets, or a control loses focus on every keystroke

Something above it is a `watch` that rebuilds the subtree, so the control is a
new instance each time. Replace the `watch` with `text!` for reactive text, pass
the signal straight to a reactive property, or use a collection for a dynamic
set of views. State owned by a rebuilt component goes with the old instance, so
own it one level up in a `Binding` and pass it in.

### A list re-renders entirely, or rows shuffle after an insert

The collection is behind a `watch` over a `Vec`, instead of `Lazy::for_each`
(`use waterui::component::lazy::Lazy;`) or `List::for_each` over an
`Identifiable` collection. And the `#[id]` field must
identify the same logical row across updates: an array index or a value
regenerated per render makes rows lose state. See
[Lists](../03-ui/05-lists.md).

### A handler receives the wrong binding

Handler parameters of the same state type bind positionally: the first
`.state(..)` call feeds the first parameter of that type. Reorder the calls, or
inject one app-state struct instead.

### Rounded corners come out as a capsule

`RoundedRectangle::new` takes a fraction of the shorter side in `0.0..=0.5`, not
points. A point value such as `12.0` saturates at fully rounded.

```rust
use waterui::prelude::*;
use waterui::shape::RoundedRectangle;

fn card() -> impl View {
    text!("Card")
        .padding()
        .clip(RoundedRectangle::new(0.1))
}
```

Use `Capsule` when fully rounded is what you want, and `FixedRoundedRectangle`
when you need an absolute radius.

### A background task never runs, or dies instantly

The handle `spawn_local` returns cancels its future when dropped. A task that
should outlive the statement that started it ends in `.detach()`.

```rust
use std::time::Duration;
use waterui::prelude::*;
use waterui::task::{sleep, spawn_local};

fn refresh_button() -> impl View {
    let status: Binding<Str> = binding("Idle");

    vstack((
        text!("{status}"),
        button("Refresh")
            .action(|State(status): State<Binding<Str>>| {
                spawn_local(async move {
                    sleep(Duration::from_millis(500)).await;
                    status.set(Str::from("Done"));
                })
                .detach();
            })
            .state(&status),
    ))
}
```

A button whose whole action is async can use `.action_async(..)`, which spawns
and detaches for you. For work that should die with a view, use `.task(..)` on
the view instead. See [Suspense](../06-advanced/03-suspense.md).

### Other silent traps

- **Padding lands on the wrong edges.** `EdgeInsets::new(top, leading, bottom,
  trailing)` is SwiftUI's order, not CSS.
- **`LongPressGesture::new(Duration::..)` is rejected.** It takes a `u32` in
  backend time units, so write `LongPressGesture::new(500)`.
- **A test's wait passes but checks nothing.** The query-chained
  `.wait_for_existence(timeout)` returns a `bool`. Wrap it in `assert!(..)`.
- **An animation snapshot shows the end state.** A test used
  `std::thread::sleep`. The animation clock advances with pumped frames, so
  sample with `pump_for(duration)`. See [Testing](../07-tools/03-testing.md).
- **An on-demand window never opens.** `conditional_window` returns an invisible
  view that must still be placed in the tree. See
  [Windows](../03-ui/08-windows.md).

## Compile errors

| Error | Cause | Fix |
|---|---|---|
| `no function or associated item named 'new' found for struct 'Binding'` | `Binding::new` does not exist. | `Binding::i32(v)`, `Binding::bool(v)`, `Binding::container(v)`, or `Binding::default()` for `Binding<Option<T>>`. |
| type annotations needed for `Binding<_>` after `binding(v)` | `binding` takes `impl Into<T>`, so `T` is unconstrained. | A typed constructor, a `let` annotation, or `binding::<T>(v)`. |
| `cannot find function 'when' in this scope` | `when` is not in the prelude. | `use waterui::widget::condition::when;` |
| `cannot find trait 'Identifiable' in this scope` on a `#[derive(Identifiable)]` | The derive expands to `impl Identifiable`, and the trait is not in the prelude. | `use waterui::Identifiable;`, which imports the trait and the derive. |
| `no method named 'map'` on a signal | `SignalExt` is not in scope, because the file does not import the prelude. | `use waterui::prelude::*;` or `use waterui::SignalExt;` |
| `no method named 'is_empty'` on a string signal | Signal string methods are prefixed. | `.str_is_empty()`, `.str_len()`, `.str_contains(..)` |
| `use of undeclared crate 'tracing'` | Logging goes through the facade re-export. | `use waterui::log::debug;`, no extra dependency. |
| `cannot find macro 'text'` | Macro not imported. | `use waterui::prelude::*;` then bare `text!`, never `waterui::text!`. |
| `ForEach<..>: View is not satisfied` | `ForEach` is a collection, not a view. | `Lazy::for_each(data, f)` (from `waterui::component::lazy`) or `List::for_each`. |
| `.title("Inbox")` takes 0 arguments | `Text::title()` (font size) shadows the navigation title. | Title the container, or use `NavigationView::new(title, content)`. |
| `expected 'Binding<T>', found '&Binding<T>'` | Controls borrow, modifiers own. | Controls take `&binding`, modifiers take `binding.clone()`. |
| `borrowed data escapes outside of function` | Views are `'static`. | Take `&'static str`, `Str` or `impl IntoText`. |
| `could not find 'webview' in 'waterui'` | Behind a cargo feature. | Enable it in `Cargo.toml`. |
| clippy `future_not_send` on an async helper | UI futures legitimately hold non-`Send` state. | An item-level `#[expect(clippy::future_not_send, reason = "..")]`. |

Do not add crate, module or file-level `allow` attributes to silence a lint. If a
lint is a genuine false positive, scope an `expect` to the item and give the
reason.

The logging row has a compiled example:

```rust
use waterui::log::debug;
use waterui::prelude::*;

fn labelled(name: &'static str) -> impl View {
    debug!(name, "building the label");
    text(name)
}
```

## Assets

### Asset not found at runtime

**Symptom**: an `ImageAsset` or `FontAsset` resolves to a missing resource.

1. Confirm the file exists in the project's `assets/` directory.
2. Check the filename case. Mobile platforms are case-sensitive.
3. The CLI bundles `assets/` automatically. If you compose a custom bundle,
   confirm it points at the right path.

See [Assets](../04-rich/05-assets.md).

### Image not displaying

- **Format**: WaterUI decodes PNG, JPEG, GIF, WebP, BMP, ICO and TIFF itself.
  AVIF and HEIF go through the platform decoder on Apple platforms, Android and
  the web, and through a bundled software decoder elsewhere.
- **Corruption**: try opening the file in an image viewer.
- **Size**: very large images can fail to decode on memory-constrained devices.

## Layout problems

**A view is invisible or zero-sized.** Its parent proposed no space. Read the
accessibility bounds (`water inspector`, or `.all()` in a test) to find which
container mis-sized it. A screenshot says something is wrong, and bounds say
where. Do not hide it with an explicit `.size(..)` on the child: that destroys
the reproduction and leaves the container bug for the next person. Layout rules
are in [Layout](../03-ui/02-layout.md).

**An edge-anchored overlay drifts or vanishes when the window grows.** A window
overlay layer must fill the window. Use `absolute(..)` and place children with
`PositionExt`.

**A custom shape's corners look stretched.** A `Shape` path is drawn in unit
space and scaled to the view's bounds, so test it against a deliberately
non-square rectangle.

**A view is hidden.** Check for `.opacity(0.0)`, a transparent background, or
the condition driving `when(..)`.

## Runtime issues

### `water run` returned to the prompt

The app crashed, because a run that stays alive does not exit. Re-run with
`--logs debug` and read the tail.

### App crashes on startup

```bash
water run --platform ios --logs debug
water run --platform android --logs debug
water run --platform linux --backend hydrolysis --logs debug
```

Common causes: a panic inside your `app(env)` function (the log carries the
message), one of the fast-fail messages above, or a Rust library and native host
out of sync, which `water clean` followed by a rebuild fixes.

### Android network calls fail with a resolution error

The `internet` permission is missing from `Water.toml`. Android denies DNS
resolution outright instead of reporting a permission failure. An Android build
warns when a dependency declares that it needs the permission, or when an HTTP
client crate is in the graph. Declare it:

```toml
[permissions.internet]
enable = true
description = "Loads content from the network"
```

### `water preview` cannot load the app, or dev rebuilds are cold

The crate is missing the `dev = ["waterui/dynamic_linking"]` feature stanza that
generated projects carry, and the preview error says ``Preview requires
`<crate>/dev` feature``. Restore it under `[features]` in `Cargo.toml`. See [Project structure](../01-getting-started/04-project-structure.md).

### A backend conflicts with `Water.toml`

`[platforms.<platform>] backend = "..."` is the project's declared backend. A
different CLI or MCP override is rejected before building. Use the declared
backend or change the declaration.

## Logging

WaterUI logs through `tracing`, and the facade re-exports it as `waterui::log`.
Never use `println!` for diagnostics: it does not reach the CLI's log pipeline.

```bash
water run --platform <platform> --logs debug
water run --platform <platform> --logs debug --native-logs
```

`--logs` accepts `error`, `warn`, `info` (the default), `debug` and `verbose`. It
streams device logs at or above that level and has the application log at it, so
`debug` shows your `debug!` output. `--native-logs` adds the platform's own
output (`NSLog`, `print` and so on) alongside WaterUI's. It is noisy, and useful
when the fault is in native code. Neither flag is supported with the web platform.

Backend dispatch and rendering have their own switches. They are read by the
app, so export them into it with `--env NAME`, which takes the value from your
shell and works on every platform:

```bash
# Trace every view the backend dispatches, indented by depth (debug level)
WATERUI_DISPATCH_DEBUG=1 water run --platform linux --backend hydrolysis \
  --logs debug --env WATERUI_DISPATCH_DEBUG

# Periodic frame-render diagnostics from the Hydrolysis runner (info level)
WATERUI_HYDROLYSIS_RENDER_DIAG=1 water run --platform linux --backend hydrolysis \
  --env WATERUI_HYDROLYSIS_RENDER_DIAG
```

Platform tools that read the same logs directly:

```bash
# Apple platforms log under the dev.waterui subsystem
log stream --predicate 'subsystem == "dev.waterui"' --level debug

# Android: filter logcat to your app's process
adb logcat --pid="$(adb shell pidof -s com.example.myapp)"
```

## Diagnosing without a device

These are faster than a simulator, and they produce evidence you can attach to a
bug report.

```bash
water preview my_view --path . --output preview.png
cargo nextest run -p my-app -E 'test(the_case)' --no-capture
water inspector
water bench --format json
```

A test on the accessibility tree tells you what the tree contains, a PNG tells
you what it looks like, and a bench tells you whether an interaction updates
precisely or rebuilds. A passing test is only evidence about what it observes,
so before trusting a green run ask what the assertion would have to see in order
to fail. See [Preview](../07-tools/01-preview.md), [Testing](../07-tools/03-testing.md),
[Benchmarks](../07-tools/04-benchmarks.md) and [Inspector](../07-tools/05-inspector.md).

## Getting help

1. Search the [GitHub issues](https://github.com/water-rs/waterui/issues).
2. Include `water doctor` output in the report.
3. Include the full error and the relevant `--logs debug` output.
4. State your OS, `rustc --version` and platform SDK versions.
5. Provide a minimal reproduction if you can.

For scripted use of the commands in this appendix, see
[Automation](automation.md).

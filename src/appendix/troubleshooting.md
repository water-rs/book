# Troubleshooting

> **In this appendix, you will:**
> - Fix Rust toolchain and platform SDK problems
> - Resolve build failures, missing-feature errors, and FFI mismatches
> - Read WaterUI's fast-fail panics and act on them
> - Debug rendering and reactivity problems with structured logging

Start with `water doctor`; it diagnoses most environment problems and can fix
several of them.

## First steps

```bash
water doctor          # check the environment
water doctor --fix    # install what can be installed automatically
water clean           # clear WaterUI build artifacts (not the cargo target dir)
```

## Rust toolchain

### Rust version too old

**Symptom**: compilation errors about unstable features or unrecognized syntax.

WaterUI requires Rust edition 2024 and rustc **1.95** or newer.

```bash
rustc --version
rustup update stable
```

If the project pins a toolchain in `rust-toolchain.toml`, raise it there too.

### Missing target triple

**Symptom**: `error[E0463]: can't find crate for 'std'` when cross-compiling.

```bash
rustup target add aarch64-apple-ios          # iOS device
rustup target add aarch64-apple-ios-sim      # iOS simulator, Apple Silicon
rustup target add x86_64-apple-ios           # iOS simulator, Intel
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi
rustup target add x86_64-linux-android
```

`water doctor --fix` installs missing targets for you.

### Cryptic cargo failures

Update Rust (`rustup update`), then refresh the lockfile (`cargo update`). If it
persists, clear the registry cache:

```bash
rm -rf ~/.cargo/registry/cache
cargo update
```

Avoid `cargo clean` unless nothing else works — it discards every compiled
artifact and forces a full rebuild.

## Missing features

WaterUI is feature-granular, and a missing feature shows up as an unresolved
path rather than a runtime problem.

### `cannot find ... in crate waterui`

**Symptom**: `svg`, `FilterViewExt`, `ImageGenerator`, `ImageAnalysis`, or one of
the generator types (`NoiseGenerator`, `LinearGradientGenerator`, and friends)
does not resolve.

These live behind the `gpu` feature, which is on by default. If you built with
`--no-default-features` — typically to cross-compile for an embedded target
without wgpu and vello — re-enable it explicitly:

```toml
waterui = { version = "0.2", default-features = false, features = ["gpu"] }
```

### `waterui::webview` does not resolve

`webview` is not a default feature. Opt in:

```toml
waterui = { version = "0.2", features = ["webview"] }
```

Then pick the engine in `Water.toml` with `webview_backend` (`default`,
`system`, `wpe`, or `cef`). Setting the key alone changes nothing if your app
never links `waterui-webview`.

`chart`, `barcode`, `map`, `particle`, and `navigation-restoration` are opt-in
the same way.

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
water run --platform ios   # let water pick and boot one
```

If `water devices` lists no simulators at all, the problem is the host Xcode
install; `water doctor` will say so.

### iOS: code signing errors

Simulator builds need no signing. Device builds need an Apple Developer account
configured in Xcode, a development certificate and provisioning profile, and the
right team selected in project settings.

### Android: SDK not found

```bash
export ANDROID_HOME="$HOME/Library/Android/sdk"   # macOS
export ANDROID_HOME="$HOME/Android/Sdk"           # Linux
```

Add it to your shell profile so it survives a new terminal.

### Android: NDK not found

**Symptom**: cross-compilation fails looking for `aarch64-linux-android-*` tools.

```bash
sdkmanager --install "ndk;27.0.12077973"
```

The CLI finds the NDK under `$ANDROID_HOME/ndk/`.

### Android: emulator not running

```bash
water devices
```

If the list is empty, start an emulator from Android Studio and re-run. `water
run --platform android` then deploys to it automatically.

### Linux: GTK4 development packages

The GTK4 backend is the native Linux bridge and needs GTK's development
headers:

```bash
sudo apt install libgtk-4-dev              # Ubuntu / Debian
sudo dnf install gtk4-devel                # Fedora
sudo pacman -S gtk4                        # Arch Linux
```

Add `libwebkitgtk-6.0-dev` (or your distribution's equivalent) if you use the
system web view.

### Linux: no usable GPU adapter

**Symptom**: Hydrolysis cannot find a wgpu adapter.

Hydrolysis is GPU-required. Install a working Vulkan stack:

```bash
sudo apt install libvulkan-dev mesa-vulkan-drivers vulkan-tools   # Ubuntu / Debian
sudo dnf install vulkan-loader-devel mesa-vulkan-drivers vulkan-tools  # Fedora
sudo pacman -S vulkan-icd-loader vulkan-tools                     # Arch Linux
```

If `vulkaninfo` reports no device, Hydrolysis refuses to boot rather than
quietly falling back to software rendering. Use a machine with a real GPU, or
set `WATER_HYDROLYSIS_FORCE_FALLBACK_ADAPTER=1` for a single diagnostic run.

## Build failures

### Linking errors

**Symptom**: `undefined reference` or `unresolved external symbol`.

- **Missing system libraries**: run `water doctor` to check SDK installation.
- **Architecture mismatch**: confirm the target with
  `rustc --print target-list | grep <platform>`.
- **Stale artifacts**: `water clean`, then rebuild.

### FFI header out of date

**Symptom**: the native backend fails to compile against missing or mismatched
function signatures. This only affects contributors working inside the WaterUI
repository.

```bash
cargo run --bin generate_header --features cbindgen --manifest-path ffi/Cargo.toml
```

Then rebuild the native backend. `ffi/waterui.h` is generated and checked in;
never edit it by hand.

### Proc macro errors

Proc macro crates compile for the host, not the target. If you see "can't load
proc macro", check `rustup show active-toolchain` and that
`cargo build -p waterui-macros` succeeds on its own.

## Fast-fail panics

WaterUI panics with a specific message instead of degrading into a
plausible-looking default. These are the ones you are most likely to hit.

### `WaterUI color token ... is not installed in the environment`

A themed view resolved a color slot that nothing installed. This happens when a
view is rendered against a bare `Environment::new()` with no backend or theme.

Install a theme before rendering. For a Hydrolysis app the CLI-generated entry
point does it for you:

```rust,ignore
let env = Environment::new();
let mut app = my_crate::app(env);
hydrolysis_m3::install_defaults(&mut app.env);
hydrolysis::run(app);
```

In your own `app(env)` you can install one directly:

```rust,ignore
pub fn app(mut env: Environment) -> App {
    env.install(Theme::new().color_scheme(ColorScheme::Dark));
    App::new(main, env)
}
```

### `WaterUI color scheme is not installed in the environment`

Same cause, raised by `current_color_scheme(env)`. When you legitimately need to
ask whether a scheme exists, use the non-panicking
`installed_color_scheme(env) -> Option<Computed<ColorScheme>>` instead.

### ``WaterUI `.floating()` requires FloatingStyle theme tokens``

`.floating()` resolves its container color, clip radius, and shadows from a
`FloatingStyle` in the environment. `Theme::install` inserts a default one, so
this panic means no theme was installed — or that a custom environment removed
it. Install your own to override:

```rust,ignore
env.install(FloatingStyle { clip_radius: 0.25, ..FloatingStyle::default() });
```

### A `Label` builder panicked

`Label::icon`, `system_icon`, `leading`, `trailing`, `spacing`, and `font` are
semantic-only. Calling them on a label built with `Label::new(semantic_text,
content)` panics rather than silently dropping the decoration. Build the
decoration into the custom content instead.

### An unsupported view panicked on Dew

Dew supports a subset of views (stacks, padding, colors, spacers, text) and
panics on anything else rather than rendering something wrong. Either compose
the screen from supported views or contribute the missing handler.

## Assets

### Asset not found at runtime

**Symptom**: an `ImageAsset` or `FontAsset` resolves to a missing resource.

1. Confirm the file exists in the project's `assets/` directory.
2. Check the filename case — mobile platforms are case-sensitive.
3. The CLI bundles `assets/` automatically; if you compose a custom `Bundle`,
   confirm it points at the right path.

### Image not displaying

- **Format**: the self-drawn renderers decode PNG, JPEG, GIF, WebP, BMP, ICO,
  AVIF, and TIFF. Native backends use the platform decoder, whose format support
  differs.
- **Corruption**: try opening the file in an image viewer.
- **Size**: very large images can fail to decode on memory-constrained devices.

## Runtime issues

### View not rendering

1. **Zero size**: the view has no intrinsic size and no frame constraint. Add
   `.size(width, height)` or make sure the parent offers space.
2. **Hidden**: check for `.opacity(0.0)` or a transparent background.
3. **Conditional**: verify the condition driving `when(...)` is what you expect.

Set `WATERUI_DISPATCH_DEBUG=1` to see which view types the backend matched and
which fell through to `body()`.

### Signals not updating

**Symptom**: changing a `Binding` leaves the UI unchanged.

1. **Do not call `.get()` in a view body.** It reads once and never subscribes.
   Use `text!` for reactive text, or a combinator for other values:

   ```rust,ignore
   // Wrong: reads once, no reactivity
   text(format!("Count: {}", count.get()))

   // Right
   text!("Count: {count}")
   ```

2. **Prefer signal-taking APIs.** Passing a `Binding` to `.spacing(...)`,
   `.blur(...)`, or `.disabled(...)` updates that one property. Wrapping the
   subtree in `watch(...)` rebuilds it and throws away any state it owned.

3. **Use a collection for a dynamic set of views.** `ForEach`/`List` over a
   reactive collection diffs by id; `watch` over a `Vec` rebuilds everything.

4. **Mutate in place.** `*count.get_mut() += 1` is the idiom;
   `count.set(count.get() + 1)` reads outside a subscription.

5. **Binding lifetime**: if the binding is dropped, its watchers disconnect.
   Keep it alive at the level that owns the state.

6. **Thread**: update bindings from the main thread. From an async task,
   dispatch back through the executor.

### State lost after an update

Expected when the component was recreated — `when(...)`, `watch(...)`, or any
parent-driven reconstruction initializes a *new* instance, and instance-local
state goes with the old one. The fix is to own that state in a `Binding` one
level up and pass it in, not to try to preserve it across the rebuild.

### App crashes on startup

```bash
water run --platform ios --logs debug
water run --platform android --logs debug
RUST_LOG=debug water run --platform linux
```

Common causes:

- A missing theme token or color scheme — see the panics above.
- A panic inside your `app(env)` function; the log carries the message.
- Rust library and native backend out of sync; rebuild both.

## Platform-specific tools

### iOS simulator

```bash
xcrun simctl erase "iPhone 16"
xcrun simctl spawn "iPhone 16" log stream --level debug --predicate 'subsystem == "dev.waterui"'
```

### Android emulator

```bash
adb shell pm clear dev.waterui.yourapp
adb logcat -s WaterUI:D
adb shell am force-stop dev.waterui.yourapp
```

Generated projects default to a `dev.waterui.<name>` bundle identifier.

### Hydrolysis on desktop

```bash
# Which views matched a handler, and which fell through to body()
WATERUI_DISPATCH_DEBUG=1 water run --platform linux --backend hydrolysis --logs debug

# Per-frame render diagnostics
WATERUI_HYDROLYSIS_RENDER_DIAG=1 water run --platform linux --backend hydrolysis
```

## Debug logging

WaterUI logs through `tracing`:

```bash
water run --platform <platform> --logs debug
RUST_LOG=debug water run --platform <platform>
RUST_LOG=waterui=debug,waterui_core=trace water run --platform <platform>
```

`--logs` accepts `error`, `warn`, `info`, `debug`, and `verbose`. Add
`--native-logs` to include the platform's own output alongside WaterUI's.

Apple platforms log to `os_log` under the `dev.waterui` subsystem:

```bash
log stream --predicate 'subsystem == "dev.waterui"' --level debug
```

Android logs to `logcat`:

```bash
adb logcat -s WaterUI:D
```

## Getting help

1. Search the [GitHub issues](https://github.com/water-rs/waterui/issues).
2. Include `water doctor` output in the report.
3. Include the full error and relevant logs.
4. State your OS, `rustc --version`, and platform SDK versions.
5. Provide a minimal reproduction if you can.

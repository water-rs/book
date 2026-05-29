# Troubleshooting

> **In this appendix, you will:**
> - Diagnose and fix common Rust toolchain and platform SDK issues
> - Resolve build failures, linking errors, and FFI mismatches
> - Debug hot reload, runtime rendering, and signal update problems
> - Use platform-specific debugging tools and structured logging

Something not working? You are in the right place. This appendix covers the most common issues WaterUI developers encounter, organized by category so you can jump straight to your problem. When in doubt, start with `water doctor` -- it diagnoses most environment problems automatically.

## First Steps

Before diving into specific issues, try these general diagnostic steps:

```bash
# Check your environment for known issues
water doctor

# Automatically fix what can be fixed
water doctor --fix

# Clear WaterUI build artifacts (not cargo target)
water clean
```

## Rust Toolchain Issues

### Rust Version Too Old

**Symptom**: Compilation errors mentioning unstable features or missing syntax.

WaterUI requires Rust edition 2024 and a minimum `rustc` version of 1.88.

```bash
# Check your version
rustc --version

# Update to latest stable
rustup update stable
```

If you are using a project-level `rust-toolchain.toml`, make sure it specifies
a sufficiently recent version.

### Missing Target Triple

**Symptom**: `error[E0463]: can't find crate for 'std'` when cross-compiling.

You need to install the target for each platform you build for:

```bash
# iOS (device)
rustup target add aarch64-apple-ios

# iOS (simulator, Apple Silicon)
rustup target add aarch64-apple-ios-sim

# iOS (simulator, Intel)
rustup target add x86_64-apple-ios

# Android
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi
rustup target add x86_64-linux-android

# Linux (usually already installed)
rustup target add x86_64-unknown-linux-gnu
```

Running `water doctor --fix` will install missing targets automatically.

### Cargo Build Fails with Cryptic Errors

**Symptom**: Internal compiler errors or linker failures.

Try these steps in order:

1. Update Rust: `rustup update`
2. Check for corrupted crate cache: `cargo update`
3. If the problem persists, remove the registry cache:
   ```bash
   rm -rf ~/.cargo/registry/cache
   cargo update
   ```

Do **not** run `cargo clean` unless absolutely necessary -- it removes all
compiled artifacts and forces a full rebuild, which can take many minutes.

## Platform SDK Issues

### iOS: Xcode Not Found

**Symptom**: `water doctor` reports Xcode is missing, or `xcodebuild` fails.

```bash
# Verify Xcode is installed
xcode-select -p

# If it points to CommandLineTools instead of Xcode.app:
sudo xcode-select -s /Applications/Xcode.app/Contents/Developer

# Accept the license
sudo xcodebuild -license accept
```

### iOS: Simulator Not Booted

**Symptom**: `water run --platform ios` fails with "No simulator found."

```bash
# List available simulators that water can see
water devices

# Let water pick and boot a simulator automatically
water run --platform ios
```

If `water devices` reports no simulators at all, the issue is the host's Xcode
install -- run `water doctor` to confirm and follow its remediation hints.

### iOS: Code Signing Errors

**Symptom**: Build succeeds but deployment fails with signing errors.

For development builds on the simulator, no signing is required. For device
builds, ensure you have:

1. An Apple Developer account configured in Xcode.
2. A development certificate and provisioning profile.
3. The correct team selected in the project settings.

### Android: SDK Not Found

**Symptom**: `water doctor` reports Android SDK is missing.

Set the `ANDROID_HOME` or `ANDROID_SDK_ROOT` environment variable:

```bash
# macOS (default Android Studio location)
export ANDROID_HOME="$HOME/Library/Android/sdk"

# Linux
export ANDROID_HOME="$HOME/Android/Sdk"

# Add to your shell profile for persistence
echo 'export ANDROID_HOME="$HOME/Library/Android/sdk"' >> ~/.zshrc
```

### Android: NDK Not Found

**Symptom**: Cross-compilation fails with missing `aarch64-linux-android-*` tools.

Install the NDK through Android Studio's SDK Manager, or via command line:

```bash
sdkmanager --install "ndk;27.0.12077973"
```

The `water` CLI will detect the NDK if it is installed under `$ANDROID_HOME/ndk/`.

### Android: Emulator Not Running

**Symptom**: `water run --platform android` fails to connect.

```bash
# List devices and emulators water can see
water devices
```

If the list is empty, start an emulator from Android Studio (or the
`avdmanager` UI) and re-run `water devices`. `water run --platform android`
will then deploy to the running device automatically.

### Linux desktop: GPU/system libraries missing

**Symptom**: Compilation fails with missing system headers, or Hydrolysis fails
to find a usable wgpu adapter on Linux.

Hydrolysis is the active Linux/desktop backend; it talks to the GPU through
`wgpu`. Make sure your distribution has a working Vulkan stack and the
development packages your wgpu backend needs:

```bash
# Ubuntu / Debian
sudo apt install libvulkan-dev mesa-vulkan-drivers vulkan-tools

# Fedora
sudo dnf install vulkan-loader-devel mesa-vulkan-drivers vulkan-tools

# Arch Linux
sudo pacman -S vulkan-icd-loader vulkan-tools
```

If `vulkaninfo` reports no available device, Hydrolysis production surfaces
will refuse to boot rather than silently fall back to a software adapter. Use a
machine with a real GPU, or set `WATER_HYDROLYSIS_FORCE_FALLBACK_ADAPTER=1` for
a one-off diagnostic run only.

The legacy GTK4 backend is no longer supported, so its system dependencies
(`libgtk-4-dev`, `webkitgtk`) are not required for normal app development.

## Build Failures

### Linking Errors

**Symptom**: `undefined reference` or `unresolved external symbol` during linking.

Common causes:

- **Missing system libraries**: The linker cannot find platform SDK libraries.
  Run `water doctor` to verify SDK installation.
- **Architecture mismatch**: Building for the wrong target. Verify with
  `rustc --print target-list | grep <platform>`.
- **Stale build artifacts**: Try `water clean` followed by a fresh build.

### FFI Header Out of Date

**Symptom**: The native backend fails to compile with missing or mismatched
function signatures.

Regenerate the C header:

```bash
cargo run --bin generate_header --features cbindgen --manifest-path ffi/Cargo.toml
```

Then rebuild the native backend. CI verifies this automatically, so if it
passes locally, the header is up to date.

### Proc Macro Errors

**Symptom**: Errors from `waterui-macros` during compilation.

Proc macro crates must be compiled for the host platform, not the target.
If you see errors like "can't load proc macro," check that:

1. You have a working host toolchain: `rustup show active-toolchain`
2. The `waterui-macros` crate compiles on its own:
   `cargo build -p waterui-macros`

## Hot Reload Issues

### Changes Not Reflected

**Symptom**: You save a file but the running app does not update.

Hot reload works by rebuilding the Rust library as a dylib and reloading it.
Check that:

1. You started the app with `water run` (not a manual build).
2. The file you changed is part of the project's crate graph.
3. The build succeeds -- check the terminal for compilation errors.

### Hot Reload Crashes

**Symptom**: The app crashes after a hot reload.

This can happen if:

- You changed a struct's memory layout (added/removed fields) while the native
  side still holds pointers to the old layout. Restart the app to pick up
  structural changes.
- A panic occurred during view construction. Check the logs for panic messages.

## Asset Problems

### Asset Not Found at Runtime

**Symptom**: An `ImageAsset`, `FontAsset`, or related asset constructor resolves
to an empty/missing resource.

1. Verify the file exists in your project's `assets/` directory.
2. Check the filename case -- mobile platforms are case-sensitive.
3. Ensure the asset is included in the build. The `water` CLI bundles the
   `assets/` directory automatically; if you compose a custom `Bundle`,
   confirm it points at the right path.

### Image Not Displaying

**Symptom**: No error, but the image view is empty.

- Check the image format. WaterUI supports PNG, JPEG, and WebP natively.
- Verify the file is not corrupted: try opening it in an image viewer.
- Check the image dimensions -- very large images may fail to decode on
  memory-constrained devices.

## Runtime Issues

### View Not Rendering

**Symptom**: A view appears blank or invisible.

Common causes:

1. **Zero size**: The view has no intrinsic size and no frame constraint.
   Add `.size(width, height)` or ensure the parent provides
   enough space.
2. **Hidden by opacity**: Check for `.opacity(0.0)` or a fully transparent
   background color.
3. **Incorrect conditional**: If using `if`/`else` in view bodies, verify the
   condition is evaluating as expected.

### Signals Not Updating

**Symptom**: Changing a `Binding` does not update the UI.

1. **Do not call `.get()` in view bodies**: This reads the value once without
   subscribing. Use `.map()` to create a `Computed<T>` that tracks changes:

   ```rust,ignore
   // Wrong: reads once, no reactivity
   text(format!("Count: {}", count.get()))

   // Correct: reactive
   text(count.map(|c| format!("Count: {c}")))
   ```

2. **Binding scope**: Ensure the binding outlives the view. If the binding is
   dropped, watchers are disconnected and updates stop.

3. **Thread safety**: Bindings must be updated from the main thread. If you are
   updating from an async task, use the executor to dispatch back to main.

### App Crashes on Startup

**Symptom**: The app exits immediately with a panic or signal.

Check the logs:

```bash
# iOS (Xcode console or)
water run --platform ios --logs debug

# Android
water run --platform android --logs debug
# or: adb logcat -s WaterUI

# Linux
RUST_LOG=debug water run --platform linux
```

Common startup crash causes:

- **Missing theme signals**: The native backend did not install required theme
  values. This is a backend bug; report it.
- **Panic in `app()` function**: Your `app(env)` function panicked. The log
  will show the panic message.
- **FFI mismatch**: The Rust library and native backend are out of sync.
  Rebuild both.

## Platform-Specific Troubleshooting

### iOS Simulator

```bash
# Reset a simulator (clears all data)
xcrun simctl erase "iPhone 16"

# View simulator logs
xcrun simctl spawn "iPhone 16" log stream --level debug --predicate 'subsystem == "dev.waterui"'
```

### Android Emulator

```bash
# Clear app data
adb shell pm clear com.your.app

# View logs filtered to WaterUI
adb logcat -s WaterUI:D

# Force-stop the app
adb shell am force-stop com.your.app
```

### Linux (Hydrolysis)

Hydrolysis is the Rust-side backend for desktop targets. Reach for these knobs
when a Linux build misbehaves:

```bash
# Trace which views the dispatcher matched and which fell through to body().
WATERUI_DISPATCH_DEBUG=1 water run --platform linux --backend hydrolysis --logs debug

# In rare diagnostic situations you can opt into a software wgpu adapter.
# Production surfaces reject software/noop adapters, so do not leave this on.
WATER_HYDROLYSIS_FORCE_FALLBACK_ADAPTER=1 water run --platform linux --backend hydrolysis
```

The legacy GTK4 backend is no longer supported -- treat it as historical
reference rather than a target you ship against.

## Debug Logging

WaterUI uses the `tracing` crate for structured logging. Enable verbose output:

```bash
# Via the water CLI
water run --platform <platform> --logs debug

# Via environment variable
RUST_LOG=debug water run --platform <platform>

# More specific filtering
RUST_LOG=waterui=debug,waterui_core=trace water run --platform <platform>
```

On Apple platforms, logs are sent to `os_log` with subsystem `dev.waterui`.
View them in Console.app or with:

```bash
log stream --predicate 'subsystem == "dev.waterui"' --level debug
```

On Android, logs go to `logcat`:

```bash
adb logcat -s WaterUI:D
```

## Getting Help

If the troubleshooting steps above do not resolve your issue:

1. Search the [GitHub Issues](https://github.com/water-rs/waterui/issues)
   for similar problems.
2. Run `water doctor` and include the output in your bug report.
3. Include the full error message and relevant log output.
4. Specify your OS, Rust version (`rustc --version`), and platform SDK versions.
5. Provide a minimal reproduction case if possible.

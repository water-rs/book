# Installation and Setup

> **In this chapter, you will:**
> - Install Rust and the cross-compilation targets you need
> - Set up one platform toolchain: Apple, Android, or Linux
> - Verify the result with `water doctor`
> - Run a generated project end to end

You need **one** platform toolchain to get started. Pick the one you already
have tooling for and skip the rest; `water doctor` skips checks your host
cannot serve.

## Step 1: install Rust

WaterUI requires **Rust 1.95 or later** on edition 2024. Install with
[rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Confirm the version, and update if it is older:

```bash
rustc --version
rustup update stable
```

### Cross-compilation targets

Add only the targets you plan to build for. `water doctor --fix` installs
missing ones for you.

```bash
# iOS device
rustup target add aarch64-apple-ios

# iOS Simulator (Apple Silicon / Intel)
rustup target add aarch64-apple-ios-sim
rustup target add x86_64-apple-ios

# Android
rustup target add aarch64-linux-android      # modern devices
rustup target add x86_64-linux-android       # Intel/AMD emulators
rustup target add armv7-linux-androideabi    # older 32-bit devices
rustup target add i686-linux-android

# Web (Hydrolysis backend)
rustup target add wasm32-unknown-unknown
```

ESP32 firmware targets are installed through the Espressif toolchain rather
than plain `rustup`; `water doctor` and `water run --platform esp32c3` report
what is missing.

## Step 2: editor setup

Any editor with Rust support works. For Visual Studio Code:

- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
- [Even Better TOML](https://marketplace.visualstudio.com/items?itemName=tamasfe.even-better-toml)
  -- highlights `Cargo.toml` and `Water.toml`
- [CodeLLDB](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb)

RustRover, Zed, and Helix are all fine alternatives.

## Step 3: platform toolchains

### Apple (iOS / macOS)

The Apple backend is a Swift 6.3 package with iOS 26 and macOS 26 deployment
targets, so you need **Xcode 26 or later** on a Mac.

Install Xcode from the Mac App Store, then:

```bash
xcode-select --install
sudo xcodebuild -license accept
```

Verify:

```bash
xcodebuild -version
xcrun simctl list devices available
```

> **Warning:** Skipping the licence acceptance produces a build failure whose
> error message never mentions the licence.

### Android

The Android runtime compiles against SDK 37 with a minimum of API 26
(Android 8.0), NDK 29, and Java 21.

Install [Android Studio](https://developer.android.com/studio), which bundles
the SDK, NDK, and emulator. Then:

1. **Settings > Languages & Frameworks > Android SDK**.
2. Under **SDK Platforms**, install a recent API level.
3. Under **SDK Tools**, install **NDK (Side by side)** and
   **Android SDK Command-line Tools**.

Export the paths from your shell profile:

```bash
export ANDROID_HOME="$HOME/Library/Android/sdk"  # macOS default
# export ANDROID_HOME="$HOME/Android/Sdk"        # Linux default

export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/<version>"
export PATH="$ANDROID_HOME/platform-tools:$ANDROID_HOME/tools/bin:$PATH"
```

Verify, and create an AVD if you have none:

```bash
adb --version
emulator -list-avds

avdmanager create avd -n Pixel_9 -k "system-images;android-34;google_apis;arm64-v8a"
```

### Linux (GTK4)

WaterUI needs more than GTK4 itself: Pango, Wayland, ALSA, VA-API, GBM, XCB,
Clang, and Fontconfig development packages all appear in the build graph.

```bash
# Debian / Ubuntu
sudo apt-get install -y pkg-config libgtk-4-dev libpango1.0-dev libwayland-dev \
  wayland-protocols libasound2-dev libva-dev libgbm-dev libxcb1-dev \
  libclang-dev libfontconfig-dev

# Fedora
sudo dnf install -y pkgconf-pkg-config gtk4-devel pango-devel wayland-devel \
  wayland-protocols-devel alsa-lib-devel libva-devel mesa-libgbm-devel \
  libxcb-devel clang-devel fontconfig-devel

# Arch
sudo pacman -S --needed pkgconf gtk4 pango wayland wayland-protocols \
  alsa-lib libva mesa libxcb clang fontconfig
```

`water doctor --fix` runs the right command for your package manager. Verify
manually with:

```bash
pkg-config --modversion gtk4
```

The GTK4 backend runs on Linux hosts only. On macOS or Windows, use
`--backend hydrolysis` for a desktop build instead.

## Step 4: install the Water CLI

```bash
cargo install waterui-cli
water --help
```

To track the development branch instead, clone the repository and install from
the checkout:

```bash
git clone https://github.com/water-rs/waterui.git
cd waterui
cargo install --path cli --locked
```

## Step 5: verify with `water doctor`

```bash
water doctor
```

Every check is reported as passing, `[fixable]`, `[manual]`, or skipped:

```text
Checking development environment...
  ✓ Xcode
  ✓ iOS SDK
  ✓ iOS Simulator SDK
  ✓ iOS Simulators
  ✓ macOS SDK
  ✓ Rust toolchain
  ⚠ Android SDK (Android SDK is missing) [fixable]
  ✓ Host CMake
  ✓ Java
  ✓ sccache
```

Install everything marked `[fixable]`:

```bash
water doctor --fix
```

Items marked `[manual]` print installation instructions instead. Blocked
Android component checks resolve once the Android SDK check passes, so fix the
SDK first and re-run.

## Step 6: list your devices

```bash
water devices
```

```text
iOS Simulators
  ● iPhone 16 Pro (A1B2C3D4-...)
  ○ iPad Air (E5F6G7H8-...)

Android
  ○ Pixel_9 (emulator)

macOS
  ● Current Machine
```

A filled circle means booted or connected; an open circle means `water run`
will launch it for you. Add `--platform esp32` to list ESP32 boards on serial
ports.

## Step 7: run a generated project

```bash
water create "Hello World" --mode playground
cd hello-world
water run --platform macos
```

Substitute `--platform ios` or `--platform android` for a simulator or
emulator. The generated `src/lib.rs` is a demo screen: a stepper-driven
counter, a derived form, a slider, and progress indicators, all rendered with
native widgets. If it launches, your environment is complete.

## Optional: build caching with sccache

Cross-compiling for several architectures recompiles the same crates many
times. [sccache](https://github.com/mozilla/sccache) caches those results.

```bash
brew install sccache      # macOS
cargo install sccache     # Linux
```

The CLI detects `sccache` automatically and warns when it is absent:

```text
  ⚠ sccache not found. Build efficiency may be reduced. Install with: brew install sccache
```

## Troubleshooting

**"No iOS simulators available"** -- download a runtime in Xcode under
**Settings > Platforms**.

**"Android emulator not found"** -- check `ANDROID_HOME` and create at least
one AVD (see the Android section above).

**"GTK4 not found"** -- install the Linux development packages listed above, or
run `water doctor --fix`.

**`water: command not found`** -- put the cargo bin directory on your `PATH`:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

## Next steps

Continue to [Your First App](03-first-app.md) to build a counter and meet the
patterns you will use in every WaterUI project.

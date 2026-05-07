# Installation and Setup

> **In this chapter, you will:**
> - Install Rust and the required cross-compilation targets
> - Set up platform toolchains for Apple, Android, or Linux
> - Install the Water CLI and verify everything with `water doctor`
> - Create and run your first project to confirm the full pipeline works

Before you can build native apps with WaterUI, you need a working toolchain.
This chapter walks you through every step -- from installing Rust to seeing
your first app launch on a real device or simulator. By the end, `water doctor`
will give you a clean bill of health.

> **Note:** You only need **one** target platform to get started. Pick the one
> you are most comfortable with and skip the rest for now. You can always come
> back and add more later.

## Step 1: Install Rust

WaterUI requires **Rust 1.88 or later** (edition 2024). Install Rust with
[rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

After installation, confirm the version:

```bash
rustc --version
# rustc 1.88.0 (... 2025-...)
```

If your installed version is older, update:

```bash
rustup update stable
```

### Required Rust Targets

Depending on which platforms you want to target, add the appropriate
cross-compilation targets:

```bash
# iOS (physical devices)
rustup target add aarch64-apple-ios

# iOS Simulator (Apple Silicon)
rustup target add aarch64-apple-ios-sim

# iOS Simulator (Intel)
rustup target add x86_64-apple-ios

# Android (most common)
rustup target add aarch64-linux-android

# Android (emulator on Intel/AMD)
rustup target add x86_64-linux-android

# Android (older devices)
rustup target add armv7-linux-androideabi
rustup target add i686-linux-android
```

> **Tip**: You do not need to add all targets right away. Start with the
> platform you plan to develop on first. The `water doctor --fix` command can
> install missing targets automatically.

## Step 2: Editor setup

Any editor with Rust support will work. A common starting point is
**Visual Studio Code** with the following extensions:

- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
  -- code completion, inline errors, go-to-definition
- [Even Better TOML](https://marketplace.visualstudio.com/items?itemName=tamasfe.even-better-toml)
  -- syntax highlighting for `Cargo.toml` and `Water.toml`
- [CodeLLDB](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb)
  -- native debugger for Rust

Other popular choices include **RustRover** (JetBrains), **Zed**, and
**Helix**.

## Step 3: Platform toolchains

Install the tools for every platform you intend to target. Remember, you only
need **one** to get started -- you can always add the others later.

### Apple (iOS / macOS)

**Requirements:**
- macOS (required -- Apple development tools only run on Mac)
- Xcode 16 or later
- Xcode Command Line Tools

Install Xcode from the Mac App Store, then install the command-line tools:

```bash
xcode-select --install
```

Verify the installation:

```bash
xcodebuild -version
# Xcode 16.x
# Build version ...

xcrun simctl list devices available
# Lists available simulators
```

You also need to accept the Xcode license if you have not already:

```bash
sudo xcodebuild -license accept
```

> **Warning:** If you skip the license acceptance, builds will fail with a
> cryptic error. Save yourself the debugging and run this command now.

### Android

**Requirements:**
- Android SDK (API level 24+)
- Android NDK (version 26+)
- Java Development Kit (JDK 17+)

The easiest path is to install [Android Studio](https://developer.android.com/studio),
which bundles the SDK, NDK, and an emulator. After installation:

1. Open Android Studio and go to **Settings > Languages & Frameworks > Android SDK**.
2. Under the **SDK Platforms** tab, install at least one recent API level
   (e.g. Android 14, API 34).
3. Under the **SDK Tools** tab, ensure **NDK (Side by side)** and
   **Android SDK Command-line Tools** are installed.

Set the required environment variables. Add these to your shell profile
(`~/.zshrc`, `~/.bashrc`, etc.):

```bash
export ANDROID_HOME="$HOME/Library/Android/sdk"  # macOS default
# export ANDROID_HOME="$HOME/Android/Sdk"        # Linux default

export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/<version>"
export PATH="$ANDROID_HOME/platform-tools:$ANDROID_HOME/tools/bin:$PATH"
```

Verify:

```bash
adb --version
# Android Debug Bridge version ...

emulator -list-avds
# Lists available Android Virtual Devices
```

If you do not have an AVD yet, create one through Android Studio's
**Device Manager** or via the command line:

```bash
avdmanager create avd -n Pixel_9 -k "system-images;android-34;google_apis;arm64-v8a"
```

### Linux (GTK4)

**Requirements:**
- GTK4 development libraries (4.x)
- pkg-config

On Debian/Ubuntu:

```bash
sudo apt install libgtk-4-dev pkg-config
```

On Fedora:

```bash
sudo dnf install gtk4-devel pkg-config
```

On Arch Linux:

```bash
sudo pacman -S gtk4 pkgconf
```

On macOS (for running GTK4 apps locally):

```bash
brew install gtk4 pkg-config
```

Verify:

```bash
pkg-config --modversion gtk4
# 4.x.x
```

## Step 4: Install the Water CLI

Clone the WaterUI repository and install the CLI:

```bash
git clone https://github.com/water-rs/waterui.git
cd waterui
cargo install --path cli --locked
```

Verify the installation:

```bash
water --help
```

## Step 5: Verify with `water doctor`


This is the moment of truth. Run the doctor command to check your entire
toolchain at once:

```bash
water doctor
```

You will see output like:

```text
Checking development environment...
  ✓ Rust toolchain
  ✓ Xcode Command Line Tools
  ✓ iOS Simulator SDK
  ✓ Android SDK
  ✓ Android NDK
  ✓ GTK4
  ✓ sccache

All checks passed!
```

If any checks fail, items marked `[fixable]` can be repaired automatically:

```bash
water doctor --fix
```

For items that cannot be auto-fixed, the doctor output includes instructions
for manual installation.

> **Tip:** Bookmark this command. It is your first line of defense whenever
> something goes wrong with your build environment.

## Step 6: Discover your devices

See which simulators, emulators, and physical devices are available:

```bash
water devices
```

Example output:

```text
iOS Simulators
  ● iPhone 16 Pro (A1B2C3D4-...)
  ○ iPad Air (E5F6G7H8-...)

Android
  ○ Pixel_9 (emulator)

macOS
  ● Current Machine
```

Booted/connected devices are marked with a filled circle. Devices that need
to be launched first are marked with an open circle. The `water run` command
handles launching automatically.

## Step 7: Create your first project

With everything in place, confirm the full pipeline works end-to-end.
Create a playground project and run it:

```bash
water create "Hello World" --mode playground
cd hello-world
water run --platform macos
```

If you have an iOS Simulator available:

```bash
water run --platform ios
```

Or Android:

```bash
water run --platform android
```

You should see the default WaterUI demo application running on your chosen
platform. The demo includes a counter, a form, controls, and a progress
indicator -- all rendered with native platform widgets.

> **Tip:** If the app launches successfully, your environment is fully set
> up. If it does not, check the terminal output for errors and re-run
> `water doctor` to diagnose the issue.

## Optional: build caching with sccache

WaterUI projects cross-compile for multiple architectures, which means you
often recompile the same crates. [sccache](https://github.com/mozilla/sccache)
caches compilation results and can significantly speed up rebuilds.

```bash
# macOS
brew install sccache

# Linux
cargo install sccache
```

The Water CLI detects `sccache` automatically and uses it when available. If
it is not found, you will see a warning:

```text
  ⚠ sccache not found. Build efficiency may be reduced. Install with: brew install sccache
```

## Troubleshooting

### "No iOS simulators available"

You need to download a simulator runtime in Xcode:

1. Open Xcode.
2. Go to **Settings > Platforms**.
3. Click the **+** button and download an iOS Simulator runtime.

### "Android emulator not found"

Ensure `ANDROID_HOME` is set correctly and that you have at least one AVD
created. See the Android section above.

### "GTK4 not found"

Install the GTK4 development libraries for your operating system. See the
Linux section above. On macOS, `brew install gtk4` is required if you want
to use the GTK4 backend.

### Permission denied when running `water`

Make sure the cargo bin directory is on your `PATH`:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

## Next steps

Your development environment is ready. Continue to
[Your First App](03-first-app.md) to build a counter application step by
step and learn the fundamental WaterUI patterns along the way.

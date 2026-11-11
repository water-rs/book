# Installation and setup

> **In this chapter, you will:**
> - Install the prebuilt `water` CLI with one command
> - Let `water doctor --fix` install and repair the rest of the toolchain
> - Know which pieces the doctor cannot install for you
> - Confirm that your simulators, emulators and devices are visible to `water`

A WaterUI project needs a Rust toolchain at a pinned version, a set of
cross-compilation targets, and, per platform, an SDK: Xcode for Apple, the
Android SDK and NDK, a JDK, system libraries on Linux, the MSVC linker on
Windows. Installing and version-matching those by hand is where most first
builds fail, so the CLI owns that work. You install one binary; it diagnoses
everything else and installs what it can.

## Step 1: install the CLI

The installer downloads a prebuilt `water` binary and verifies its SHA-256
checksum before unpacking it. Nothing is compiled.

```bash
# macOS and Linux
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/water-rs/waterui/releases/download/cli-latest/waterui-cli-installer.sh | sh
```

```powershell
# Windows
powershell -ExecutionPolicy Bypass -c "irm https://github.com/water-rs/waterui/releases/download/cli-latest/waterui-cli-installer.ps1 | iex"
```

The release builds cover Apple Silicon macOS, Linux on x86_64 and aarch64
(glibc 2.28 or newer, plus a musl build for x86_64), and Windows on x86_64 and
arm64. Intel Macs are not supported. Confirm the install:

```bash
water --version
```

`water update` upgrades the CLI in place, and `water update --check` reports
the newest release without installing it. See [The Water CLI](01-cli.md) for the
full command tour.

## Step 2: install Rust

The doctor installs the Rust toolchain only through `rustup`, so rustup is the
one Rust piece you install yourself. Use [rustup.rs](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Without rustup on `PATH`, the doctor cannot install, update or add targets to
Rust, so every such repair is reported as `[manual]` and points you back to
this step. With it, you do not install a specific Rust
version or any target yourself. The doctor reads the floor from three places and
enforces the highest: the CLI's own `rust-version`, your project's `Cargo.toml`,
and the `rust-version` of the framework your project uses (the framework at
this book's revision declares 1.99, on edition 2024). It also honors a
`rust-toolchain.toml` pin in your project, and installs the pinned channel plus
any `components` and `targets` the file lists.

## Step 3: run the doctor

```bash
water doctor
```

The doctor prints a heading per group, with the items under it:

| Group | What it covers |
| --- | --- |
| Rust toolchain | rustup, the active toolchain and its version floor |
| Host tools | `git`, a C toolchain on Linux, long-path support on Windows |
| Apple (iOS, macOS) | Xcode, SDKs, simulators, the Apple Rust targets, the Metal Toolchain |
| Hydrolysis (desktop, web) | Linux system packages, MSVC, DXC and ARM64 LLVM on Windows, `spirv-opt`, the `wasm32` target, `wasm-pack`, and the web package manager your `Water.toml` declares |
| WinUI | The experimental WinUI backend's prerequisites, on Windows |
| GTK4 | The experimental GTK4 backend's libraries, on Linux |
| Android | SDK, platform-tools, platforms, build-tools, NDK, Rust targets, run targets, CMake, JDK, Kotlin |
| Build helpers | `sccache`, `cargo-nextest` |

Each item is reported as passing, `[fixable]`, `[manual]`, or skipped. A
`[fixable]` item has an installer behind it; a `[manual]` item prints the step
only you can take. Skipped items do not apply to your host, such as the Apple
group on Linux.

### Run it inside your project

Which groups count as required depends on where you run the command. Inside a
project (a directory with a `Water.toml`), every backend and platform is in
scope, because the CLI can build your project for any of them; a group that
does not apply to your host still reports its items as skipped. Outside a
project, only the backends your host builds for are required: Hydrolysis (its
web pieces included) on every host, Apple on macOS, WinUI on Windows and GTK4
on Linux. Android is a cross-compiled platform, so outside a project its group
is marked `(optional)` and its missing pieces print as hints that do not fail
the run.

So the first time through, create a project first (see [Your first
app](03-first-app.md)), then run `water doctor` from inside it to check
everything it needs.

### Fix what is fixable

```bash
water doctor --fix
```

The doctor installs each fixable item, re-runs the diagnosis, and repeats for up
to three passes, because one fix can unblock another. A failed install does not
stop the others; it is reported and left out of the retry set. Interactive runs
ask before each install, and `--yes` answers yes to every prompt. On CI or any
non-interactive shell, installs run without asking, except those that modify
the machine outside `~/.water`: the doctor states what it will change and skips
them unless you add `--yes`.

```bash
water doctor --fix --yes
```

`--json` is a global flag; `water --json doctor` emits one record per item for
scripts.

## What the doctor installs

### On every host

- **The Rust toolchain.** `rustup default`, `rustup toolchain install` or
  `rustup update` as needed to meet the floor or the project pin, plus the
  targets and components the build requires.
- **Apple Rust targets** on macOS: `aarch64-apple-ios`,
  `aarch64-apple-ios-sim` and `aarch64-apple-ios-macabi`, the Mac Catalyst
  target that `water preview --platform ios` builds for. Apple support is ARM64-only. Intel macOS and Intel
  iOS simulator targets are retired and are neither installed nor accepted.
- **`sccache`**, the compile cache, at 0.9.0 or newer. It installs through
  Homebrew on macOS or winget on Windows when either is present; otherwise, and
  always on Linux, the doctor unpacks a pinned, checksum-verified release under
  `~/.water/tools`, so it needs no package manager.
- **`cargo-nextest`**, which `water bench` uses, through `cargo binstall` when
  you have it and `cargo install --locked` otherwise.
- **For the web backend**: the `wasm32-unknown-unknown` target and `wasm-pack`
  (installed with `cargo install --locked wasm-pack`). When your `Water.toml`
  has a `[web]` section, the doctor also checks the JavaScript package manager
  it declares: it installs bun or pnpm with their official install scripts and
  enables yarn through `corepack`. npm ships with Node.js, so a missing npm
  stays manual.

### macOS

You install Xcode yourself: from the Mac App Store, then selected with
`xcode-select`. The Apple backend's deployment floor is iOS 26 and macOS 26, so
you need **Xcode 26 or newer**. The doctor cannot install Xcode, and its report
for the Xcode, iOS SDK, iOS Simulator SDK, macOS SDK and iOS Simulators items is
`[manual]` until you do. You also download a missing iOS simulator runtime
yourself, in Xcode under **Settings > Platforms**.

The doctor does install the **Metal Toolchain**, the separately downloaded Xcode
component that the engine's shader build needs, with
`xcodebuild -downloadComponent MetalToolchain`. The component lands inside the
Xcode application, so on a non-interactive run it needs `--yes`.

Homebrew is the doctor's package manager on macOS. Android Studio, a JDK, CMake
and `spirv-opt` install through it, so with no Homebrew those items become
`[manual]`.

### Android

The doctor installs, in dependency order:

1. **The Android SDK root.** On macOS it installs Android Studio through
   Homebrew, on Windows through winget. On Linux it installs the command-line
   tools into the SDK directory that `ANDROID_SDK_ROOT` or `ANDROID_HOME` names,
   and into `~/Android/Sdk` when neither is set.
2. **Platform-tools (`adb`), SDK platforms, build-tools and the NDK** through
   `sdkmanager`. The NDK version is pinned by the CLI (29.0.14206865 at this
   revision). Until the SDK item passes, these show as blocked, and the "Android
   Run Targets" item stays blocked until `adb` is installed; the `--fix` loop
   re-diagnoses after each pass, so they resolve on the next one.
3. **The Android Rust targets** through rustup.
4. **A JDK 21**: Homebrew on macOS, winget or a pinned Temurin archive under
   `~/.water/tools` on Windows, and your distribution's package manager on
   Linux.
5. **CMake and Kotlin**, which the Android build uses.

One thing is still yours: **the doctor does not create an emulator.** The
"Android Run Targets" item passes when a device is connected or at least one AVD
exists, and otherwise tells you to connect a device or create an AVD. Create
one in Android Studio's Device Manager or with `avdmanager`, or plug in a phone
with USB debugging enabled.

### Linux

The doctor detects your package manager (apt, dnf, pacman, zypper or apk),
works out which of the desktop build's system libraries are missing, and
installs them with the right command for your distribution, through `sudo` when
it is available. It also checks that `cc` and `ld` exist, since rustc links
through them. Run `water doctor` first to see the exact package list and command
it would run, and `--fix` to run it. You never type the package list yourself.

The package list includes the GTK4 development libraries, because the doctor
also probes the experimental GTK4 backend (Tier 3). Its group counts as required
on Linux, both inside a project and outside one, even though the supported Linux
desktop path is Hydrolysis. See [Platforms](05-platforms.md) for support tiers.

### Windows

- **MSVC C++ build tools**, for `link.exe`. This is a system-wide install of the
  Visual Studio Build Tools C++ workload: a large download that needs
  administrator rights, which is why the doctor asks for `--yes` outside an
  interactive shell.
- **DXC**, the DirectX shader compiler, unpacked into `~/.water/tools`.
- **Long-path support**, a machine setting that checkouts nested in the build
  cache require. The fix is an elevated registry write, so expect a UAC prompt,
  and it too needs `--yes` on a non-interactive run.
- **LLVM tools on Windows ARM64**, needed by native assembly dependencies,
  through winget or the LLVM MSI installer. The MSI writes outside `~/.water`,
  so it follows the same `--yes` rule.

Windows does not support dynamic linking, so development builds link statically
there. See [Platforms](05-platforms.md) for what runs there.

## What stays manual

| Piece | Why |
| --- | --- |
| rustup | The doctor repairs toolchains through rustup but never installs or reinstalls it, and never edits shell profiles |
| Xcode | Only the App Store or Apple Developer site supplies it |
| iOS simulator runtimes | Downloaded in Xcode Settings |
| An Android emulator (AVD) or device | A hardware or personal choice |
| Homebrew, winget | The doctor uses them; it does not bootstrap them |
| `spirv-opt` on Windows | No package exists for winget; the doctor prints the MSYS2 route instead |

When the only items left are manual, `water doctor --fix` says nothing is
fixable automatically and the report tells you what to do for each.

## Step 4: list your devices

```bash
water devices
```

`water devices` prints a section per platform. An iOS section lists paired
physical devices and simulators; the Android section lists AVDs and connected
phones; macOS always lists the current machine. A filled circle marks a booted
simulator or emulator and a reachable device. An open circle marks a simulator
or AVD that is shut down, which `water run` boots, or a paired iPhone that is
not reachable right now.

```bash
water devices --platform ios      # iOS devices and simulators
water devices --platform android  # AVDs and connected phones
water devices --platform macos    # the current machine
water --json devices              # machine-readable
```

Without `--platform` (`-p`), `water devices` scans iOS and Android together, so
it needs both `adb` and Xcode's `simctl`: on a Mac without the Android SDK, or on
Linux or Windows, it stops with an error. Pass the one platform you have there.
Pass the name or identifier it prints to `water run --device <id>`.

## Step 5: editor setup

Any editor with Rust support works. For Visual Studio Code, install
[rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
and
[Even Better TOML](https://marketplace.visualstudio.com/items?itemName=tamasfe.even-better-toml)
for `Cargo.toml` and `Water.toml`. RustRover, Zed and Helix work too.

## Troubleshooting

**`water: command not found`**: the installer puts the binary in your cargo bin
directory. Add `$HOME/.cargo/bin` to `PATH` and open a new shell.

**The Rust item stays `[manual]`**: rustup is missing or its `cargo` and `rustc`
proxies are not on `PATH`. The doctor does not repair either; go back to Step 2.

**An Android item says "Blocked"**: the Android SDK item (or, for "Android Run
Targets", platform-tools) has not passed yet. If `water doctor --fix` ran out of
passes before reaching it, run it again.

**No iOS simulators**: download a runtime in Xcode under **Settings > Platforms**.

## Next steps

Continue to [Your first app](03-first-app.md) to create a project and run it,
then come back and run `water doctor` inside it.

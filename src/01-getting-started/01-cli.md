# The Water CLI

> **In this chapter, you will:**
> - Install the `water` command-line tool
> - Choose between playground and app project modes
> - Learn `create`, `run`, `build`, `package`, `preview`, and the maintenance commands

`water` is the single entry point for WaterUI projects: scaffolding, cross-
compiling for mobile and embedded targets, launching on simulators and devices,
rendering previews, and packaging for distribution. It wraps Xcode, Gradle,
GTK4, and ESP-IDF build systems so you stay in Rust.

## Installation

From crates.io:

```bash
cargo install waterui-cli
```

Or from a WaterUI checkout, which is what you want if you also work on the
framework:

```bash
cargo install --path cli --locked
```

Verify:

```bash
water --help
```

> **Tip**: While iterating on the CLI itself, `cargo build -p waterui-cli` is
> much faster than a full install. Reinstall when you need the new binary on
> your `PATH`.

## Project modes

### Playground mode

Playground mode manages every native backend project for you, inside the global
build cache at `~/.water/build_cache/<absolute-project-path>/managed_backends/`.
You write Rust and nothing else.

```bash
water create "My Experiment" --mode playground
```

`water create` writes four files and initialises a git repository if the
directory is not already inside one:

```text
my-experiment/
  Cargo.toml
  Water.toml          # type = "playground"
  src/lib.rs
  .gitignore
```

Playground projects auto-initialise their backends on every `water run` and
re-scaffold templates so manifest changes (permissions, theme colours) are
always picked up. Nothing platform-specific lands in your working tree.

> **Tip:** Playground mode is what you want while following this book.

### App project mode

App mode (the default) checks the native projects into your repository under
`backends/`, so you can edit Xcode settings, add Swift or Kotlin sources, and
wire the projects into CI.

```bash
water create "Production App" --backends apple,android
```

```text
production-app/
  Cargo.toml
  Water.toml          # type = "app"
  src/lib.rs
  .gitignore
  backends/
    apple/            # Swift package, checked in
    android/          # Gradle project, checked in
    ffi/              # Generated FFI companion crate
```

Only the backends you asked for are scaffolded. `water backend add gtk4` adds
another one later.

## Command reference

### `water create`

```bash
# Interactive: prompts for name, bundle id, and backends
water create

# Playground
water create "Counter" --mode playground

# App with explicit backends
water create "My App" --backends apple,android

# Custom bundle identifier
water create "My App" --bundle-id dev.waterui.myapp --backends apple

# Link to a local WaterUI checkout (framework development)
water create "Dev App" --waterui-path ../waterui --backends apple
```

| Argument | Description |
|----------|-------------|
| `name` | Display name. The folder is its kebab-case form, the crate its snake_case form. |
| `--bundle-id` | Bundle identifier. Defaults to `dev.waterui.<snake_case_name>`. |
| `--backends` | Comma-separated: `apple`, `android`, `gtk4`, `hydrolysis`, `esp32`. App mode only. |
| `--mode` | `app` (default) or `playground`. |
| `--waterui-path` | Path to a local WaterUI checkout. |

`--backends` accepts aliases: `ios`/`macos` map to `apple`, `gtk`/`linux` to
`gtk4`, and `esp32s3`/`dew` to `esp32`. In app mode with no `--backends` and no
prompt, you get `apple,android`.

Host restrictions apply at scaffold time: GTK4 backends require a Linux host,
and Hydrolysis requires macOS, Linux, or Windows.

### `water run`

Builds, packages, and launches in one step. This is the command you will use
most.

```bash
water run --platform ios
water run --platform ios --device "iPhone 16 Pro"
water run --platform android
water run --platform macos
water run --platform macos --backend hydrolysis
water run --platform linux                 # GTK4 by default
water run --platform windows
water run --platform esp32c3               # Dew firmware; --device qemu to emulate
water run --platform ios --logs debug
water run --platform ios --logs debug --native-logs
```

Omit `--platform` and `water run` targets the host: `macos`, `linux`, or
`windows`.

| Argument | Description |
|----------|-------------|
| `--platform`, `-p` | `ios`, `android`, `macos`, `linux`, `windows`, `web`, `esp32s3`, `esp32c3`. Defaults to the host. |
| `--backend`, `-b` | `apple`, `android`, `gtk4`, `hydrolysis`, `dew`. Overrides the platform default. |
| `--device`, `-d` | Device name or identifier. Defaults to the first booted or available device. |
| `--path` | Project directory (defaults to `.`). |
| `--logs` | Minimum level to stream: `error`, `warn`, `info`, `debug`, `verbose`. |
| `--native-logs` | Include all native logs (`NSLog`, `logcat`), not just WaterUI's. |

Platform defaults and the combinations the CLI accepts:

| Platform | Default backend | Also accepts |
|----------|----------------|--------------|
| iOS | Apple | — |
| macOS | Apple | Hydrolysis |
| Android | Android | — |
| Linux | GTK4 | Hydrolysis |
| Windows | Hydrolysis | — |
| Web | Hydrolysis | — |
| ESP32-S3 / ESP32-C3 | Dew | — |

For app-mode projects the default is the first configured backend in that
priority order, so a project with only a Hydrolysis backend runs on Hydrolysis
without `--backend`.

`tracing::debug!` output only reaches your terminal with `--logs debug`.

### `water build`

Compile the Rust library for a target without packaging or launching -- useful
in CI and as the step Xcode and Gradle call. App-mode projects only; playground
projects go through `water run` and `water package`.

```bash
water build --platform ios
water build --platform ios-simulator --arch arm64
water build --platform android --arch arm64
water build --platform macos --release
water build --platform macos --output-dir ./out
water build --platform esp32s3 --release
```

| Argument | Description |
|----------|-------------|
| `--platform`, `-p` | `ios`, `ios-simulator`, `android`, `macos`, `linux`, `windows`, `esp32s3`, `esp32c3`. |
| `--backend`, `-b` | `apple`, `android`, `gtk4`, `hydrolysis`, `dew`. |
| `--arch`, `-a` | `arm64`, `x86-64`, `armv7`, `x86`. Apple and Android backends only. |
| `--release` | Optimised build. |
| `--path` | Project directory (defaults to `.`). |
| `--output-dir` | Copy the built library here. Apple and Android backends only. |

### `water package`

Produce installable artifacts. `--backend` is required.

```bash
water package --platform ios --backend apple
water package --platform ios --backend apple --release --distribution
water package --platform android --backend android --arch arm64
water package --platform android --backend android --arch arm64,x86-64
```

| Argument | Description |
|----------|-------------|
| `--platform`, `-p` | `ios`, `ios-simulator`, `android`, `macos`, `linux`, `windows`, `web`. |
| `--backend`, `-b` | Required: `apple`, `android`, `gtk4`, `hydrolysis`. |
| `--release` | Optimised build. |
| `--distribution` | Package for store submission. |
| `--arch` | Android architectures, comma-separated: `arm64`, `x86-64`, `armv7`, `x86`. Required for Android. |
| `--path` | Project directory (defaults to `.`). |

ESP32 firmware is flashed by `water run --platform esp32s3|esp32c3`, not
packaged.

### `water preview`

Render a view function to PNG without launching the app.

```bash
water preview my_card --platform macos --path ./app
water preview dashboard --platform ios --frame 390x844
water preview login_screen --output login.png
water preview 'text("inline").bold()' --expr
```

The target is a `#[preview]` function path, or -- with `--expr` -- a WaterUI
expression that evaluates to `impl View`:

```rust,ignore
use waterui::prelude::*;

#[preview]
fn my_card() -> impl View {
    text("Hello Preview!")
}
```

| Argument | Description |
|----------|-------------|
| `target` | `#[preview]` function name or path, or an expression with `--expr`. |
| `--expr` | Treat the target as an expression rather than a function path. |
| `--platform`, `-p` | `ios`, `macos`, `android`. Defaults to the native preview platform. |
| `--backend` | `apple`, `android`, `hydrolysis`. |
| `--theme` | `material3`. Hydrolysis previews only. |
| `--frame`, `-f` | `WIDTHxHEIGHT` (default `375x667`). |
| `--output`, `-o` | Output file (default `preview.png`). |
| `--scenario` / `--output-dir` | Hydrolysis scenario TOML for interaction capture, and where to write its frames. |
| `--path` | Project directory (defaults to `.`). |

Two subcommands share the same surface: `water preview test` runs semantic
assertions against a preview (add `--all` to sweep every `#[preview]` in the
crate), and `water preview perf` profiles it through the offscreen GPU
pipeline. Preview symbols are `waterui_preview_<crate_name>_<function_name>`,
so names must be unique within a crate.

### `water doctor`

```bash
water doctor
water doctor --fix
```

The doctor probes the Apple toolchain (Xcode, iOS and macOS SDKs, installed
simulators), the Rust toolchain and cross-compilation targets, the Android SDK
and its components (platform-tools/`adb`, SDK platforms, build-tools, NDK, Rust
Android targets, and at least one device or AVD), host tooling (CMake, Java,
Kotlin, the `wasm32-unknown-unknown` target, `wasm-pack`), Linux system packages
and GTK4 on Linux hosts, and `sccache`.

Checks are reported as `[fixable]` or `[manual]`. `--fix` installs the fixable
ones; anything else prints manual instructions. Checks for platforms your host
cannot serve are skipped rather than failed.

> **Tip:** Run `water doctor` first whenever a build fails in a way that does
> not look like your code.

### `water devices`

```bash
water devices
water devices --platform ios
water devices --platform android
water devices --platform esp32          # ESP32 boards on serial ports
water --json devices --platform all     # --json is a global flag
```

Output lists each device's name, identifier, and state.

### `water device`

Drive a running app for automation and screenshots:

```bash
water device capture --id <udid>
water device tap --id <udid> --x 100 --y 200
water device swipe --id <udid> --from 100,600 --to 100,200
water device text --id <udid> --input "hello"
water device describe --id <udid>       # dump on-screen UI elements
```

### `water backend`

App-mode projects only:

```bash
water backend list
water backend add gtk4
water backend remove android --yes
```

Accepted names are `apple`, `android`, `gtk4`, `hydrolysis`, and `esp32`.

### `water clean`

```bash
water clean                             # all backends in this project
water clean --backend apple
water clean --recursive --path ~/projects
water clean --recursive --yes
water clean --global-cache --yes        # wipe ~/.water/build_cache
```

`--backend` takes `apple`, `android`, `gtk4`, `hydrolysis`, or `all` (the
default). In recursive mode the CLI finds every directory holding a valid
`Water.toml` and clears each playground's managed cache or each app project's
`target/`.

### `water gc`

```bash
water gc build-cache
```

Removes stale entries from `~/.water/build_cache`, keeping the project at
`--path` (default `.`) marked active.

### `water inspector`

Attach the inspector app to a running WaterUI runtime:

```bash
water inspector --target 127.0.0.1:9229
```

## Next steps

Continue to [Installation and Setup](02-setup.md) to configure a platform
toolchain, or jump to [Your First App](03-first-app.md) if `water doctor`
already passes.

# The Water CLI

> **In this chapter, you will:**
> - Install the `water` command-line tool
> - Understand the difference between playground and app project modes
> - Learn the key commands: `create`, `run`, `build`, `package`, and more

Every WaterUI project starts with the `water` CLI. It is your single entry
point for creating projects, compiling Rust for mobile and desktop targets,
launching apps on simulators and devices, and packaging for distribution. Think
of it as `cargo` for cross-platform native apps -- it wraps the complexity of
Xcode, Gradle, and GTK4 build systems so you can focus on writing Rust.

## Installation

The CLI is part of the WaterUI repository. Install it from source:

```bash
cargo install --path cli --locked
```

After installation, verify the tool is on your `PATH`:

```bash
water --help
```

> **Tip**: If you are actively developing the CLI itself, use
> `cargo build -p waterui-cli` for faster iteration, then reinstall with
> `cargo install --path cli` when you need the updated binary in your `PATH`.

## Project modes

WaterUI supports two project modes, each suited to a different stage of
development. Choosing the right one upfront will save you time.

### Playground mode

Playground mode is designed for **quick experimentation**. When you create a
project with `--mode playground`, the CLI manages all native backend projects
automatically inside the global build cache under
`~/.water/build_cache/<absolute-project-path>/managed_backends/`. You only
write Rust.

```bash
water create "My Experiment" --mode playground
```

What you get on disk:

```text
my-experiment/
  Cargo.toml
  Water.toml          # type = "playground"
  src/lib.rs
  assets/
    raw/
    images/
```

Playground projects:
- Auto-initialise Apple and Android backends on every `water run`.
- Re-scaffold backend templates automatically so manifest changes (such as
  permissions) are always picked up.
- Store all generated native projects in the global build cache, keeping your
  working directory clean and free of platform clutter.

> **Tip:** Playground mode is what you want while following this book. It keeps
> the boilerplate out of your way so you can concentrate on learning WaterUI
> itself.

### App project mode

App mode (the default) gives you **explicit control** over backend
configuration. Native backend projects live under a `backends/` directory in
your project root and are checked into version control.

```bash
water create "Production App" --backends apple,android
```

What you get:

```text
production-app/
  Cargo.toml
  Water.toml          # type = "app"
  src/lib.rs
  assets/
    raw/
    images/
  backends/
    apple/            # Swift Package, checked in
    android/          # Gradle project, checked in
    gtk4/             # GTK4 backend crate, checked in
    ffi/              # Generated FFI companion crate
```

App projects are required for:
- Customising native build settings (Xcode schemes, Gradle dependencies, etc.)
- Adding platform-specific native code
- Production deployment pipelines

Now that you understand both modes, let's look at what the CLI can do.

## Command Reference

### `water create`

Scaffold a new WaterUI project.

```bash
# Interactive mode (prompts for name, bundle ID, backends)
water create

# Playground project
water create "Counter" --mode playground

# App project with explicit backends
water create "My App" --backends apple,android

# With custom bundle identifier
water create "My App" --bundle-id dev.waterui.myapp --backends apple

# Link to a local WaterUI checkout (for framework development)
water create "Dev App" --waterui-path ../waterui --backends apple
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `name` | Project display name (for example, "Water Example"). The folder name is derived as kebab-case. |
| `--bundle-id` | Bundle identifier (defaults to `com.example.<snake_case_name>`). |
| `--backends` | Comma-separated list: `apple`, `android`, `gtk4`, `hydrolysis`. Only valid in app mode. |
| `--mode` | `app` (default) or `playground`. |
| `--waterui-path` | Path to a local WaterUI checkout (for framework development). |

When run without arguments in an interactive terminal, the CLI prompts for each
value with sensible defaults.

GTK4 app backends can only be scaffolded on Linux hosts at this checkpoint.
Hydrolysis is available for macOS, Linux, Windows, and Web.

### `water run`

This is the command you will use most often. It builds, packages, and runs the
application on a target device -- all in one step.

```bash
# Run on iOS Simulator (default device)
water run --platform ios

# Run on a specific iOS Simulator
water run --platform ios --device "iPhone 16 Pro"

# Run on Android (connected device or first emulator)
water run --platform android

# Run on macOS
water run --platform macos

# Run on macOS with the Hydrolysis renderer
water run --platform macos --backend hydrolysis

# Run on Linux (defaults to the GTK4 backend)
water run --platform linux

# Run on Windows
water run --platform windows

# Stream debug logs
water run --platform ios --logs debug

# Include native platform logs (verbose)
water run --platform ios --logs debug --native-logs
```

If you omit `--platform`, `water run` defaults to the host platform: `macos`
on macOS, `linux` on Linux, and `windows` on Windows.

**Arguments:**

| Argument | Description |
|----------|-------------|
| `--platform`, `-p` | Target platform: `ios`, `android`, `macos`, `linux`, `windows`, `web`. Defaults to the host platform. |
| `--backend`, `-b` | Override the default backend for the platform. |
| `--device`, `-d` | Device name or identifier. If omitted, uses the first booted or available device. |
| `--path` | Project directory (defaults to `.`). |
| `--logs` | Minimum log level to stream: `error`, `warn`, `info`, `debug`, `verbose`. |
| `--native-logs` | Include all native logs (`NSLog`, Android `logcat`), not just WaterUI logs. |

The default backend for each platform is:

| Platform | Default backend |
|----------|----------------|
| iOS | Apple |
| macOS | Apple |
| Android | Android |
| Linux | GTK4 |
| Windows | Hydrolysis |
| Web | Hydrolysis |

Valid backend/platform combinations:

| Backend | Supported platforms |
|---------|---------------------|
| Apple | iOS, macOS |
| Android | Android |
| GTK4 | Linux |
| Hydrolysis | macOS, Linux, Windows, Web |

> **Note:** If you have multiple simulators or emulators available, `water run`
> picks the first booted one. Use `--device` to target a specific device by
> name.

### `water build`

Compile the Rust library for a target platform without packaging or running.
This is useful in CI pipelines or when you want to check compilation without
launching an app. `water build` only operates on app-mode projects; playground
projects are built and packaged via `water run` and `water package`.

```bash
# Build for iOS device
water build --platform ios

# Build for iOS Simulator (specific architecture)
water build --platform ios-simulator --arch arm64

# Build for Android
water build --platform android --arch arm64

# Release build
water build --platform macos --release

# Build and copy to a specific output directory
water build --platform macos --output-dir ./out
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `--platform`, `-p` | Target: `ios`, `ios-simulator`, `android`, `macos`, `linux`, `windows`. |
| `--backend`, `-b` | Backend override. |
| `--arch`, `-a` | Architecture: `arm64`, `x86_64`, `armv7`, `x86`. Apple/Android backends only. |
| `--release` | Build in release mode. |
| `--path` | Project directory (defaults to `.`). |
| `--output-dir` | Copy the built library to this directory (Apple/Android backends only). |

### `water package`

Package the application for distribution. When you are ready to ship, this is
how you produce installable artifacts. `--backend` is required.

```bash
# Package for iOS (physical device)
water package --platform ios --backend apple

# Release build for distribution
water package --platform ios --backend apple --release --distribution

# Package for Android (must specify architecture)
water package --platform android --backend android --arch arm64

# Package for Android (multiple architectures)
water package --platform android --backend android --arch arm64,x86_64
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `--platform`, `-p` | Target: `ios`, `ios-simulator`, `android`, `macos`, `linux`, `windows`, `web`. |
| `--backend`, `-b` | Required. Backend to package with. |
| `--release` | Build in release mode (optimised). |
| `--distribution` | Package for store distribution (App Store, Play Store). |
| `--arch` | Target architecture(s) for Android (comma-separated). Required for Android. |
| `--path` | Project directory (defaults to `.`). |

### `water preview`

Render a view function to a PNG image without launching the full application.
This is useful for visual testing and documentation.

```bash
# Preview a function on macOS
water preview my_card --platform macos --path ./app

# Custom frame size
water preview dashboard --platform ios --frame 390x844

# Custom output path
water preview login_screen --platform macos --output login.png
```

The function must be annotated with the `#[preview]` attribute macro:

```rust,ignore
use waterui::prelude::*;

#[preview]
fn my_card() -> impl View {
    text("Hello Preview!")
}
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `function_path` | Function name or path (for example, `dashboard::admin::card`). |
| `--platform`, `-p` | Target: `ios`, `macos`, `android`. |
| `--backend` | `apple`, `android`, or `hydrolysis`. Defaults to the platform's native preview backend. |
| `--frame`, `-f` | Frame size as `WIDTHxHEIGHT` (default: `375x667`). |
| `--output`, `-o` | Output file path (default: `preview.png`). |
| `--path` | Project directory (defaults to `.`). |

### `water doctor`

Not sure if your environment is set up correctly? `water doctor` checks
everything for you.

```bash
# Check toolchain
water doctor

# Attempt to fix missing dependencies automatically
water doctor --fix
```

The doctor checks for:
- Rust toolchain and required targets
- Xcode and command-line tools (macOS)
- Android SDK and NDK
- GTK4 development libraries
- `sccache` (optional, for build caching)

Items marked `[fixable]` can be installed automatically with `--fix`.

> **Tip:** Run `water doctor` any time something does not compile as expected.
> It often catches missing targets or outdated toolchains before you start
> debugging your own code.

### `water devices`

List available simulators, emulators, and connected devices.

```bash
# List all devices across all platforms
water devices

# List only iOS simulators
water devices --platform ios

# List only Android devices and emulators
water devices --platform android

# JSON output (for scripting). --json is a global flag.
water --json devices --platform all
```

The output shows each device's name, identifier, and state (booted/available).

### `water clean`

Remove build artifacts.

```bash
# Clean all backends in the current project
water clean

# Clean only the Apple backend
water clean --backend apple

# Clean only the Android backend
water clean --backend android

# Recursively clean all WaterUI projects under a directory
water clean --recursive --path ~/projects

# Skip confirmation in recursive mode
water clean --recursive --yes

# Wipe the global managed build cache under ~/.water/build_cache
water clean --global-cache --yes
```

In recursive mode, the CLI finds every directory containing a valid
`Water.toml` and clears each project's managed build cache (for playgrounds)
or `target/` directory (for app projects).

### `water gc`

Garbage-collect stale entries in the global managed build cache. Run this if
playground caches under `~/.water/build_cache/` have piled up across many
abandoned projects.

```bash
water gc build-cache
```

## Next steps

With the CLI installed, continue to [Installation and Setup](02-setup.md) to
configure your platform toolchains, or jump straight to
[Your First App](03-first-app.md) if you already have everything in place.

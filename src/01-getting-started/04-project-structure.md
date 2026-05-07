# Project structure and Water.toml

> **In this chapter, you will:**
> - Understand how playground and app projects are laid out on disk
> - Learn every section of the `Water.toml` manifest
> - Discover how assets, fonts, and permissions are managed
> - Know when to switch from playground to app project mode

Every WaterUI project follows a consistent layout. Understanding this
structure early will save you time when you need to add assets, configure
permissions, or prepare for production. This chapter covers both project
modes, the `Water.toml` and `Cargo.toml` manifests, and the asset system.

## Playground project layout

When you create a project with `--mode playground`, the on-disk layout is
minimal. This is the mode you have been using throughout this tutorial:

```text
my-app/
  Cargo.toml           # Rust crate configuration
  Water.toml           # WaterUI project manifest
  src/
    lib.rs             # Your application code
  assets/
    raw/               # Arbitrary files (JSON, fonts, data)
    images/            # Image resources
```

The generated native backend projects live **outside** your project tree, in
the global managed cache at:

```text
~/.water/build_cache/<absolute-project-path>/managed_backends/
  apple/               # Generated Apple backend (Swift Package)
  android/             # Generated Android backend (Gradle project)
  ffi/                 # Generated FFI companion crate
  preview_ffi/         # Generated preview wrapper crate
```

Key characteristics:

- **You only edit Rust files and assets.** The native backend projects in
  the global cache are generated and managed by the CLI.
- **The cache is rebuilt on every `water run`.** Changes to `Water.toml`
  (such as adding permissions) flow into the native projects automatically.
- **Backend configuration is not allowed in `Water.toml`.** The `[backends]`
  section must be absent for playground projects.
- **Permissions are configured in `Water.toml`.** The `[permissions]`
  section is only available in playground mode.

> **Tip:** Playground mode is ideal for learning, prototyping, and following
> this book's examples. You do not need to think about native build systems
> at all. To reclaim disk space across abandoned playgrounds, run
> `water gc build-cache` or `water clean --global-cache --yes`.

## App project layout

When you need more control -- custom Xcode settings, platform-specific
native code, or CI/CD integration -- create a project with explicit
`--backends`. The native projects live inside your repository under a
`backends/` directory:

```text
my-app/
  Cargo.toml
  Water.toml
  src/
    lib.rs
  assets/
    raw/
    images/
  backends/
    apple/             # Swift Package (checked in)
      Package.swift
      Sources/
      ...
    android/           # Gradle project (checked in)
      app/
      build.gradle.kts
      ...
    gtk4/              # GTK4 backend crate (checked in)
    ffi/               # FFI companion crate (checked in)
```

Key characteristics:

- **Backend directories are version-controlled.** You can customise native
  build settings, add platform-specific code, and manage backend
  dependencies.
- **The `[backends]` section in `Water.toml`** tracks which backends are
  configured and their per-backend settings.
- **Permissions are managed in native projects directly** (`Info.plist` for
  Apple, `AndroidManifest.xml` for Android).

Now let's look at the configuration files that tie everything together.

## Water.toml

The `Water.toml` file is the central configuration for a WaterUI project. It
is a TOML file with the following sections.

### `[package]`

The `[package]` section defines the application identity:

```toml
[package]
type = "playground"          # or "app"
name = "My Application"
bundle_identifier = "com.example.myapp"
```

**Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `type` | `"playground"` or `"app"` | Project mode. Playground auto-manages backends; app requires explicit backend directories. |
| `name` | string | Human-readable application name displayed in the OS. |
| `bundle_identifier` | string | Unique identifier (reverse domain notation). Used for iOS bundle ID and Android application ID. |
| `assets_path` | string | Path to the assets directory relative to project root. Defaults to `"assets"`. Omitted from the file when it equals the default. |
| `accessory` | boolean | When `true`, builds a headless (accessory) app on macOS -- no dock icon, no menu bar. Defaults to `false`. Omitted from the file when `false`. |

### `[backends]`

The `[backends]` section is only present in app (`type = "app"`) projects.
It is populated when you run `water create` with `--backends`, or when you
add a backend to an existing project with `water backend add <name>`.

```toml
[backends]
path = "backends"            # Base path for backend directories (relative to project root)

[backends.apple]
# Apple backend configuration (auto-generated)

[backends.android]
# Android backend configuration (auto-generated)

[backends.gtk4]
# GTK4 backend configuration (auto-generated)
```

For playground projects, this section must be absent. The CLI stores backend
data in the global build cache instead.

> **Warning:** Adding a `[backends]` section to a playground project or a
> `[permissions]` section to an app project causes the CLI to reject the
> manifest with an error. Each mode has its own configuration approach.

### `waterui_path`

For framework developers who work on WaterUI itself, the `waterui_path`
field points to a local checkout of the WaterUI repository:

```toml
waterui_path = "../waterui"
```

When set, all backends use this local path instead of published crate
versions. The CLI sets this automatically when you create a project with
`--waterui-path`.

### `[permissions]`

The `[permissions]` section is **only available in playground mode**. It
provides a declarative way to request native platform permissions without
editing native project files:

```toml
[permissions.camera]
enable = true
description = "Required for barcode scanning"

[permissions.location]
enable = true
description = "Used to show nearby stores"

[permissions.microphone]
enable = true
description = "Needed for voice recording"
```

Each permission entry has two fields:

| Field | Type | Description |
|-------|------|-------------|
| `enable` | boolean | Whether to request this permission. |
| `description` | string | A user-facing explanation of why the permission is needed. This text appears in the system permission dialog. |

When `water run` rebuilds a playground project, it reads these permissions and
injects the appropriate entries into `Info.plist` (Apple) and
`AndroidManifest.xml` (Android) automatically.

For app projects (`type = "app"`), permissions are managed directly in the
native project files. Attempting to use `[permissions]` in an app project
causes the CLI to reject the manifest with an error.

> **Note:** Always write clear, user-facing descriptions for permissions.
> Vague descriptions like "We need this" will get your app rejected from
> app stores. Explain *why* the permission is needed in terms the user
> understands.

## Cargo.toml

The `Cargo.toml` file is a standard Rust crate manifest. When `water create`
scaffolds a project, it generates a `Cargo.toml` that:

- Defines a plain library crate (`crate-type = ["lib"]`). The CLI generates
  a separate FFI companion crate that handles `staticlib`/`cdylib` exports,
  so your user crate stays a normal Rust library.
- Depends on `waterui` with the `assets`, `media`, `webview`, and
  `flow-markdown` features enabled on native targets.
- Uses Rust edition 2024.

A minimal generated `Cargo.toml` looks like:

```toml
[package]
name = "counter"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["lib"]

[dependencies]
waterui = { version = "0.2", default-features = false }

[target."cfg(not(target_arch = \"wasm32\"))".dependencies]
waterui = { version = "0.2", default-features = false, features = ["assets", "media", "webview", "flow-markdown"] }

[features]
dev = ["waterui/dynamic_linking"]
```

### Font management

Custom fonts are declared in `Cargo.toml` metadata so the build system can
bundle them into native projects:

```toml
[[package.metadata.waterui.assets.font]]
name = "Inter"
local_path = "assets/raw/Inter-Variable.ttf"

[[package.metadata.waterui.assets.font]]
name = "JetBrainsMono"
local_path = "assets/raw/JetBrainsMono-Regular.ttf"
```

Each entry declares a font family name and either a `local_path` (relative
to the crate root) or a `remote_path` URL the CLI downloads on demand. The
Water CLI reads this metadata during packaging and copies the font files
into the appropriate locations for each native backend. Built-in font names
such as `Inter`, `Roboto`, `JetBrainsMono`, `FiraCode`, and `SourceCodePro`
resolve from the registry automatically when neither path is provided.

> **Tip:** Place local font files in `assets/raw/` and declare them here.
> WaterUI handles bundling them into every platform's app package
> automatically -- no need to configure Xcode or Gradle font resources
> manually.

## Asset Directory Layout

WaterUI enforces a **strict asset layout** to ensure cross-platform
compatibility. All assets live under the directory specified by
`package.assets_path` (default: `assets/`).

```text
assets/
  raw/             # Arbitrary files: JSON, fonts, data files, etc.
    data.json
    Inter-Variable.ttf
  images/          # Image resources
    logo.png
    icon@2x.png
```

### `assets/raw/`

Files placed here are bundled as-is into the application package. Use this for:

- Custom fonts (`.ttf`, `.otf`)
- Data files (`.json`, `.csv`, `.toml`)
- Shaders (`.wgsl`, `.metal`)
- Any other non-image resource

### `assets/images/`

Image files placed here are processed by the asset pipeline. The pipeline
handles:

- Resolution variants (`@2x`, `@3x` suffixes)
- Format conversion as needed per platform

## The Application Entry Point

Every WaterUI application requires three things in `src/lib.rs`. You have seen
all three in the previous chapter, but let's formalise them here.

### 1. The Root View Function

A function that returns `impl View`:

```rust
fn main() -> impl View {
    text("Hello, World!")
}
```

The name `main` is a convention, not a requirement. You can name it anything.

### 2. The App Constructor

A public function named `app` that takes an `Environment` and returns an
`App`:

```rust
pub fn app(env: Environment) -> App {
    App::new(main, env)
}
```

The `App` struct holds the application's windows and environment. The
simplest form creates a single window with a default title. You can
customise:

```rust
pub fn app(env: Environment) -> App {
    App::new(main, env).title("My Counter App")
}
```

For multi-window applications:

```rust
use waterui::window::Window;

pub fn app(env: Environment) -> App {
    App::new_with_windows(
        [
            Window::new("Main", main_view),
            Window::new("Settings", settings_view),
        ],
        env,
    )
}
```

### 3. The FFI Export Macro

```rust
waterui_ffi::export!();
```

This macro generates the C-ABI functions that native backends call to
initialise the runtime, obtain the root view tree, and drive the render loop.
Without this line, the native backend cannot communicate with your Rust code.

> **Warning:** Forgetting `waterui_ffi::export!()` is one of the most common
> mistakes. Your project will compile, but the app will crash at launch because
> the native backend cannot find the FFI entry points.

## Putting It All Together

A complete, well-structured project looks like this:

```text
my-app/
  Cargo.toml
  Water.toml
  src/
    lib.rs             # Entry point: main(), app(), export!()
    views/
      mod.rs           # View module declarations
      home.rs          # Home screen view
      settings.rs      # Settings screen view
  assets/
    raw/
      config.json
    images/
      logo.png
```

```toml
# Water.toml
[package]
type = "playground"
name = "My App"
bundle_identifier = "com.example.myapp"
```

```toml
# Cargo.toml
[package]
name = "my-app"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["staticlib", "cdylib"]

[dependencies]
waterui = "0.2"
waterui-ffi = "0.2"
```

```rust
// src/lib.rs
use waterui::prelude::*;
use waterui::app::App;

mod views;

fn main() -> impl View {
    views::home()
}

pub fn app(env: Environment) -> App {
    App::new(main, env).title("My App")
}

waterui_ffi::export!();
```

## Playground vs Full: When to Switch

Start with **playground mode** for:
- Learning and experimentation
- Prototyping ideas
- Small personal projects
- Following this book's examples

Switch to **full project mode** when you need:
- Custom native build settings
- Platform-specific native code (Swift/Kotlin extensions)
- CI/CD integration with native build tools
- App Store or Play Store submission
- Fine-grained control over backend dependencies

To convert a playground project to a full project, change `type = "playground"`
to `type = "app"` in `Water.toml`, copy the backend directories from `.water/`
to your project root, and add a `[backends]` section. Alternatively, create a
fresh full project and move your Rust code over.

> **Tip:** There is no rush to switch. Many developers stay in playground mode
> well into development and only convert when they are ready to customise
> native settings for release.

## What's Next

With a solid understanding of how WaterUI projects are structured, you are
ready to dive into the framework's core concepts. In
[The View System](../02-core/01-view.md), you will learn how the `View`
trait works, how views compose, and how the framework turns your Rust types
into platform-native UI.

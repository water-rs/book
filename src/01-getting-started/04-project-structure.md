# Project structure and Water.toml

> **In this chapter, you will:**
> - Compare the playground and app project layouts
> - Configure every section of the `Water.toml` manifest
> - Add assets and custom fonts
> - Decide when to switch from playground to app mode

## Playground layout

`water create --mode playground` writes four files and initialises a git
repository if you are not already inside one:

```text
my-app/
  Cargo.toml           # Rust crate manifest
  Water.toml           # WaterUI project manifest
  src/lib.rs           # Your application code
  .gitignore
```

Generated native projects live outside your tree, in the global managed cache:

```text
~/.water/build_cache/<absolute-project-path>/managed_backends/
  apple/               # Swift package
  android/             # Gradle project
  gtk4/                # GTK4 backend crate
  hydrolysis/          # Hydrolysis backend crate
  esp32/               # ESP32 firmware crate
  ffi/                 # FFI companion crate
  preview_ffi/         # Preview companion crate
```

Only the backends a command actually needs are generated. Every `water run`
re-scaffolds the templates, so `Water.toml` changes -- permissions, theme
colours, the web engine -- reach the native projects without a manual step;
build outputs inside the cache survive that regeneration.

Two rules follow from the split: a playground manifest must have **no**
`[backends]` section, and `[permissions]` is available **only** in playground
mode.

> **Tip:** To reclaim disk space across abandoned playgrounds, run
> `water gc build-cache` or `water clean --global-cache --yes`.

## App layout

App mode checks the native projects into your repository, so you can edit
Xcode settings, add Swift or Kotlin sources, and wire them into CI:

```text
my-app/
  Cargo.toml
  Water.toml
  src/lib.rs
  .gitignore
  backends/
    apple/             # Swift package (checked in)
      Package.swift
      Sources/
    android/           # Gradle project (checked in)
      app/
      build.gradle.kts
    ffi/               # FFI companion crate (checked in)
```

Only the backends you passed to `--backends` (or added later with
`water backend add`) appear. `[backends]` in `Water.toml` tracks them, and
permissions move to the native files -- `Info.plist` and
`AndroidManifest.xml`.

## Water.toml

### `[package]`

```toml
[package]
type = "playground"          # or "app"
name = "My Application"
bundle_identifier = "dev.waterui.myapp"
```

| Field | Type | Description |
|-------|------|-------------|
| `type` | `"playground"` or `"app"` | Project mode. |
| `name` | string | Display name shown by the OS. |
| `bundle_identifier` | string | Reverse-domain identifier: iOS bundle ID and Android application ID. |
| `assets_path` | string | Assets directory relative to the project root. Defaults to `"assets"`; omitted from the file at the default. |
| `accessory` | boolean | Build a headless macOS accessory app: no dock icon, no menu bar. Defaults to `false`. |

### `[backends]`

App projects only. Populated by `water create --backends` and
`water backend add`:

```toml
[backends]
path = "backends"            # base directory, relative to the project root

[backends.apple]
[backends.android]
[backends.gtk4]
[backends.hydrolysis]

[backends.esp32]
chip = "esp32c3"
panel_width = 240
panel_height = 240
band_height = 16
```

The ESP32 entry is the single source of truth for the selected chip; the CLI
derives the target triple and QEMU machine from it.

> **Warning:** `[backends]` in a playground manifest, or `[permissions]` in an
> app manifest, makes the CLI reject the project outright. Each mode has one
> configuration path.

### `webview_backend`

Selects the engine behind the `WebView` component:

```toml
webview_backend = "default"   # default | system | wpe | cef
```

- `default` -- bundled WPE on Linux, the system engine elsewhere.
- `system` -- the platform web view (WebKitGTK on Linux).
- `wpe` -- WaterUI's bundled WPE WebKit runtime (Linux).
- `cef` -- WaterUI's bundled Chromium Embedded Framework runtime (macOS,
  Linux, Windows), independent of the rendering backend. The Dew backend
  excludes CEF.

Setting this alone never adds a runtime: the CLI links an engine only if your
app actually depends on `waterui-webview`.

### `waterui_path`

```toml
waterui_path = "../waterui"
```

Points every generated backend at a local WaterUI checkout instead of published
crates. `water create --waterui-path` sets it for you, and a CLI built from a
local checkout sets it automatically.

### `[permissions]`

Playground mode only. Declare a permission once and the CLI writes the matching
`Info.plist` key and `AndroidManifest.xml` entry on the next `water run`:

```toml
[permissions.camera]
enable = true
description = "Scan barcodes on product labels"

[permissions.location]
enable = true
description = "Show stores near you on the map"

[permissions.microphone]
enable = true
description = "Record voice notes"
```

Each entry takes `enable` (boolean) and `description` (the text the system
dialog shows the user; vague wording gets apps rejected from stores).

The permission keys are a closed set: `internet`, `camera`, `microphone`,
`location`, `coarse_location`, `storage`, `write_storage`, `photo_library`,
`contacts`, `calendars`, `bluetooth`, `bluetooth_admin`, `vibrate`, and
`wake_lock`.

### `[theme]`

Optional cross-platform colour slots, used to seed native launch screens and
backend defaults:

```toml
[theme]
background = "#101014"
surface = "#1B1B20"
foreground = "#F2F2F7"
accent = "#3B82F6"
```

The available slots are `background`, `surface`, `surface_variant`, `border`,
`foreground`, `muted_foreground`, `accent`, and `accent_foreground`.

### `[app.crates]`

App mode only. Overrides the generated crate names when the defaults collide
with something in your workspace:

```toml
[app.crates]
ffi = "myapp_ffi"
gtk = "myapp_gtk"
hydrolysis = "myapp_hydrolysis"
```

## Cargo.toml

`water create` generates a plain library crate. The FFI companion owns the
`staticlib`/`cdylib` output, so your crate stays a normal Rust library:

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
waterui = { version = "0.2", default-features = false, features = ["assets", "media", "flow-markdown"] }

[features]
dev = ["waterui/dynamic_linking"]
```

The scaffold sets `default-features = false` and names features explicitly, so
non-wasm targets get `assets`, `media` (which implies `video`), and
`flow-markdown`. The `waterui` crate's own defaults are
`["gpu", "assets", "media", "flow-markdown"]`; add `"gpu"` to the list when you
want GPU-backed drawing, filters, or SVG. Opt-in features include `webview`,
`chart`, `barcode`, `map`, `particle`, and `navigation-restoration`.

### Custom fonts

Fonts are declared in Cargo metadata so the CLI can bundle them for every
backend:

```toml
[[package.metadata.waterui.assets.font]]
name = "Inter"

[[package.metadata.waterui.assets.font]]
name = "MyBrandFont"
local_path = "assets/fonts/MyBrandFont.ttf"

[[package.metadata.waterui.assets.font]]
name = "lucide"
remote_path = "https://github.com/lucide-icons/lucide/releases/download/0.562.0/lucide-font-0.562.0.zip"
required-feature = "webfont"
```

Give a `local_path` (relative to the crate root), a `remote_path` the CLI
downloads on demand, or neither -- names in the built-in registry (`Inter`,
`Roboto`, `JetBrainsMono`, `FiraCode`, `SourceCodePro`, and the Noto Sans CJK
families) resolve automatically. `required-feature` skips the font unless that
feature is enabled on the declaring package, which is how icon-set crates ship
their fonts without forcing them on every consumer.

The CLI scans your dependencies' metadata too, so a font declared by a library
crate is bundled without any change to your manifest.

## Assets

Create the assets directory yourself; `water create` does not. Everything under
`assets/` (or whatever `package.assets_path` names) is discovered recursively
and classified by file extension:

```text
assets/
  Icon.png             # app icon
  logo.png             # ImageAsset
  intro.mp4            # VideoAsset
  theme.ttf            # FontAsset
  config.json          # DataAsset
  model.onnx           # LargeFileAsset (memory-mapped)
  icons/
    settings.png       # nested directories become nested modules
```

Extensions map to types: images (`.png`, `.jpg`, `.webp`, `.avif`, …), video,
audio, fonts, large binaries (`.onnx`, `.safetensors`, `.gguf`, …), and
everything else as data. Directory structure becomes module structure in the
generated asset code, reached through the `asset!` macro.

A file named `Icon.<image ext>` at the top level of the assets root is the
application icon. Declaring two of them, or pointing the name at a non-image,
fails the build rather than silently picking one.

## The application entry point

Every WaterUI crate needs two functions.

**A root view** returning `impl View`:

```rust,ignore
fn main() -> impl View {
    text("Hello, World!")
}
```

The name `main` is a convention; anything works.

**A public `app` constructor**:

```rust,ignore
pub fn app(env: Environment) -> App {
    App::new(main, env)
}
```

`App::new` opens one window immediately. Give it a title, or install
environment values before handing it off:

```rust,ignore
pub fn app(mut env: Environment) -> App {
    env.install(Theme::new().color_scheme(ColorScheme::Dark));
    App::new(main, env).title("My Counter App")
}
```

For several windows, build them yourself. The first window is the main one:

```rust,ignore
use waterui::app::App;
use waterui::prelude::*;
use waterui::window::{Window, WindowState};

fn main_view() -> impl View { text("Main") }
fn settings_view() -> impl View { text("Settings") }

pub fn app(env: Environment) -> App {
    App::new_with_windows(
        [
            Window::new("Main", Binding::container(WindowState::Normal), main_view),
            Window::new("Settings", Binding::container(WindowState::Closed), settings_view),
        ],
        env,
    )
}
```

A window's `WindowState` binding controls whether it is open, so a window
created as `Closed` appears when you flip its state to `Normal`.

There is no third piece. The FFI companion crate is generated and maintained by
the CLI; `waterui_ffi::export!()` does not belong in your source.

## A worked example

```text
my-app/
  Cargo.toml
  Water.toml
  src/
    lib.rs             # main() and app()
    views/
      mod.rs
      home.rs
      settings.rs
  assets/
    Icon.png
    config.json
    images/
      logo.png
```

```toml
# Water.toml
[package]
type = "playground"
name = "My App"
bundle_identifier = "dev.waterui.myapp"
```

```rust,ignore
// src/lib.rs
use waterui::app::App;
use waterui::prelude::*;

mod views;

fn main() -> impl View {
    views::home()
}

pub fn app(env: Environment) -> App {
    App::new(main, env).title("My App")
}
```

## When to switch modes

Stay in **playground mode** for learning, prototypes, and small projects --
anything where native build settings are not the point.

Move to **app mode** when you need custom native build settings, Swift or
Kotlin code of your own, CI that drives Xcode or Gradle directly, store
submission, or per-backend dependency control.

To migrate, create a fresh app project with the backends you want and move
`src/`, your assets, and your manifest settings across. If you want to keep the
generated native projects, copy them from
`~/.water/build_cache/<absolute-project-path>/managed_backends/`, set
`type = "app"`, and add the matching `[backends]` section.

## Next steps

Continue to [The View System](../02-core/01-view.md) to learn how the `View`
trait works, how views compose, and how the framework turns Rust types into
platform-native UI.

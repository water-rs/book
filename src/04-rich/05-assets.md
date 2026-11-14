# Assets and fonts

> **In this chapter, you will:**
> - Turn files under `assets/` into typed handles with `asset!`, `assets!` and `include_bundle!`
> - Load bundled data and memory-mapped files outside a view body through a `ResourceContext`
> - Fetch remote data behind the `remote` feature
> - Declare fonts in `Water.toml`, pre-seed the font cache with `water fetch`, and build with `--offline`
> - Set the app icon and launch screen from `assets/Icon.svg`, `assets/Launch.svg` and `[launch]`

An app is more than Rust: it ships images, a config file, a font, sometimes a
model. Where those files end up differs on every platform: an `.app` bundle on
macOS, `assets/` inside an APK on Android, a directory beside the executable
on desktop. WaterUI keeps that out of your code. You put files in one
directory, refer to them with a macro, and the `water` CLI stages them into
whatever the platform's package looks like. The macros need the `assets`
feature, which is on by default.

## Quick start

Create `assets/` at the project root (`water create` already did) and drop a
file in it. Then name it with `asset!`:

```rust
use waterui::prelude::*;

fn hero() -> impl View {
    vstack((asset!("logo.png"), asset!("intro.mp4")))
}
```

`asset!` looks at the file extension and expands to a typed handle.
`ImageAsset` and `VideoAsset` are views, so they go straight into a stack. The
path is a logical path relative to the packaged asset root, not a path on your
disk, and the macro does not check that the file exists. A typo shows up when
the file is loaded, not when the crate compiles.

> **Feature flags:** `ImageAsset` needs the `media` feature on `waterui` and
> `VideoAsset` needs `video`. Without them the extension still picks the kind,
> but the type does not exist and the expansion fails to compile. See
> [the media chapter](01-media.md) for the views they build.

## What `asset!` returns

| Extensions | Handle | What it gives you |
|---|---|---|
| `png` `jpg` `jpeg` `gif` `webp` `avif` `bmp` `ico` `svg` | `ImageAsset` | a view (renders through `Photo`), `url(&ResourceContext)` |
| `mp4` `mov` `webm` `avi` `mkv` | `VideoAsset` | a view, `raw(..)`, `player(..)`, `url(..)` |
| `mp3` `wav` `ogg` `flac` `aac` `m4a` | `AudioAsset` | `url(..)`, `path(..)` |
| `ttf` `otf` `ttc` `otc` `woff` `woff2` | `FontAsset` | `path(..)`, `logical_path()` |
| `onnx` `safetensors` `gguf` `ggml` `pt` `pth` `bin` | `LargeFileAsset` | async `load(..)` into a `LargeFile`, `path(..)` |
| everything else | `DataAsset` | `load(..)` into `Data`, `path(..)` |

The handles are small `Copy` values holding a `Bundle` and a logical path.
They resolve to a real location only when you hand them a `ResourceContext`,
which is why a handle can live in a `const` or a field and still point at the
right directory in each running app. `AssetKind::from_extension` is the same
classification as a function, if you need it in your own code. It matches the
lowercase spellings only, so `asset!("LOGO.PNG")` is a `DataAsset`.

Two options change what the macro produces:

- `asset!("shader.wgsl", embed = true)` expands to
  `Data::from_static(include_bytes!(..))`, baking the bytes into the binary.
  The path is then relative to the source file, like `include_bytes!`, and
  only data files support `embed`. Combining it with a remote URL is a compile
  error. `Data::from_static` is not a `const fn`, so build the value in a
  function rather than a `const`.
- A path starting with `https://` is a remote asset, covered
  [below](#remote-data). Plain `http://` is rejected at compile time unless the
  host is loopback (`localhost`, `127.0.0.0/8`, `[::1]`).

```rust,ignore
use waterui::prelude::*;

fn shader_source() -> Data {
    asset!("shader.wgsl", embed = true)
}
```

## Typed access with `assets!`

`asset!("images/hero.png")` is a string; renaming the file breaks it at
runtime. `assets!()` turns the whole `assets/` directory into a module of
accessor functions, so a rename breaks the build instead:

```rust,ignore
use waterui::prelude::*;

// Once, in lib.rs. Reads `[package] assets_path` from Water.toml
// (default "assets").
assets!();

fn banner() -> impl View {
    // assets/images/hero.png  ->  assets::images::hero()
    assets::images::hero()
}
```

Directories become nested modules and each file becomes a function named after
its stem, converted to snake case, so `Hero-Image.png` is `hero_image()`. Each
function returns the same typed handle `asset!` would. Two files with the same
stem in one directory (`logo.png` and `logo.svg`) collide, and the macro
reports it. The module also carries `assets::BUNDLE`, the `Bundle` its
handles resolve against.

Because every file gets an accessor, the module needs the features of every
kind it finds. `water create` puts `Icon.svg` in `assets/`, so `assets!()` in a
new project emits `assets::icon()`, an `ImageAsset`, and does not compile until
`media` is enabled on `waterui`. A video file in the directory needs `video` in
the same way.

`assets!()` is optional. The CLI always stages the whole `assets/` directory
(minus `assets/README.md`, which only documents the workflow); the macro adds
the typed module. The macro does not know about that exception, so it still
emits `assets::readme()`, a handle to a file no package contains.

## Extra bundles with `include_bundle!`

`include_bundle!` mounts any other directory as a named bundle. The path is
relative to the crate root and the module name after `as =` is yours:

```rust,ignore
use waterui::prelude::*;

include_bundle!("manuals", as = manuals);

fn manual_asset() -> DataAsset {
    // manuals/guide/intro.json  ->  manuals::guide::intro()
    manuals::guide::intro()
}
```

Files in a named mount live under a subdirectory named after the mount, which
is why `manuals::BUNDLE` is `Bundle::new("manuals")` and not the main bundle.
A named mount can never claim the app icon (below). Declaring the same mount
twice is an error at package time.

A `Bundle` can also be built by hand. `Bundle::new` takes the mount name as a
`&'static str`, while `Bundle::path` and `Bundle::url` take the logical path as
a plain `&str`, so they resolve a file whose name you only know at runtime:

```rust
use std::path::PathBuf;

use waterui::prelude::*;

fn manual_path(resources: &ResourceContext) -> PathBuf {
    Bundle::new("manuals").path(resources, "guide/intro.json")
}
```

`include_web!` is a sibling macro for bundling a web frontend; it needs the
`webview` feature as well and is covered in [the webview chapter](03-webview.md).

### How the CLI finds your mounts

Each of these macros also emits a small record, a static named
`waterui_meta_bundle_<mount>` in a dedicated linker section of your compiled
library. After a build, the CLI reads those records back from the library it
just compiled and stages each directory next to the app. Nothing scans your
source for macro calls, so a mount behind a `cfg` is staged exactly when the
`cfg` compiled it in, and a mount emitted by your own macro is found like any
other. The record is build-time information; the running app never reads it.
You never write one by hand.

## Loading outside a view body

`ImageAsset` and `VideoAsset` find their own `ResourceContext` when the view
builds. Everything else (`Data`, `LargeFile`, a path for an audio engine) needs
the context passed in explicitly.

A `ResourceContext` holds two directories supplied by the host: the asset root
and the font directory. Each app instance owns one, and it travels in the
instance's `Environment`. Nothing in WaterUI reads a process-wide resource
root, so two instances in one process (an embedded host, a test) resolve the
same handle against their own files.

You normally do not create one. The backend installs it before your app
function runs. A view's `body` receives the `Environment`, and
`ResourceContext::from_environment` returns `None` only when the host forgot to
install one:

```rust
use waterui::prelude::*;

fn load_greeting(resources: &ResourceContext) -> Result<String, AssetError> {
    let data = asset!("greeting.txt").load(resources)?;
    data.into_string()
        .map_err(|error| AssetError::io(error.to_string()))
}

struct Greeting;

impl View for Greeting {
    fn body(self, env: &Environment) -> impl View {
        ResourceContext::from_environment(env).map(|resources| {
            match load_greeting(resources) {
                Ok(greeting) => text(greeting),
                Err(error) => {
                    waterui::log::error!("greeting.txt: {error}");
                    text("Greeting unavailable")
                }
            }
        })
    }
}
```

`DataAsset::load` reads the whole file synchronously and returns `Data`, which
dereferences to `[u8]`. That suits a settings file or a small table. For
anything larger, load it from async code as shown next. `Data` also has
`as_str`, `into_string`, `len`, `from_bytes` and `from_static`.

### Large files

`LargeFile` memory-maps its file instead of reading it, which is what you want
for a model or an archive. Setting up the map is async, so `LargeFileAsset::load`
always returns a future. Pair it with [a `Suspense`](../06-advanced/03-suspense.md)
and implement `SuspendedView` yourself when the loader needs the environment:

```rust
use waterui::prelude::*;
use waterui::widget::suspense::SuspendedView;

struct ModelStatus;

impl SuspendedView for ModelStatus {
    async fn body(self, env: Environment) -> impl View {
        let message = match ResourceContext::from_environment(&env) {
            None => String::from("No resource context installed"),
            Some(resources) => match asset!("classifier.onnx").load(resources).await {
                Ok(model) => {
                    model.warm().await;
                    format!("Model ready: {} bytes", model.len())
                }
                Err(error) => error.to_string(),
            },
        };
        text(message)
    }
}

fn model_view() -> impl View {
    suspense(ModelStatus)
}
```

A mapped file pages in lazily, so the first read of an untouched page blocks
the calling thread on a page fault. `warm().await` touches every page on a
background thread first. The operating system may still evict pages under
memory pressure, so run inference on a background thread rather than the UI
thread.

`AssetError` tells you what went wrong: `NotFound`, `Io`, `Mmap`, `Network`,
`InvalidPath` or `HttpNotAllowed`. It implements `Display` and
`std::error::Error`, and it is `#[non_exhaustive]`, so a `match` on it needs a
wildcard arm.

### Creating a context yourself

An embedding host, or a test that renders a view without a backend, builds the
context and installs it before constructing the app:

```rust
use waterui::prelude::*;

fn host_environment() -> Environment {
    let mut env = Environment::new();
    env.insert(ResourceContext::new("/opt/host/assets", "/opt/host/fonts"));
    env
}
```

`ResourceContext::application()` is what a standalone app uses. It returns an
`std::io::Result` and captures the directories once: `WATERUI_ASSETS_ROOT` when
that variable is set, otherwise the first existing `waterui_assets`,
`resources/waterui_assets` beside the executable, or `Resources/waterui_assets`
one level up (a macOS bundle's `Contents/Resources`), falling back to the first
of these when none exists. The font directory is
`fonts` beside the asset root. [The embedding chapter](../06-advanced/13-embedding.md)
covers hosts that supply their own.

> **Platforms:** on `wasm32` only `AssetError`, `AssetKind` and `Data` exist;
> the typed handles, `Bundle` and `ResourceContext` are not compiled. A local
> image there expands to `Photo::from_path(..)`, a video to
> `video(Url::from_file_path_str(..))` and a data file to
> `Data::from_local(..)`. Audio, font and large-file paths, `assets!` and
> `include_bundle!` do not compile for `wasm32`.

## Remote data

`asset!` also accepts an `https://` URL. For a data file or a large file the
expansion is not a handle but a future: a remote data URL becomes
`Data::from_remote(url)`, and a remote large file becomes
`LargeFile::from_remote(url)`. Both exist only with the `remote` feature of
`waterui`. Await the future:

```rust
use waterui::prelude::*;

async fn remote_manifest() -> impl View {
    match asset!("https://raw.githubusercontent.com/water-rs/waterui/dev/Cargo.toml").await {
        Ok(data) => text(format!("Fetched {} bytes", data.len())),
        Err(error) => {
            waterui::log::error!("Cargo.toml: {error}");
            text("Manifest unavailable")
        }
    }
}

fn manifest_size() -> impl View {
    suspense(remote_manifest())
}
```

`Data::from_remote` downloads into memory every time it is awaited.
`LargeFile::from_remote` downloads once into the per-user cache directory
under `waterui/assets`, named by the SHA-256 of the URL, and maps that file on
later calls. Remote audio is a compile error. A remote image or video expands
to the `Photo` or `video` view built from the URL directly, which needs the
`media` or `video` feature; a remote `Photo` also needs `remote` to fetch, and
without it reports an error naming the missing feature. The `remote` feature
links an HTTP client, which is why it is off by default: an app that never
names a URL carries none.

## Fonts

A font is declared, not loaded. You tell the CLI which family the app needs;
it resolves the file, caches it, and stages it into every package; your views
then ask for the family by name:

```toml
# Water.toml
[[assets.font]]
name = "Inter"

[[assets.font]]
name = "Lexend"
local_path = "fonts/Lexend-Regular.ttf"

[[assets.font]]
name = "Noto Sans CJK SC"
platforms = ["android", "web"]
```

```rust
use waterui::prelude::*;
use waterui::text::font::{Body, Font};

fn headline() -> impl View {
    text("WaterUI 水 みず 물").font(Font::new(Body).family("Inter"))
}
```

`Font::new(Body)` keeps the size and weight of the body token and
`.family("Inter")` swaps the face. [The text chapter](../03-ui/01-text.md)
covers the font tokens.

Each `[[assets.font]]` table takes:

| Key | Meaning |
|---|---|
| `name` | the family name views use. Required. Write the family name recorded inside the font file: Apple registers the staged file with Core Text, which knows it by that name, while Android and the web map the declared name to the file |
| `local_path` | a font file relative to the project root; absolute paths are an error |
| `remote_path` | a URL to the face, or to an archive containing it |
| `platforms` | the platforms whose builds bundle it; absent means all. Accepted names are `ios`, `macos`, `tvos`, `watchos`, `visionos`, `android`, `linux`, `windows` and `web`; an unknown name or an empty list is an error |

A declaration has exactly one source. With neither path, the name is looked up
in a built-in registry of pinned downloads: Noto Sans, Inter, Roboto,
Noto Sans CJK JP, KR, SC and TC, Noto Sans Arabic, Noto Sans Hebrew,
Noto Color Emoji, JetBrainsMono, FiraCode, SourceCodePro and STIX Two Math.
Each registry entry names an immutable release and carries a SHA-256 that the
download must match. A name the registry does not know, without a path, fails
the build with a message saying so, instead of rendering in whatever face the
shaper falls back to.

A Hydrolysis web build has no system fonts to fall back on, so it fails unless
at least one font is declared for `web`. Its default family is `Roboto` when
you declare it, otherwise the declared family that sorts first alphabetically.

Library crates declare fonts the same way in their `Cargo.toml`, which is how
icon-set crates ship their glyph fonts. This is the declaration in
`waterui-icons-material-icon`:

```toml
[[package.metadata.waterui.assets.font]]
name = "Material Design Icons"
remote_path = "https://cdn.jsdelivr.net/npm/@mdi/font@7.4.47/fonts/materialdesignicons-webfont.ttf"
required-feature = "webfont"
```

`required-feature` makes the declaration count only when that feature is
enabled on the declaring crate. A dependency's `local_path` is relative to that
crate's own root. The CLI walks your app's dependency graph for
these tables, so adding a crate that needs a font needs no change in your
`Water.toml`. When several declarations name the same family, a local file
beats a remote URL, a remote URL beats the registry, and your `Water.toml`
wins a tie against a dependency.

### The font cache and `water fetch`

Remote and registry fonts are downloaded once into a per-user cache (the
`waterui/fonts` directory under the platform cache location) and reused by
every project. A normal `water build`, `run` or `package` fetches whatever the
cache lacks. `water fetch` does that step on its own:

```bash
water fetch                        # every backend this host can build
water fetch --backend hydrolysis   # only the crates one backend's build scans
water fetch --path ./my-app
```

It reports each family as already cached, fetched, or unsatisfiable, such as a
missing `local_path` file or an unknown registry name, and exits with an error
if any are unsatisfiable.

Use it to warm a CI image or a laptop that is about to go offline, then build
with `--offline`:

```bash
water fetch
water package --offline
```

`--offline` is the same switch as `cargo build --offline`, and
`CARGO_NET_OFFLINE` is honored the same way. It exists on `build`, `run`,
`package`, `preview`, `mcp` and `inspector`. With it, a declared font missing from the cache
is a build error naming the font, its URL and `water fetch`, never a silent
download. See [the CLI chapter](../01-getting-started/01-cli.md) for the rest
of the command surface.

> **`FontAsset` is not how you register a font.** `asset!("body.ttf")` gives
> you a handle to a file under `assets/` that the CLI copies like any other
> asset. The family your views can name comes from the declarations above.

## The app icon

Put one square image named `Icon` at the root of `assets/`, and the CLI
generates every platform's format from it: iOS full-bleed, macOS rounded
rectangle, Android adaptive layers, Windows `.ico`, and a freedesktop hicolor
icon set on Linux. `Icon.svg`
is the usual choice; a square raster (`Icon.png`) works too. Exactly one
root-level `Icon.*` is allowed, and it must be an image. A new project starts
with the WaterUI logo, so replacing that single file rebrands the app
everywhere. A file named `Icon.png` in a subdirectory is a regular asset, not a
second claim. See [the icons chapter](../03-ui/12-icons.md) if you are looking
for in-app icons rather than the launcher icon.

## The launch screen

The launch screen is what the platform shows between the tap on the icon and
the app's first frame. No Rust runs while it is visible, so it cannot be a
view. It is declared once, in `Water.toml`, and the CLI projects it onto each
platform's own mechanism:

```toml
[launch]
background = "#0B1E3F"        # defaults to [theme] background
background_dark = "#000000"   # defaults to background
```

The artwork is one optional square `Launch.svg` (or `Launch.png`) at the root
of `assets/`, subject to the same one-file rule as `Icon.*`. Colors are
`#RRGGBB`; a malformed one fails when the manifest is read. With neither
`[launch]` nor a `[theme]` background, each platform keeps its own default
background: the system background on iOS, the window background on Android,
the page default on the web. The artwork also follows each platform rather
than one shared look:

| Platform | Without `Launch.*` | With `Launch.*` |
|---|---|---|
| iOS | no artwork | centered, inside the safe area |
| Android | the OS default, the launcher icon | shown as the splash icon, which Android masks to a circle |
| Web (Hydrolysis) | the app icon above a download progress bar | replaces the icon |
| macOS, GTK, desktop Hydrolysis | nothing | ignored |

Desktop platforms have no launch screen. The window appears with the first
frame already drawn, so `[launch]` has no effect there.

## What staging does to your files

When the CLI copies `assets/` and every `include_bundle!` mount into a package,
it decodes each PNG and re-encodes it as 8-bit RGBA with the best compression
setting, re-encodes each JPEG at quality 85, and copies every other file
unchanged. A 16-bit PNG therefore loses its extra precision, and a JPEG goes
through one more lossy pass than the file you committed. [The packaging chapter](../07-tools/07-packaging.md)
covers where each platform puts the result, and
[the project structure chapter](../01-getting-started/04-project-structure.md)
shows where `assets/` sits in a new project.

Continue with [the media chapter](01-media.md), which shows the images and video you ship as assets.

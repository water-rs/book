# Project structure and Water.toml

> **In this chapter, you will:**
> - Read the one project layout `water create` produces and know which files are yours
> - Write the `app` entry point, with state the whole application shares
> - Configure `Water.toml`, section by section
> - Choose facade features in `Cargo.toml`, including the `dev` feature
> - Know where generated native projects live and why you never edit them

Every WaterUI project has the same shape. WaterUI owns the program entry, and
the `water` CLI generates and manages every backend project (Xcode, Gradle, and
the rest) outside your tree. There is nothing to choose when you create a
project, and nothing native to check in. What you keep is a small Rust crate
and one manifest.

## The layout

```text
my-app/
  Water.toml          # WaterUI project manifest
  Cargo.toml          # Rust crate manifest
  .cargo/config.toml  # Apple deployment-target variables the CLI keeps in sync
  .gitignore
  .mcp.json           # lets MCP clients find `water mcp` (see the MCP chapter)
  assets/
    README.md         # keeps the directory in git until you add assets
    Icon.svg          # the app icon: one root-level Icon.* for every platform
    Launch.svg        # optional launch artwork: one root-level Launch.*
  i18n/               # optional translation catalogs, one TOML per locale
  src/
    lib.rs            # your application
```

`water create "My App"` writes everything above except the launch artwork and
`i18n/` (see [Create a project](01-cli.md) and [your first
app](03-first-app.md)). A project on a release channel also gets a
`Cargo.lock` and a `Water.lock` pinned to that channel. The CLI writes
`.mcp.json` only when none exists, so the file is yours once it is there. In
`.cargo/config.toml` the CLI owns only the `MACOSX_DEPLOYMENT_TARGET` and
`IPHONEOS_DEPLOYMENT_TARGET` entries and keeps the rest of your
configuration.

Generated native projects are not in this tree. The CLI keeps them in its
build cache under `~/.water/build_cache`, writes them from its templates when
a command needs them, and builds only the backends that command uses. That is
why a change to `Water.toml` (a permission, a theme color, a backend) reaches
the native project on the next `water run` with no manual step. `water clean`
clears this project's generated backends, and `water gc build-cache` removes
the cache entries of projects you abandoned; see [the CLI chapter](01-cli.md).

> **Note:** Projects from the removed app mode carried `type = "app"` or
> `type = "playground"` under `[package]`, and a `[backends]` table. The CLI
> refuses such a manifest and names the keys to delete; it never reinterprets
> them. `[app.crates] ffi` is an error that points you at
> `[app.crates] apple`. The web view engine is not a manifest key either: it
> follows your dependencies. Linking `waterui-browser-cef` or
> `waterui-browser-wpe` selects that bundled engine, and linking neither uses
> the engine the platform provides.

## The entry point

`src/lib.rs` exposes one function, `app`, that receives the root
`Environment` and returns an `App`:

```rust
use waterui::app::App;
use waterui::prelude::*;

fn root() -> impl View {
    text("Hello, WaterUI")
}

pub fn app(env: Environment) -> App {
    App::new(root, env)
}
```

`App::new` takes anything that builds a view (`impl ViewBuilder`) and the
environment, and opens a single window showing it. Take `mut env` when the app
installs something app-wide, such as a theme. For several windows, see
[Windows](../03-ui/08-windows.md).

State that the whole application shares, such as a value that a menu
command changes too, is built in `app`. Clone it into the view builder, and
install it with `App::state` so any handler, in a view or in the menu bar, can
take it as a `State<T>` parameter:

```rust
use waterui::app::App;
use waterui::prelude::*;

fn counter(count: Binding<i32>) -> impl View {
    vstack((
        text!("Count: {count}"),
        button("Increment").action(|State(count): State<Binding<i32>>| *count.get_mut() += 1),
    ))
}

pub fn app(env: Environment) -> App {
    let count = Binding::i32(0);
    App::new(
        {
            let count = count.clone();
            move || counter(count.clone())
        },
        env,
    )
    .state(&count)
    .menu_bar(Menu::new(
        "Counter",
        "Reset".action(|State(count): State<Binding<i32>>| count.set(0)),
    ))
}
```

A view builder is a `Fn`, which may be called more than once, so the closure
clones the binding instead of moving it out. The content of every window
renders under the application's environment, which is why the button finds
the value that `App::state` installed. `text!("Count: {count}")` reads the
binding reactively, so only that text updates when the count changes.
[The reactive chapter](../02-core/02-reactive.md) covers `Binding` and
`Computed`, [Menus](../03-ui/09-menus.md) covers the menu bar, and [the
environment chapter](../02-core/03-environment.md) covers what else belongs in
`env`.

## Cargo.toml

The generated manifest is an ordinary library crate that depends on the
`waterui` facade with `default-features = false` and no features, because the
starter code needs none. It declares two features of its own:

```toml
[features]
dev = ["waterui/dynamic_linking"]
media = ["waterui/media"]
```

Keep `dev`. The CLI turns it on for development builds (`water run`,
`water build`, previews), which link your app against one shared copy of the
framework (`waterui-dylib`). Packaging builds link statically and leave it
off, and Windows builds always link statically. It is a build strategy, not a
capability. Without it the CLI cannot build the project: the backend crate it
generates forwards `dev` to your crate, so Cargo rejects the dependency graph
before anything compiles, and `water preview` stops with a message naming the
missing feature.

`media` is off until you turn it on. The comment the CLI writes above it in
the generated file explains how: pass `--features media` to `cargo build`, or
add `media` to your crate's `default` features.

Capabilities are facade features, named in the `features` list of the
`waterui` dependency. Because the generated dependency turns default features
off, a new project has none of the following until you name them. A crate that
depends on `waterui` with default features enabled gets this set:

| Default feature | What it brings |
|---|---|
| `gpu` | the GPU rendering stack (also enables `effects`, filters on views) |
| `assets` | the asset pipeline |
| `dialog`, `snackbar` | dialogs and snackbars |
| `highlight` | the `Code` widget and its syntax highlighter |
| `locale-datetime`, `locale-list` | the `waterui::locale` formatters |

Heavier capabilities are never on by default, so an app that does not use
them carries none of their code:

| Feature | What it brings |
|---|---|
| `media`, `video`, `video-gpu` | media components, video, WaterUI's own GPU video player |
| `webview` | the `WebView` component |
| `flow-markdown`, `markdown-math` | Markdown rendering, and math formulas inside it |
| `canvas`, `chart`, `barcode`, `particle` | self-drawn graphics components |
| `widgets` | the `WidgetKit` authoring surface (`#[waterui::widget]`) |
| `ts`, `ts-ota` | TypeScript views and their over-the-air update client |
| `inspector` | the devtools inspector hub |
| `remote` | remote fetching (`Url::fetch` and friends); links an HTTP client |
| `navigation-restoration` | restoring navigation state |

A feature that gates an API is named in the chapter that documents the API,
for example [Media](../04-rich/01-media.md), [WebView](../04-rich/03-webview.md),
[Charts](../04-rich/06-charts.md) and [Canvas](../05-graphics/01-canvas.md).
`all` turns on the capability set CI builds with. It leaves out
`dynamic_linking`, which is a build strategy, along with `ts`, `ts-ota`,
`markdown-math` and `widgets`.

## Water.toml

`Water.toml` is read by the CLI, never by your Rust code. Only `[package]` is
required. Every other table is optional, and `water create` writes only what
it needs.

### [package]

```toml
[package]
name = "My App"
bundle_identifier = "dev.example.myapp"
# assets_path = "assets"    # default
# accessory = false         # macOS: build as a headless accessory app
# embedded = false          # the crate is embedded in a host application
```

`name` is the human-readable app name and `bundle_identifier` the
reverse-DNS identifier. `assets_path` moves the assets directory. With
`embedded = true` the crate is a library for a host application's build:
`water build` produces the artifact the host consumes (an AAR on Android),
and `water run` and `water package` refuse. See
[Embedding](../06-advanced/13-embedding.md).

### [platforms.&lt;platform&gt;] and [hydrolysis]

A project can pin the backend it uses on a platform:

```toml
[platforms.linux]
backend = "hydrolysis"
```

Platform names are `ios`, `ios-simulator`, `macos`, `android`, `linux`,
`windows` and `web`. Backend names are `apple`, `gtk4`, `hydrolysis` and
`winui`. A backend the platform cannot use is a manifest error. The CLI uses
the declaration for `build`, `run`, `package` and previews; an explicit
`--backend` flag may repeat it, but a different value is an error before
building. `water create` writes no declarations, and without one each platform
uses its default:

| Platform | Default backend | Also possible |
|---|---|---|
| macOS | `apple` | `hydrolysis` |
| iOS, iOS simulator | `apple` | none |
| Android | `hydrolysis` | none |
| Linux | `gtk4` | `hydrolysis` |
| Windows | `hydrolysis` | `winui` |
| Web | `hydrolysis` | none |

Apple and Hydrolysis are the mainline backends. GTK4 and WinUI are
experimental. See [Platforms](05-platforms.md) and
[Backends](../08-internals/04-backends.md).

`[hydrolysis]` carries selections for the Hydrolysis backend. Today that is
the Android painter:

```toml
[hydrolysis]
painter = "gpu"    # or "hwui"
```

Absent means `gpu`. `hwui` draws through the host view hierarchy with
`RenderNode` and raises the generated app's minimum to Android API 34.
`water run --painter` overrides the table for one Android run. A painter the
pinned host does not provide is an error when the CLI generates the Android
project, never a silent substitute.

### waterui_path and [framework]

```toml
waterui_path = "../.."
```

`waterui_path` points every backend at a local WaterUI checkout instead of the
published versions. It is a top-level key, so it sits before the first table.
The CLI copies the checkout's `[patch]` entries into your `Cargo.toml` and
records the ones it wrote under `waterui_patches`. Leave that key to the CLI.
Your own `[patch]` entries survive the next copy.

`[framework]` records the exact framework and backend selection of a release
channel. You do not write it: `water create` and `water channel` resolve a
channel and persist the result, so builds do not re-resolve it. Treat it as
generated. A local checkout is never recorded there, because `waterui_path` is
its record, and selecting a channel with `water channel` removes
`waterui_path`.

### [permissions]

```toml
[permissions.internet]
enable = true
description = "Required to download map tiles"

[permissions.location]
enable = true
description = "Required to show the user's position on the map"
```

Keys are `internet`, `camera`, `microphone`, `location`, `coarse_location`,
`storage`, `write_storage`, `photo_library`, `contacts`, `calendars`,
`bluetooth`, `bluetooth_admin`, `vibrate`, `wake_lock`,
`activity_recognition` and `nearby_wifi_devices`. The CLI turns each into the
platform's declaration (Android manifest entries, Info.plist usage strings),
and `description` is the text the user reads in the system prompt, so write it
for them. Declaring a permission is only half the job: a permission the user
grants at runtime, such as camera, microphone or location, is also requested
in code with the async `request` function of `waterkit-permission`.
[Maps](../04-rich/02-maps.md) shows it for location.

### [theme] and [launch]

Some colors are needed before any Rust runs. `[theme]` carries them as
`#RRGGBB` strings, and the CLI writes them into the native project it
generates:

```toml
[theme]
background = "#0B0B0F"
surface = "#15151C"
surface_variant = "#1E1E28"
border = "#2A2A36"
foreground = "#F5F5F7"
muted_foreground = "#A0A0AE"
accent = "#4A84F6"
accent_foreground = "#FFFFFF"

[launch]
background = "#0B0B0F"      # defaults to theme.background
background_dark = "#000000" # defaults to background
```

`accent` becomes the Apple `AccentColor` asset, `background`, `accent` and
`foreground` are bound into the generated Android theme, and `background` is
the default launch-screen color. These slots do not change the theme tokens
inside the running app: that is the `Theme` you install in code, covered in
[Styling](../03-ui/11-styling.md). Keep the two in agreement.

`[launch]` styles what the platform shows from the tap on the icon until the
first frame. No Rust runs yet, so it is declared here instead of built as a
view. The artwork is the optional root-level `Launch.*` image in the assets
directory. A malformed color fails when the manifest is read.

### [apple]

```toml
[apple.deployment-targets]
ios = "27.0"
macos = "27.0"
```

Apple builds target the framework's deployment floor unless you raise it, with
any of `macos`, `ios`, `tvos`, `watchos` and `visionos`. A value below the
floor is an error, and an API that needs a newer OS fails to compile below it,
with an error that names this table. `water create --template web` writes
`27.0` for every Apple platform, the version `include_web!` needs.

### [web]

```toml
[web]
package_manager = "bun"    # bun (default), pnpm, npm or yarn
```

`[web]` names the JavaScript package manager the CLI runs for a bundled web
frontend (`include_web!`): install, build and dev. Without the table the CLI
uses `bun`. `water create --template web` writes it, adds the `web/` Vite
frontend, and enables the `assets` and `webview` features. See
[WebView](../04-rich/03-webview.md).

### [app.crates]

```toml
[app.crates]
apple = "my-app-companion"
```

`[app.crates]` overrides the crate name of a generated backend crate: `apple`
(the Apple companion crate), `gtk`, `hydrolysis` and `winui`. The old `ffi`
key was removed; use `apple`.

### [assets]

```toml
[[assets.font]]
name = "Inter"
```

`[[assets.font]]` bundles a font family beyond what dependency crates declare
for themselves. `name` alone resolves through the CLI's built-in font
registry. At most one source may be added: a `local_path` relative to the
project root, or a `remote_path` URL that `water fetch` downloads into the font
cache ahead of time, since builds never touch the network. An optional
`platforms` list limits which builds bundle it. Fonts, the app icon and
everything else under `assets/` are covered in
[Assets](../04-rich/05-assets.md).

### [signing]

```toml
[signing.macos]
team_id = "ABCDE12345"
notary_profile = "my-notary-profile"

[signing.android]
keystore = "release.keystore"
key_alias = "release"
```

Release packaging reads these; development builds keep each platform's own
signing. The file carries no secrets, and passwords come from the environment
at package time. See [Packaging](../07-tools/07-packaging.md).

### [app_values]

A capability crate that needs configuration only your app knows asks for it,
and packaging fails, naming the key and the crate that asked, until you supply
it. The CLI puts each value where the platform reads it: the Firebase file as
the Android module's `google-services.json`, the merchant IDs as the Apple Pay
entitlement, and the Cast receiver ID as an Android string resource.

```toml
[app_values]
firebase_config = "google-services.json"
apple_pay_merchant_ids = ["merchant.com.example"]
cast_receiver_app_id = "CC1AD845"
```

## Where to go next

Run the project on a device with [Platforms](05-platforms.md), put images and
fonts in place with [Assets](../04-rich/05-assets.md), or localize the app with
the `i18n/` directory in [Internationalization](../06-advanced/06-i18n.md).

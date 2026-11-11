# Previews

> **In this chapter, you will:**
>
> - Mark a view function with `#[preview]` and render it to a PNG with `water preview`
> - Pick the platform and backend a preview renders through
> - Add the `dev` feature that preview builds link against
> - Capture interaction timelines and assert on a preview with `water preview test`
> - Know what a repeated preview rebuilds and what it reuses

Spinning up a device, navigating five screens deep and waiting for a full app build is too much friction for a two-pixel adjustment. A preview shortcuts that loop: annotate a function, run one command, read a PNG.

`water preview` is a one-shot command. There is no file watcher and no patching of a running view tree: you save, you run it again, and the build cache decides how much work that costs. On a desktop host each run builds a small generated preview binary that links your crate, mounts your view in that binary's own process, and writes the PNG. On Android the same kind of build is shipped to a preview host on the device, which renders it there (see [Where each platform renders](#where-each-platform-renders)).

## The `#[preview]` attribute

Mark a free function returning `impl View` with `#[preview]`. The macro is part of `waterui::prelude`.

```rust,ignore
use waterui::prelude::*;

#[preview]
fn sidebar() -> impl View {
    vstack((
        text("Sidebar"),
        text("Content"),
    ))
}
```

The macro leaves your function in place and adds an exported companion that builds the view, wraps it in `AnyView` and hands back a boxed pointer. `#[preview]` applies to free functions only; a method with `self` is a compile error.

### Default arguments

A preview has nothing to invent argument values from, so every parameter takes a default through the attribute:

```rust,ignore
use waterui::prelude::*;

#[preview(count = 5, name = "John Appleseed")]
fn user_card(count: i32, name: &str) -> impl View {
    vstack((
        text!("Name: {name}"),
        text!("Count: {count}"),
    ))
}
```

A missing default, an unknown parameter name and a duplicated one are all compile errors pinned to the offending identifier. Parameters must be plain identifiers.

> **Tip:** Pick defaults that look like real data. A card previewed with an empty name tells you nothing about wrapping or typography. When the data includes Chinese, include Japanese and Korean too, so the preview covers CJK shaping.

### The exported symbol

Each preview function exports one symbol:

```text
waterui_preview_{crate_name}_{function_name}
```

`crate_name` is `CARGO_PKG_NAME` with dashes turned into underscores. `function_name` is the bare function name: the macro never sees the module path, so the module contributes nothing.

| Crate name     | Function                 | Export symbol                       |
|----------------|--------------------------|-------------------------------------|
| `my_app`       | `sidebar`                | `waterui_preview_my_app_sidebar`    |
| `together-app` | `dashboard::admin::card` | `waterui_preview_together_app_card` |

Preview function names must therefore be unique within a crate: two previews with the same bare name export the same unmangled symbol, which a debug build rejects as defined twice. On the command line you may write the module path (`dashboard::admin::card`), but only the last segment is used to find the symbol.

The export exists in **debug builds only** (it is gated on `debug_assertions`). A release build carries neither the entry point nor the view it would force into the binary, so previews cost a shipped app nothing. The CLI's preview builds always keep it: the Hydrolysis development profile, which optimizes and turns debug assertions off, turns them back on for your crate alone.

### Previews are found in the compiled artifact

The CLI never reads your `.rs` files to find previews. A named target becomes a symbol name, and the generated binary links against that symbol; `water preview test --all` opens the compiled library and lists its `waterui_preview_*` exports. Both see only what the compiler produced, so a preview behind a `cfg` or emitted by a macro exists exactly when it is compiled, and a function that is `cfg`'d out of the build is not previewable.

## Project requirements

Your crate declares a `dev` feature that turns on dynamic linking of the WaterUI runtime:

```toml
# Cargo.toml
[features]
dev = ["waterui/dynamic_linking"]
```

Projects from `water create` already carry it. The CLI turns this feature on for its dynamically linked development builds, previews included; Windows and Android builds link the runtime statically and leave it off. The Apple preview path (`--platform macos` and `--platform ios` with the Apple backend) also checks it before building and stops with the exact line to add if `dev` is missing or does not include `waterui/dynamic_linking`. The generated binary links the shared WaterUI runtime dylib instead of embedding a second copy, which is also what keeps rebuilds short (see below).

The generated binary also needs your crate's `app` function, the one `water create` scaffolds as `pub fn app(env: Environment) -> App`. A preview resolves under the `Environment` that `app(env)` returns, so themes, fonts and realizations your app installs there apply to previews too.

Monorepo examples set `waterui_path = "../.."` in `Water.toml` so previews build against the local checkout. See [Project structure](../01-getting-started/04-project-structure.md).

## The `water preview` command

```bash
water preview sidebar --platform macos --path ./my-app --output preview.png
```

| Argument / flag       | Description                                                          | Default                    |
|-----------------------|----------------------------------------------------------------------|----------------------------|
| `target`              | `#[preview]` function name, or an expression with `--expr`           | required                   |
| `--expr`              | Treat the target as a Rust expression returning `impl View`          | off                        |
| `--platform`, `-p`    | `ios`, `macos`, `android`, `linux` or `windows`                      | the host's own platform    |
| `--backend`           | `apple` or `hydrolysis`                                              | from `Water.toml`/platform |
| `--theme`             | Hydrolysis theme package; `material3` is the only one                | `material3` on Hydrolysis  |
| `--frame`, `-f`       | Logical size as `WIDTHxHEIGHT`                                       | `375x667`                  |
| `--output`, `-o`      | Output PNG path                                                      | `preview.png`              |
| `--content-timeout`   | Seconds a capture waits for content views still owe                  | `10`                       |
| `--scenario`          | Hydrolysis scenario TOML for an interaction timeline                 | none                       |
| `--output-dir`        | Directory for scenario frames; required with `--scenario`            | none                       |
| `--device`, `-d`      | Android serial or AVD name; only with `--platform android`           | remembered or picked       |
| `--offline`           | Build like `cargo build --offline`; a declared font missing from the font cache fails instead of being fetched | off |
| `--path`              | Project directory                                                    | `.`                        |

A target that is not a plain Rust path is treated as an expression even without `--expr`.

### Where each platform renders

No preview boots an iOS simulator. Each platform renders inside a generated binary on this machine, or inside a preview host on an Android device:

| Platform  | Backend      | How the PNG is produced                                                                  |
|-----------|--------------|------------------------------------------------------------------------------------------|
| `macos`   | `apple`      | Generated preview binary; the AppKit realization                                         |
| `ios`     | `apple`      | The same binary built for Mac Catalyst; the UIKit realization on this Mac, iPad idiom    |
| `macos`   | `hydrolysis` | Generated Hydrolysis binary                                                              |
| `linux`   | `hydrolysis` | Generated Hydrolysis binary                                                              |
| `windows` | `hydrolysis` | Generated Hydrolysis binary                                                              |
| `android` | `hydrolysis` | Hydrolysis preview host's instrumentation on a connected device, driven over `adb`       |

The backend comes from `--backend`, else from `[platforms.<platform>] backend` in `Water.toml`, else the platform's default: `apple` on macOS and iOS, `hydrolysis` on Android and Windows, `gtk4` on Linux. An explicit `--backend` that contradicts the declaration fails before any build, and a combination outside the table above is rejected by name. GTK has no preview renderer, so a Linux project that declares nothing needs `--backend hydrolysis` or a `hydrolysis` declaration, and one that declares `gtk4` must change the declaration, since the flag would conflict with it. Desktop platforms must match the host: you cannot preview `--platform windows` from a Mac.

`--platform ios` builds for the `aarch64-apple-ios-macabi` target and runs on this Mac under the iPad idiom, so you see the iOS look. Its capture replays the layer tree offscreen rather than through the window compositor, so backdrop filters are not evaluated: a material background shows its tint but not the blur behind it. Check materials on a device with `water run --platform ios`; see [Platforms](../01-getting-started/05-platforms.md).

On Android the preview runs on a real device or emulator, so you get device fonts, the device GPU and the device's own realizations. `--device` takes a serial or AVD name; without it the last device you used wins while it is still connected, then a single candidate is taken, and several candidates bring up a picker (a non-interactive terminal fails instead, naming them). `--device` with any other platform is an error.

The Android preview host is the one exception to a fresh process per run. It stays alive on the device between runs and serves each run as a new render request. The CLI pushes only the payload parts whose content changed, and a changed payload restarts the host, since a loaded library cannot be swapped.

```bash
# AppKit, through the Apple backend
water preview my_view --platform macos

# UIKit through Mac Catalyst, phone-sized frame
water preview profile_card --platform ios --frame 390x844

# The self-drawn renderer on this host
water preview my_view --backend hydrolysis --theme material3

# An Android device over adb
water preview home_screen --platform android --device emulator-5554

# An inline expression instead of a function
water preview 'vstack((text("A"), text("B")))' --expr
```

`--expr` compiles the expression into generated preview code with `waterui::prelude::*` and your crate's root items (`use your_crate::*`) in scope, so it can call your public view functions. It is a real compile, not an interpreter, so a type error is a compiler error.

`--theme` is rejected with the Apple backend; it selects a Hydrolysis style package ([Styling](../03-ui/11-styling.md)).

### Waiting for owed content

A capture is taken after the view tree mounts. Content a view still owes at that point, such as a decoding image or a `Suspense` future ([Suspense](../06-advanced/03-suspense.md)), is waited for up to `--content-timeout` seconds. If it never lands, the preview fails naming what is still pending instead of writing a half-loaded frame. Raise the bound for a slow fetch; a positive finite number of seconds is required.

### Interaction scenarios

On the Hydrolysis backend a scenario captures a timeline instead of a single frame. It is a TOML file of capture times and input events:

```toml
captures_ms = [0, 120, 400]

[[events]]
at_ms = 50
kind = "pointer_down"
x = 100.0
y = 240.0

[[events]]
at_ms = 90
kind = "pointer_up"
x = 100.0
y = 240.0
```

```bash
water preview checkout --backend hydrolysis --scenario press.toml --output-dir frames/
```

Capture times must be non-empty and unique. Event kinds are `pointer_move` (alias `hover`), `pointer_down`, `pointer_up`, `pointer_cancel` and `scroll` (alias `wheel`). Every event except `pointer_cancel` needs `x` and `y`; `scroll` also needs a non-zero `dx` or `dy` and may set `is_line_delta`; `button` is `primary` (the default), `secondary` or `middle`. One PNG per capture time lands in `--output-dir`, named like `frame-0120ms.png`, so you see a ripple mid-flight rather than only its resting state. `--scenario` and `--output-dir` are accepted only with the Hydrolysis backend, and each requires the other.

## Asserting on a preview

`water preview test` reuses the same target resolution, builds the view without a render target and runs a Rust automation body against its accessibility tree:

```bash
water preview test sidebar \
    --code 'app.query().role(Role::BUTTON).label("Save").assert_exists();'
```

The body receives `app: &mut waterui_testing::SemanticApp`, with `waterui_testing::*` and the WaterUI prelude in scope. Use `--code-file` for anything longer than a line; `--code` and `--code-file` are mutually exclusive. `--all` tests every `#[preview]` function in the crate and cannot be combined with a target or `--expr`.

`water preview test` runs on desktop Hydrolysis only: it forces that backend, so a `Water.toml` declaring Apple for the platform conflicts, and `--platform android` is refused. `--theme` defaults to `material3`. A failing assertion usually means an accessibility defect rather than a harness problem, since the tree under test is the accessibility tree. For the full test API see [Testing](03-testing.md).

`--all` finds its targets with a probe build, then reads the `waterui_preview_{crate}_*` exports from the compiled library. The probe compiles the same crate graph the per-target builds then reuse, so discovery costs one warm build.

Frame-time and counter budgets belong to `water bench`, not preview; see [Benchmarks](04-benchmarks.md).

## What a repeated preview rebuilds

Previewing again after an edit costs one incremental cargo build of the generated binary, then a render.

- **Your crate rebuilds; the runtime does not.** With `dev = ["waterui/dynamic_linking"]` the WaterUI runtime is a shared dylib on macOS and Linux; Windows and the Android launcher link it statically. The Apple preview binary is built under the same development configuration `water run` uses for the Apple backend, so it reuses the runtime unit `water run` already compiled rather than relinking a variant on every switch.
- **A repeated target is free to the build.** The generated target module is rewritten only when its content changes, so previewing the same function twice leaves the binary's own compilation unit alone. Switching to another `#[preview]` function changes that module and rebuilds the generated crate that holds it, but not your crate, which it depends on.
- **Each function is its own entry.** Mark as many functions as you like; each exports its own symbol, and one build of your crate carries all of them:

```rust,ignore
use waterui::prelude::*;

#[preview]
fn sidebar() -> impl View { text("Sidebar") }

#[preview]
fn header() -> impl View { text("Header") }

#[preview(count = 3)]
fn notification_list(count: usize) -> impl View {
    text!("{count} notifications")
}
```

```bash
water preview sidebar
water preview header
water preview notification_list
```

- **State never carries over.** Every run constructs a new view, on desktop in a new process. Nothing from the previous run, no `Binding`, `Computed` or `Environment`, survives, so set up the data shape you want to inspect inside the preview function or through `#[preview(...)]` defaults.

### Build caching with sccache

When `sccache` is installed, the CLI routes preview builds through it. If it cannot find the binary, each run prints a warning with the install command for your host; on macOS:

```text
sccache not found. Build efficiency may be reduced. Install with: brew install sccache
```

The CLI skips sccache, and says so, when `RUSTC_WRAPPER` is already set or `WATERUI_DISABLE_SCCACHE` is `1`, `true`, `yes` or `on`.

### Debug output

The CLI's diagnostics go through `tracing` and are printed only when `RUST_LOG` is set: run `RUST_LOG=debug water preview ...` to see them on stderr. A failing preview binary's own stderr is carried in the error the CLI reports.

## Troubleshooting

- **Preview requires `<crate>/dev` feature**: add `dev = ["waterui/dynamic_linking"]` to your `[features]`.
- **Preview backend … does not support platform …**: the message lists the valid combinations; fix the flag or the `Water.toml` declaration.
- **Explicit backend … conflicts with Water.toml backend …**: drop `--backend`, or change `[platforms.<platform>] backend`.
- **No `waterui_preview_*` exports found** (from `--all`): the compiled library has no `#[preview]` function of this crate, for example because a `cfg` removed it.
- **A named target is not found**: the build fails to resolve the `waterui_preview_*` symbol the name maps to. Check the spelling of the function's last path segment.
- **The preview waits then fails naming pending content**: raise `--content-timeout`, or give the preview function data it can show without a network.

## Next

Assert on the same views in a test that runs headless in `cargo nextest`: [Testing](03-testing.md). To look at a running app's tree live, see [Inspector](05-inspector.md); to drive previews and tests from an agent, see [MCP](06-mcp.md).

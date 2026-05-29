# Preview system

> **In this chapter, you will:**
>
> - Mark a view function with `#[preview]` and render it to a PNG with `water preview`
> - Set up the `dev` feature flag and `Water.toml` keys that preview requires
> - Read the preview command's arguments, defaults, and supported platforms
> - Understand the build, handshake, and render path that produces the image

You wrote a card view. You want to see it. Spinning up the simulator, navigating five screens deep, and waiting for a debug build is too much friction for a two-pixel adjustment. The preview system shortcuts that loop: annotate the function, run one command, get a PNG.

Preview is supported on **macOS, the iOS Simulator, physical iOS, and Android** — the four targets that can load a Rust dylib through the WaterUI dynamic-linking path. There is no Linux, Windows, or Web preview backend.

## The `#[preview]` attribute

Mark any function returning `impl View` with `#[preview]`:

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

The macro keeps your original function untouched and generates a `#[unsafe(no_mangle)] extern "C"` companion that constructs the view, wraps it in `AnyView`, and returns the boxed pointer. The preview support app loads that symbol at render time.

### Default arguments for parameterized views

If your view function takes parameters, every parameter needs a default value supplied through the macro attribute. Preview has no other way to invent argument values:

```rust,ignore
#[preview(count = 5, name = "John")]
fn user_card(count: i32, name: &str) -> impl View {
    vstack((
        text!("Name: {name}"),
        text!("Count: {count}"),
    ))
}
```

Forgetting a default produces a compile error pinned to the parameter:

```text
error: Function parameter `count` needs a default value in #[preview(count = ...)]
```

> **Tip:** Pick defaults that resemble real data. A `user_card` previewed with `name = ""` teaches you nothing about typography or wrapping; `name = "John Appleseed"` does.

### Symbol naming

The macro emits exactly one C symbol per preview function:

```text
waterui_preview_{crate_name}_{function_path}
```

`crate_name` is `CARGO_PKG_NAME` with dashes converted to underscores. `function_path` is the path you pass on the command line, with `::` flattened to `_`.

| Crate name      | Function path                  | Export symbol                                         |
|-----------------|--------------------------------|-------------------------------------------------------|
| `my_app`        | `sidebar`                      | `waterui_preview_my_app_sidebar`                      |
| `together-app`  | `dashboard::admin::card`       | `waterui_preview_together_app_dashboard_admin_card`   |

There is no fallback or "leaf-only" alternate; the path you give to `water preview` is the path the symbol resolves against. Misspelling it produces a `Preview component not found` error that prints both the requested function path and the expected export name.

## Project requirements

Preview is a development-mode feature. Two things must be true before `water preview` will work.

### A `dev` feature on your crate

Your root crate must declare a `dev` feature that turns on `waterui/dynamic_linking`:

```toml
# Cargo.toml
[features]
dev = ["waterui/dynamic_linking"]
```

`water preview` reads your `Cargo.toml` and refuses to continue if either the feature or the `waterui/dynamic_linking` enablement is missing — it surfaces the exact line you need to add. The CLI then scaffolds a generated wrapper crate (`managed_backends/preview_ffi`) that depends on your app crate with `features = ["dev"]` and emits the dylib that the support app loads.

### A clean local WaterUI worktree (dev mode)

If `Water.toml` points `waterui_path` at a local checkout, that checkout must be a git worktree with **no uncommitted changes** to runtime-affecting paths (`core/`, `components/`, `ffi/`, `macros/`, `Cargo.lock`, etc.). Preview hashes the clean `HEAD` commit into a *runtime fingerprint* that travels in the TCP handshake. A dirty worktree fails fast with:

```text
Preview dev mode requires a clean WaterUI worktree at <path>.
Commit or stash changes before running preview.
```

For the WaterUI monorepo's own examples and playgrounds, `Water.toml` must explicitly set `waterui_path = "../.."` so the CLI uses the local checkout instead of resolving WaterUI from the registry. Release-mode projects (no `waterui_path`) skip this rule and resolve WaterUI through registry metadata.

## The `water preview` command

```bash
water preview sidebar --platform macos --path ./my-app --output preview.png
```

### Arguments and flags

| Argument / flag           | Description                                           | Default       |
|---------------------------|-------------------------------------------------------|---------------|
| `function_path`           | Function path, e.g. `dashboard::admin::card`          | required      |
| `--platform`, `-p`        | `ios`, `macos`, or `android`                          | required      |
| `--backend`               | `apple`, `android`, or `hydrolysis`                   | per-platform  |
| `--frame`, `-f`           | Render size as `WIDTHxHEIGHT`                         | `375x667`     |
| `--output`, `-o`          | Output PNG path                                       | `preview.png` |
| `--path`                  | Project directory                                     | `.`           |

The default backend follows the platform: `apple` for `ios`/`macos`, `android` for `android`. The `hydrolysis` backend is only valid with `--platform macos` and renders directly through WaterUI's self-drawn renderer instead of a support app. Any other combination is rejected with a clear error.

### Examples

```bash
# Preview a top-level function on macOS
water preview my_view --platform macos

# Preview a nested function with a custom frame size on the iOS Simulator
water preview settings::profile_card --platform ios --frame 390x844

# Preview on Android emulator, save to a specific file
water preview home_screen --platform android --output screenshots/home.png
```

> **Try it:** Add `#[preview]` to any view function, run `water preview <name> --platform macos`, and look for `preview.png` in your project root. Re-run the command — the second invocation reuses the running support app and only rebuilds your dylib.

## How preview works internally

```text
    water preview sidebar --platform macos
                  |
                  v
    1. Resolve preview requirements      (waterui_path, runtime fingerprint)
    2. Try to connect to existing support app via TCP (Ping/Pong)
       - If absent, scaffold ~/.water/preview_support and launch it on platform
    3. Verify handshake: support app fingerprint == expected fingerprint && platform matches
    4. Build managed_backends/preview_ffi as a dylib (cargo + Rust dynamic linking)
    5. Compute DylibId from the build signature + dylib path/size/mtime
    6. Send Render { dylib, symbol, frame } over TCP (or DylibId if app already has it)
    7. Support app loads the dylib via libloading, ad-hoc codesigns on macOS if needed
    8. Resolve waterui_preview_<crate>_<path>, call it, render the AnyView
    9. PNG bytes flow back over TCP and are written to --output
```

### Step 1: support app on disk

The CLI manages a generated WaterUI app at `~/.water/preview_support/`. It is scaffolded the first time you run a preview and re-scaffolded only when the embedded templates or the WaterUI runtime fingerprint change. Its `main` returns a single `Preview` view from the `waterui-preview` crate that owns the TCP server and rendering loop. You never edit it.

### Step 2: TCP handshake

The support app binds a TCP server starting at port 2106 (configurable). The CLI connects, sends `Ping`, and reads `Pong { protocol }`. The protocol struct carries the support app's runtime platform and its WaterUI core fingerprint. Both must match what the CLI computed for the project; otherwise the connection is rejected and the CLI launches a fresh support app for the right runtime.

After the handshake, requests use a binary frame format (4-byte big-endian length prefix + bincode payload). Request types: `Ping`, `HasDylib`, `Render`, `Shutdown`.

### Step 3: dylib build and identity

`water preview` builds the generated `managed_backends/preview_ffi` crate. Because that wrapper depends on your crate with `features = ["dev"]`, the resulting dylib contains every `#[preview]` symbol your code defines.

The CLI assigns each build a `DylibId` derived from a SHA-256 over `(build_signature, dylib path, file length, mtime)`, where `build_signature` includes the runtime fingerprint, target triple, and crate name. This id is the cache key the support app uses to recognise an already-loaded library — the CLI sends only the id when the app already has the bytes, and the full payload otherwise.

### Step 4: render

The support app loads the dylib through `libloading`, resolves the export symbol, calls it to get an `AnyView`, hands it to the platform `ViewRenderer` at the requested frame size, encodes the result as PNG, and ships the bytes back. On macOS, if the initial `dlopen` fails the system applies an ad-hoc `codesign --force --sign -` and retries; you never sign preview dylibs by hand.

## macOS codesigning

System Integrity Protection requires loaded dylibs to be signed. The preview support app handles this transparently: it tries `dlopen`, runs `codesign --verify` on failure, applies an ad-hoc signature with `codesign --force --sign - --timestamp=none` if needed, and retries the load. The ad-hoc signature satisfies the OS without any Apple Developer account.

## Environment variables

The TCP server, on-disk caches, and timeouts are configurable when defaults do not fit your environment:

| Variable                                | Description                          | Default     |
|-----------------------------------------|--------------------------------------|-------------|
| `WATERUI_PREVIEW_HOST`                  | TCP bind/connect address             | `127.0.0.1` |
| `WATERUI_PREVIEW_PORT_START`            | First port to try                    | `2106`      |
| `WATERUI_PREVIEW_PORT_RANGE`            | Number of consecutive ports to scan  | `50`        |
| `WATERUI_PREVIEW_DYLIB_CACHE_SIZE`      | Max in-memory dylib cache entries    | `8`         |
| `WATERUI_PREVIEW_MAX_FRAME_BYTES`       | Max TCP frame size (bytes)           | `128 MiB`   |
| `WATERUI_PREVIEW_CONNECT_TIMEOUT_MS`    | TCP connect timeout                  | `100`       |
| `WATERUI_PREVIEW_HANDSHAKE_TIMEOUT_MS`  | Ping/Pong handshake timeout          | `500`       |
| `WATERUI_PREVIEW_REQUEST_TIMEOUT_MS`    | General request timeout              | `20000`     |
| `WATERUI_PREVIEW_RENDER_TIMEOUT_MS`     | Render request timeout               | `120000`    |

## Build-cache hygiene

Each project gets its own managed build cache under `~/.water/build_cache/<absolute-project-path>/managed_backends/`. Stale entries — caches whose source projects are gone or have not been touched in a while — accumulate over time. The dedicated command to clean them is:

```bash
water gc build-cache
```

`water preview` and `water run` may trigger this in a detached subprocess, but they never scan the cache on the hot path. Run it manually if you want to free disk after archiving an old project.

## Error recovery

The command retries transient failures and prints actionable errors for the rest:

- TCP drops mid-render (broken pipe, EOF, timeout) → relaunch the support app and retry once.
- Symbol not found → print the function path, the expected export symbol, and a `#[preview]` snippet.
- Support app crashes on launch → surface the crash via the device event stream rather than waiting for a timeout.

## Next: hot reload

The preview command builds, loads, and renders a single moment. Re-running it on a saved file gives you a fast iteration loop because the support app and dylib cache survive between invocations. The [next chapter](02-hot-reload.md) breaks that loop down — what is reused, what is rebuilt, and what state does and does not persist between renders.

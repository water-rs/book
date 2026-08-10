# Preview system

> **In this chapter, you will:**
>
> - Mark a view function with `#[preview]` and render it to a PNG with `water preview`
> - Set up the `dev` feature flag and worktree state that preview requires
> - Choose between the native support-app path and the Hydrolysis direct-render path
> - Assert on and profile a preview with `water preview test` and `water preview perf`

Spinning up a simulator, navigating five screens deep, and waiting for a debug build is too much friction for a two-pixel adjustment. The preview system shortcuts that loop: annotate the function, run one command, get a PNG.

Two rendering paths exist, and they have different requirements:

| Path | Command | What it renders |
|------|---------|-----------------|
| Native support app | `water preview <fn> --platform macos\|ios\|android` | Your view through the real Apple or Android backend, via a long-lived support app that loads your code as a dylib |
| Hydrolysis direct | `water preview <fn> --backend hydrolysis --theme material3` | Your view through WaterUI's self-drawn GPU renderer, as a managed offscreen binary — macOS host only |

The support-app path covers **macOS, the iOS Simulator, and Android** — the targets that can load a Rust dylib through WaterUI's dynamic-linking path. There is no Linux, Windows, or Web preview.

## The `#[preview]` attribute

Mark any free function returning `impl View` with `#[preview]`:

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

`#[preview]` only applies to free functions. Putting it on a method with `self` is a compile error.

### Default arguments for parameterized views

If your view function takes parameters, every parameter needs a default value supplied through the macro attribute. Preview has no other way to invent argument values:

```rust,ignore
#[preview(count = 5, name = "John Appleseed")]
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

Naming a parameter that does not exist, or naming one twice, is also a compile error rather than a silently ignored argument.

> **Tip:** Pick defaults that resemble real data. A `user_card` previewed with `name = ""` teaches you nothing about typography or wrapping.

### Symbol naming

The macro emits exactly one C symbol per preview function:

```text
waterui_preview_{crate_name}_{function_name}
```

`crate_name` is `CARGO_PKG_NAME` with dashes converted to underscores. `function_name` is the *bare* function name — a proc macro does not receive the surrounding module path, so the module the function lives in contributes nothing.

| Crate name     | Function                   | Export symbol                       |
|----------------|----------------------------|-------------------------------------|
| `my_app`       | `sidebar`                  | `waterui_preview_my_app_sidebar`    |
| `together-app` | `dashboard::admin::card`   | `waterui_preview_together_app_card` |

**Preview function names must therefore be unique within a crate.** Two `#[preview] fn card()` in different modules produce the same export symbol. `water preview test --all` and `water preview perf --all` detect this during discovery and refuse to run, naming both files.

You may still pass a module path on the command line — only the last segment is used. Misspelling the name produces a `Preview component not found` error that prints both the requested path and the expected export symbol.

## Project requirements

The support-app path is a development-mode feature. Two things must be true before it will work. (The Hydrolysis path builds a managed backend binary instead and needs neither.)

### A `dev` feature on your crate

Your root crate must declare a `dev` feature that turns on `waterui/dynamic_linking`:

```toml
# Cargo.toml
[features]
dev = ["waterui/dynamic_linking"]
```

`water preview` reads your `Cargo.toml` and refuses to continue if either the feature or the `waterui/dynamic_linking` enablement is missing — it surfaces the exact line you need to add. The CLI then scaffolds a generated wrapper crate (`managed_backends/preview_ffi`) that depends on your app crate with `features = ["dev"]` and emits the dylib the support app loads.

### A clean local WaterUI worktree (dev mode)

If `Water.toml` points `waterui_path` at a local checkout, that checkout must be a git worktree with **no uncommitted changes** to runtime-affecting paths (`core/`, `components/`, `ffi/`, `macros/`, `src/`, `utils/`, `backends/`, `kit/`, `icon/`, `Cargo.toml`, `Cargo.lock`, `.gitmodules`, `rust-toolchain*`). Preview hashes the clean `HEAD` commit into a *runtime fingerprint* that travels in the TCP handshake. A dirty worktree fails fast with:

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

| Argument / flag    | Description                                                | Default            |
|--------------------|------------------------------------------------------------|--------------------|
| `target`           | `#[preview]` function name, or an expression with `--expr` | required           |
| `--expr`           | Treat the target as a Rust expression returning `impl View`| off                |
| `--platform`, `-p` | `ios`, `macos`, or `android`                               | host native (macOS)|
| `--backend`        | `apple`, `android`, or `hydrolysis`                        | per-platform       |
| `--theme`          | `material3` — Hydrolysis only, and required there          | none               |
| `--frame`, `-f`    | Render size as `WIDTHxHEIGHT`                              | `375x667`          |
| `--output`, `-o`   | Output PNG path                                            | `preview.png`      |
| `--scenario`       | Hydrolysis scenario TOML for interaction capture           | none               |
| `--output-dir`     | Directory for scenario frames (required with `--scenario`) | none               |
| `--path`           | Project directory                                          | `.`                |

`--platform ios` means the iOS **Simulator**. The default backend follows the platform: `apple` for `ios`/`macos`, `android` for `android`. The valid combinations are `ios/apple`, `macos/apple`, `macos/hydrolysis`, and `android/android`; anything else is rejected by name.

`--expr`, `--scenario`, and `--output-dir` work only with `--backend hydrolysis`.

```bash
# Preview a top-level function on macOS through the Apple backend
water preview my_view --platform macos

# Preview on the iOS Simulator with a custom frame size
water preview profile_card --platform ios --frame 390x844

# Preview on an Android emulator, save to a specific file
water preview home_screen --platform android --output screenshots/home.png

# Render an inline expression through the self-drawn renderer
water preview 'vstack((text("A"), text("B")))' --expr \
    --backend hydrolysis --theme material3
```

### Interaction scenarios

The Hydrolysis path can drive input and capture a timeline instead of a single frame. A scenario is a TOML file listing capture timestamps and events:

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

Event kinds are `pointer_move` (alias `hover`), `pointer_down`, `pointer_up`, `pointer_cancel`, and `scroll` (alias `wheel`). Pointer events need `x`/`y`; `scroll` needs a non-zero `dx` or `dy`; `button` accepts `primary`, `secondary`, or `middle`. One PNG is written into `--output-dir` per capture timestamp, so you can see a ripple mid-flight rather than only its resting state.

## Asserting on and profiling a preview

Two subcommands reuse the same target resolution and run on the Hydrolysis path (macOS host only).

`water preview test` builds the view, produces its accessibility tree without a render target, and runs a Rust automation body against that tree:

```bash
water preview test sidebar --theme material3 \
    --code 'app.query().role(Role::BUTTON).label("Save").assert_exists();'
```

The body receives `app: &mut waterui_testing::SemanticApp`, with `waterui_testing::*` and the WaterUI prelude already in scope. Because the tree under test is the accessibility tree, a preview that fails these assertions is usually an accessibility bug, not a test-harness problem. Use `--code-file` for anything longer than a line, and `--all` to run every `#[preview]` function in the crate.

`water preview perf` profiles the same view through the offscreen GPU pipeline:

```bash
water preview perf sidebar --theme material3 --samples 240 --format html -o perf.html
```

It reports per-phase timings, frame percentiles, rebuild ratio, scene- and clip-layer counts, and cache hit rates. `--warmups` (10), `--samples` (120), and `--repetitions` (7) control the measurement shape. The `--max-p95-us`, `--max-rebuild-ratio`, `--max-scene-layers`, `--max-gpu-surface-layers`, and `--max-clip-layers` thresholds turn a run into a pass/fail gate, which is what makes this usable in CI. `--flamegraph` writes a CPU call-stack SVG, and `--trace` writes a Chrome/Perfetto trace.

## How the support-app path works

```text
    water preview sidebar --platform macos
                  |
                  v
    1. Resolve preview requirements      (waterui_path, runtime fingerprint)
    2. Connect to an existing support app, or scaffold and launch one
    3. Verify handshake: fingerprint and platform must both match
    4. Fingerprint the project's build inputs (SHA-256 over file contents)
    5. Rebuild managed_backends/preview_ffi as a dylib if that fingerprint moved
    6. Compute DylibId from the build signature + dylib path/size/mtime
    7. Send Render { dylib, symbol, frame } (or just the id, if the app has it)
    8. Support app loads the dylib via libloading, ad-hoc codesigns if needed
    9. Resolve the export symbol, render the AnyView, ship PNG bytes back
```

### The support app on disk

The CLI manages a generated WaterUI app at `~/.water/preview_support/`. It is scaffolded the first time you run a preview and re-scaffolded only when the embedded templates or the WaterUI runtime fingerprint change. Its `main` returns a single `Preview` view from the `waterui-preview` crate, which owns the TCP server and rendering loop. You never edit it.

The app shuts itself down after 15 minutes idle (`WATERUI_PREVIEW_IDLE_SHUTDOWN_SECS`).

### Handshake and transport

The support app binds a TCP server starting at port 2106. On macOS it also writes a JSON entry into a local instance registry under the WaterUI cache directory, so the CLI can find a running app by fingerprint instead of scanning ports.

The CLI sends `Ping` and reads `Pong { protocol }`. The protocol struct carries the support app's runtime platform and its WaterUI core fingerprint; both must match what the CLI computed for the project, or the CLI launches a fresh support app for the right runtime.

After the handshake, requests use a length-prefixed binary frame format (4-byte big-endian length + bincode payload). The request set is `Ping`, `HasDylib`, `Render`, and `Shutdown`.

### Build freshness

`water preview` fingerprints your project's build inputs by hashing the *contents* of every build-input file — everything under `src/` and `assets/`, plus top-level `Cargo.toml`, `Cargo.lock`, `Water.toml`, and `build.rs`, plus files with build-input extensions (`.rs`, `.swift`, `.kt`, `.java`, `.metal`, `.wgsl`, `.toml`, `.json`, `.yaml`, `.plist`, and the C-family headers and sources). `target/`, `.git/`, `.jj/`, `.water/`, `node_modules/`, `.gradle/`, `.idea/`, and `.vscode/` are skipped.

That fingerprint goes into a *build signature* alongside the runtime fingerprint, target triple, crate name, and link mode. The signature is written next to the dylib as `<dylib>.waterui-preview-dylib-signature`. If the stored signature matches the one the CLI just computed, the build is skipped entirely — content, not timestamps, decides. Touching a file, or rewriting it with identical bytes, does not trigger a rebuild. Adding or deleting a file does, because relative paths are hashed alongside contents.

### Dylib identity

Each build gets a `DylibId`: a SHA-256 over the build signature, the dylib path, its length, and its mtime. This is the cache key the support app uses to recognise an already-loaded library. The CLI first asks `HasDylib { id }`; on a hit only the render request crosses the wire, and on a miss the CLI sends the bytes — or, on macOS, a local file path, since the CLI and support app share a filesystem.

The support app keeps an in-memory LRU of loaded libraries (default capacity 8).

### Render

The support app loads the dylib through `libloading`, resolves the export symbol, calls it to get an `AnyView`, hands it to the platform `ViewRenderer` at the requested frame size, encodes the result as PNG, and ships the bytes back.

## macOS codesigning

System Integrity Protection requires loaded dylibs to be signed. The support app handles this transparently: it tries `dlopen`, and on failure checks whether the dylib is already signed. If it is, the original load error is reported as-is; if it is not, an ad-hoc signature is applied and the load retried. The ad-hoc signature satisfies the OS without any Apple Developer account, so you never sign preview dylibs by hand.

## Environment variables

The TCP server, on-disk caches, and timeouts are configurable when defaults do not fit your environment. Values that are present but unparseable fail fast rather than falling back to the default.

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
| `WATERUI_PREVIEW_IDLE_SHUTDOWN_SECS`    | Support app idle shutdown            | `900`       |
| `WATER_CACHE_DIR`                       | Root for the preview instance registry | OS cache dir |

## Build-cache hygiene

Playground projects keep their generated backends under `~/.water/build_cache/<absolute-project-path>/managed_backends/` rather than scattering `.water` directories through your source tree. Entries whose source projects are gone, or which have not been used in 30 days, are removed by:

```bash
water gc build-cache
```

Nothing runs this for you. Run it when you want the disk back after archiving old projects.

## Error recovery

- TCP drops mid-render (broken pipe, EOF, timeout) → relaunch the support app and retry once.
- Symbol not found → print the function path, the expected export symbol, and a `#[preview]` snippet.
- Support app crashes on launch → surface the crash through the device event stream rather than waiting out the timeout.

## Next: the iteration loop

A single `water preview` run builds, loads, and renders one moment. The [next chapter](02-preview-iteration.md) covers what survives between runs — the support app, its loaded runtime, and the dylib cache — and what does not.

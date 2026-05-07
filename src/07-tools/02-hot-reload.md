# Hot reload

> **In this chapter, you will:**
>
> - See exactly what the preview pipeline reuses across runs and what it rebuilds
> - Read how the project watcher decides whether the dylib is still fresh
> - Understand the dylib identity that drives the support app's cache
> - Know which platforms support this loop and which kinds of edits force a fresh launch

"Hot reload" in WaterUI today is a re-rendering loop, not a live-attached runtime patch. Each save-and-rerun rebuilds your crate's preview dylib and asks the long-lived preview support app to render it again. The win is that almost everything around your code — the support app process, its TCP connection, its loaded WaterUI runtime, and any cached dylibs — survives between invocations.

The whole pipeline only exists for the targets that can load a Rust dylib through WaterUI's dynamic-linking path: **macOS, the iOS Simulator, physical iOS, and Android.** There is no Linux, Windows, or Web hot-reload story.

## What gets reused, what gets rebuilt

```text
Edit src/views/sidebar.rs and save
        |
        v
You re-run:  water preview sidebar --platform macos
        |
        v
1. CLI reconnects to the running support app over TCP and re-validates the handshake
2. ProjectWatcher scans your project; if nothing changed since the last build, the dylib is reused
3. Otherwise, managed_backends/preview_ffi is rebuilt as a dylib (incremental cargo build)
4. CLI computes the new DylibId; if the support app still has it, only the id is sent
5. Support app loads the (possibly new) dylib, resolves the symbol, renders, returns PNG
```

The first run scaffolds and launches the support app; later runs skip that work entirely. On a warm cache with no source changes, you mostly pay for a TCP round-trip and a fresh render call.

## Project watcher

`ProjectWatcher` decides whether a rebuild is needed by snapshotting your project's input files and comparing the snapshot against the previous one. It watches:

- `src/` and `assets/` (recursive) — all files with build-input extensions
- Top-level files: `Cargo.toml`, `Cargo.lock`, `Water.toml`, `build.rs`
- Build-input extensions include `.rs`, `.toml`, `.json`, `.yaml`, `.swift`, `.kt`, `.java`, shader files (`.metal`, `.wgsl`), and other assets that may be `include_*!`'d at compile time
- Ignored: `target/`, `.water/`, `.git/`, `.jj/`, `node_modules/`, `.gradle/`, `.idea/`, `.vscode/`

Each scan produces both a "latest mtime" and a structural fingerprint over file paths and sizes, so additions and removals are detected even when their mtimes are older than the current dylib.

```rust,ignore
let mut watcher = ProjectWatcher::new();

// First check: always returns changed = true (no previous stamp).
let stamp = watcher.stamp(project_path).await?;
assert!(stamp.changed);

// Second check without modifying files: changed = false.
let stamp = watcher.stamp(project_path).await?;
assert!(!stamp.changed);

// Edit a watched file, then check again: changed = true.
let stamp = watcher.stamp(project_path).await?;
assert!(stamp.changed);
```

The check reads metadata only, so it stays cheap even on large projects.

## Dylib identity

Every preview build produces a `DylibId` that the support app uses as its cache key. The id is a SHA-256 over:

```text
DylibId = SHA-256(
    build_signature  // runtime fingerprint + target triple + crate name + link mode
    || dylib_path
    || file_length
    || mtime_seconds || mtime_subsec_nanos
)
```

Two consequences fall out of this:

1. **Cache hits across runs.** If you re-render without rebuilding, the id is unchanged and the CLI tells the support app `HasDylib { id }`. The app reports that it already has the bytes loaded, and only the render request flies across the wire.
2. **ABI mismatches invalidate automatically.** The build signature embeds the WaterUI runtime fingerprint (the clean `git rev-parse HEAD` of the local `waterui_path` worktree). Changing the WaterUI checkout — even just to a different commit — produces a different id, and the dylib will not be confused with the previous one.

The on-disk dylib lives next to its build signature file at `<dylib>.waterui-preview-dylib-signature`. The support app also keeps an in-memory LRU of loaded libraries (default capacity 8, configurable via `WATERUI_PREVIEW_DYLIB_CACHE_SIZE`).

## The reload cycle, step by step

### 1. Watcher scan

The CLI calls `ProjectWatcher::stamp` and gets back the latest mtime plus a `changed` flag. The mtime is fed into the freshness check on disk; `changed` only signals that the watcher itself saw a difference.

### 2. Freshness check

If a dylib already exists on disk, its mtime is at least as new as the project's latest mtime, **and** its stored build signature still matches the one the CLI just computed, the build is skipped. Otherwise the CLI runs `cargo build` on `managed_backends/preview_ffi` with `-Cprefer-dynamic` so WaterUI itself is also dynamically linked.

### 3. Identity and transfer

The CLI hashes the new dylib into a `DylibId` and queries the support app with `HasDylib { id }`. If the answer is "no", the bytes are streamed in the next `Render` request; on macOS the CLI prefers passing a local file path to the in-process support app instead of copying the bytes.

### 4. Load and render

The support app stores the bytes (or maps the file) and loads them through `libloading`. macOS retries with an ad-hoc `codesign` if the initial `dlopen` is rejected. The export symbol is resolved, called once to produce a fresh `AnyView`, and rendered through the platform `ViewRenderer`. The PNG bytes ship back to the CLI.

## Persistent sessions

The support app outlives any single CLI invocation. After a successful render the CLI calls `session.detach()`, which intentionally `mem::forget`s the `Running` handle so dropping it does not kill the app. Subsequent commands reuse the connection.

On failure the CLI calls `session.shutdown()` instead — it sends a `Shutdown` request and drops the handle so the next invocation gets a clean process.

The practical timing this produces:

- **First** invocation: scaffold + launch + first build (a few seconds).
- **Subsequent** invocations on unchanged code: TCP reconnect + cached render (sub-100 ms once the OS is warm).
- **Subsequent** invocations after editing one file: incremental cargo rebuild + fresh render, typically a few seconds with `sccache` warm.

> **Tip:** If consecutive previews feel slow, look for `Connected to existing preview app` versus `No preview app running, launching...` in the CLI logs. The second message means something invalidated the running app — usually a `waterui_path` change or a runtime fingerprint mismatch.

## Build caching with sccache

The preview build path threads `sccache` into `RustBuild` automatically when it can find the binary. With sccache warm, incremental rebuilds are dominated by linker time. Without sccache the CLI prints a one-time hint:

```text
sccache not found. Build efficiency may be reduced. Install with: brew install sccache
```

Install it once and forget about it; nothing in the preview pipeline disables sccache, and you should not set `WATERUI_DISABLE_SCCACHE=1` because doing so makes the per-project build cache balloon.

## State across reloads

Every `water preview` call asks the support app to re-render a brand-new `AnyView`. The view tree is rebuilt from the new code — that is the entire point. There is no shared `Binding`, `Computed`, or `Environment` carried over between invocations: the support app constructs each preview from scratch, drives one render, and drops everything it built.

If you want a preview that exercises a specific data shape, set up that data inside the preview function or pass it via `#[preview(...)]` defaults. Do not expect the support app to remember inputs from the previous run.

## Per-function granularity

`#[preview]` works at the function level. Mark as many functions as you like in the same crate; each gets its own export symbol:

```rust,ignore
#[preview]
fn sidebar() -> impl View { /* ... */ }

#[preview]
fn header() -> impl View { /* ... */ }

#[preview(count = 3)]
fn notification_list(count: usize) -> impl View { /* ... */ }
```

Switching between them shares the same dylib and the same support-app session:

```bash
water preview sidebar --platform macos
water preview header --platform macos
water preview notification_list --platform macos
```

## When you need a full restart

This loop covers most edits. A few cases still force you to relaunch the support app:

- **WaterUI runtime changed.** Bumping the local `waterui_path` checkout, switching commits, or leaving a dirty WaterUI worktree changes the runtime fingerprint. The handshake will reject the running support app and the CLI launches a fresh one.
- **`Water.toml` or backend configuration changed.** The watcher catches the file change, but switching backends or platforms also invalidates the support app.
- **Support app crashed.** When the CLI sees the support app exit, it shuts the session down so the next preview launches a clean process.

Struct layout changes inside *your* crate are safe by themselves: every preview rebuilds your dylib from source and constructs a fresh view tree, so there is no stale binding state to corrupt. Layout problems only matter if you keep state outside the preview's view tree, and the preview path explicitly does not.

## Architecture summary

```text
+------------------+       TCP (port 2106+)       +------------------------+
|                  | <--------------------------> |                        |
|   water CLI      |   Binary protocol (bincode)  |  Preview support app   |
|                  |                              |                        |
+--------+---------+                              +-----------+------------+
         |                                                    |
   Build managed_backends/preview_ffi             Load dylib via libloading
   as a dylib (cargo + sccache)                   Ad-hoc codesign on macOS
         |                                        Resolve preview symbol
   Watch project inputs (ProjectWatcher)          Render AnyView via native
         |                                        ViewRenderer, encode PNG
   Compute DylibId
   (build signature + path/size/mtime)            LRU dylib cache
```

The build-and-watch side and the load-and-render side communicate only over TCP. That is what lets the support app survive across CLI runs, and that is the whole basis of the "hot" feel.

## Next: how WaterUI renders

You now know what the developer tools accelerate. The [Internals](../08-internals/01-rendering.md) section opens the box on the runtime they accelerate: how WaterUI walks the view tree, crosses the FFI boundary, and lays out widgets on each platform.

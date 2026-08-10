# The preview iteration loop

> **In this chapter, you will:**
>
> - Learn why WaterUI's preview system replaces hot reload
> - See exactly what the preview pipeline reuses across runs and what it rebuilds
> - Read how content fingerprinting decides whether the dylib is still fresh
> - Know which edits force a fresh support app

WaterUI does not support hot reload; the preview system replaces it. There is no file watcher, no daemon polling your source tree, and no patching of a running app's view tree. `water preview` is a one-shot command: you save a file, you run it again.

What makes that loop fast is that almost everything around your code survives between invocations — the support app process, its loaded WaterUI runtime, its TCP connection, and its cache of previously loaded dylibs. Only your crate is rebuilt, and only when its contents actually changed.

This chapter is about the support-app path (macOS, iOS Simulator, Android). The Hydrolysis path (`--backend hydrolysis`) has no long-lived process at all: it rebuilds and re-runs a managed offscreen binary each time.

## What gets reused, what gets rebuilt

```text
Edit src/views/sidebar.rs and save
        |
        v
You re-run:  water preview sidebar --platform macos
        |
        v
1. CLI reconnects to the running support app and re-validates the handshake
2. CLI hashes every build-input file; if the hash matches the last build, the dylib is reused
3. Otherwise managed_backends/preview_ffi is rebuilt (incremental cargo build)
4. CLI computes the new DylibId; if the support app still has it, only the id is sent
5. Support app loads the (possibly new) dylib, resolves the symbol, renders, returns PNG
```

The first run scaffolds and launches the support app; later runs skip that entirely.

## Freshness is content, not timestamps

Before every build the CLI walks your project and hashes the *contents* of each build-input file, together with its relative path and length, into one SHA-256 fingerprint. Inputs are everything under `src/` and `assets/`, the top-level `Cargo.toml`, `Cargo.lock`, `Water.toml`, and `build.rs`, and any file with a build-input extension (`.rs`, `.swift`, `.kt`, `.java`, `.metal`, `.wgsl`, `.toml`, `.json`, `.yaml`, `.plist`, and the C-family sources and headers). `target/`, `.git/`, `.jj/`, `.water/`, `node_modules/`, `.gradle/`, `.idea/`, and `.vscode/` are skipped. Symlinked source directories are followed, so a project whose `src/` points elsewhere is still tracked correctly.

That fingerprint is folded into a build signature:

```text
build_signature =
    inputs=<sha256 of project build inputs>
    runtime=<waterui runtime fingerprint>
    target=<target triple>
    crate=<preview crate name>
    link_mode=<dylib or cdylib, prefer-dynamic>
```

The signature is stored next to the dylib as `<dylib>.waterui-preview-dylib-signature`. If the stored signature equals the one just computed, the build is skipped.

Hashing content rather than mtimes means an editor that rewrites a file on save, a `touch`, or a checkout that restores identical bytes all cost nothing. Adding or removing a file is still caught, because each file's relative path is hashed alongside its contents.

## Dylib identity

The build signature also feeds the `DylibId` that the support app uses as its cache key:

```text
DylibId = SHA-256(
    build_signature
    || dylib_path
    || file_length
    || mtime_seconds || mtime_subsec_nanos
)
```

The CLI asks `HasDylib { id }` first. On a hit only the render request crosses the wire. On a miss the CLI sends the dylib — as raw bytes, or, when the CLI and support app share a filesystem (macOS local preview), as a local file path so a multi-megabyte payload never goes through the socket.

Because the build signature embeds the WaterUI runtime fingerprint — the clean `git rev-parse HEAD` of the local `waterui_path` worktree — changing your WaterUI checkout produces a different id, and the old dylib can never be mistaken for the new one.

The support app keeps an in-memory LRU of loaded libraries (default capacity 8, configurable via `WATERUI_PREVIEW_DYLIB_CACHE_SIZE`).

## Persistent sessions

The support app outlives any single CLI invocation. After a successful render the CLI calls `session.detach()`, which clears the drop hooks that would otherwise terminate the app when the CLI exits, so the next command finds it still listening.

On failure the CLI calls `session.shutdown()` instead: it sends a `Shutdown` request and drops the handle, so the next invocation starts from a clean process rather than inheriting a broken one.

Left alone, the support app exits by itself after 15 minutes of inactivity (`WATERUI_PREVIEW_IDLE_SHUTDOWN_SECS`).

The timings this produces:

- **First** invocation: scaffold, launch, and full build — several seconds.
- **Subsequent** invocations on unchanged code: reconnect plus a cached render.
- **Subsequent** invocations after an edit: incremental cargo rebuild plus a fresh render.

> **Tip:** If consecutive previews feel slow, look for `Connected to existing preview app` versus a fresh launch in the CLI logs (`--logs debug`). A fresh launch means something invalidated the running app — usually a `waterui_path` change or a runtime fingerprint mismatch.

## Build caching with sccache

The preview build path threads `sccache` into the build automatically when it can find the binary. Without it the CLI prints a one-time hint:

```text
sccache not found. Build efficiency may be reduced. Install with: brew install sccache
```

Install it once and forget about it. Nothing in the preview pipeline disables sccache, and you should not set `WATERUI_DISABLE_SCCACHE=1` — doing so makes per-project build caches balloon.

## State across reloads

Every `water preview` call asks the support app to construct a brand-new `AnyView` and render it once. There is no shared `Binding`, `Computed`, or `Environment` carried over between invocations: the support app builds each preview from scratch, drives one render, and drops everything.

If a preview needs to exercise a specific data shape, set that data up inside the preview function or pass it through `#[preview(...)]` defaults. Do not expect the support app to remember anything from the previous run.

This is also why struct layout changes inside your own crate are safe: nothing stateful survives to be corrupted.

## Per-function granularity

`#[preview]` works at the function level. Mark as many functions as you like in one crate — each gets its own export symbol, and switching between them shares the same dylib and the same support-app session:

```rust,ignore
#[preview]
fn sidebar() -> impl View { /* ... */ }

#[preview]
fn header() -> impl View { /* ... */ }

#[preview(count = 3)]
fn notification_list(count: usize) -> impl View { /* ... */ }
```

```bash
water preview sidebar --platform macos
water preview header --platform macos
water preview notification_list --platform macos
```

Remember that the export symbol uses only the crate name and the bare function name, so these three names must be unique across the whole crate.

## When you need a fresh support app

- **The WaterUI runtime changed.** Bumping the local `waterui_path` checkout, switching commits, or leaving that worktree dirty changes the runtime fingerprint. The handshake rejects the running support app and the CLI launches a new one.
- **You switched platform or backend.** A support app is built for one runtime platform; the handshake checks that too.
- **The support app crashed.** When the CLI sees it exit, it shuts the session down so the next preview gets a clean process.
- **The CLI itself was rebuilt.** The support app is re-scaffolded when its embedded templates change.

## Architecture summary

```text
+------------------+       TCP (port 2106+)       +------------------------+
|                  | <--------------------------> |                        |
|   water CLI      |   Binary protocol (bincode)  |  Preview support app   |
|                  |                              |                        |
+--------+---------+                              +-----------+------------+
         |                                                    |
   Hash project build inputs                       Load dylib via libloading
         |                                         Ad-hoc codesign on macOS
   Build managed_backends/preview_ffi              Resolve preview symbol
   as a dylib (cargo + sccache)                    Render AnyView via native
         |                                         ViewRenderer, encode PNG
   Compute DylibId                                 LRU dylib cache
```

The build side and the render side share nothing but the socket. That separation is what lets the support app survive across CLI runs, and it is the whole basis of the "hot" feel.

## Next: how WaterUI renders

The [Internals](../08-internals/01-rendering.md) section opens the box on the runtime these tools accelerate: how WaterUI walks the view tree, crosses the FFI boundary, and turns nodes into widgets.

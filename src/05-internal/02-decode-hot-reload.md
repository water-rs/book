# Decode hot reload in WaterUI

Hot reload is a popular feature for web programmers. WaterUI piggybacks on the CLI to stream new
dynamic libraries into a running app so you can iterate quickly without restarting the process.

Rust is ahead-of-time compiled, so the trick is to rebuild just your view crate, send the resulting
`*.dylib`/`*.dll`/`*.so` over a WebSocket, and `dlopen` it inside the running process. That is
precisely what `waterui::hot_reload::Hotreload` does.

## How It Works

1. `Hotreload::new(view)` renders the initial view via `Dynamic`.
2. A background thread connects to the CLI (`water run --hot-reload`) using the
   `WATERUI_HOT_RELOAD_PORT` environment variable.
3. Whenever the CLI finishes compiling, it streams the new library bytes.
4. The loader writes them to `./hot_reload/waterui_hot_<uuid>.{dylib,so,dll}` and uses `libloading`
   to fetch the exported `waterui_main` symbol.
5. The new view is handed to the original `Dynamic`, which rebuilds the UI in place.

## Usage

```rust
use waterui::hot_reload::Hotreload;

fn main_view() -> impl View {
    Hotreload::new(app())
}
```

Start the CLI in hot-reload mode:

```bash
water run --platform mac --hot-reload
```

Each save triggers Cargo to rebuild the crate, the CLI streams the binary, and the view updates
without restarting simulators or the browser.

## Limitations

- Only the main view crate is reloaded. Changes to dependencies still require a full restart.
- Keep ABI-compatible signatures; `Hotreload` expects an exported `extern "C" fn -> *mut AnyView`.
- Clean up temporary libraries in `./hot_reload/` occasionally—they accumulate as you iterate.

Despite the constraints, hot reload dramatically shortens the feedback loop for UI tweaks, copy
updates, and layout experiments.

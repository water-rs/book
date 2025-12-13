# Hot Reload Internals

Hot reload allows you to modify your Rust code and see changes instantly in the running application without restarting the app or losing state. WaterUI achieves this by compiling your code into a dynamic library (`.dylib`, `.so`, or `.dll`) and injecting it into the running process.

## Architecture

The system consists of three parts:

1.  **The CLI (`waterui-cli`)**: Watches your source files, rebuilds the project as a dynamic library when changes are detected, and hosts a WebSocket server.
2.  **The Runtime (`HotReload` View)**: A special view component in your app that connects to the CLI, downloads the new library, and swaps the view pointer.
3.  **The Macro (`#[hot_reload]`)**: An attribute macro that instruments functions to be individually reloadable.

## Per-Function Hot Reload

You can mark specific view functions with `#[hot_reload]`. This wraps the function body in a `HotReloadView`.

```rust
#[hot_reload]
fn my_feature() -> impl View {
    vstack((
        text("Edit me!"),
        button("Click me", || println!("Clicked")),
    ))
}
```

The macro generates a unique C-exported symbol for this function (e.g., `waterui_hot_reload_my_feature`).

## The Reload Process

1.  **Change Detection**: The CLI detects a file save.
2.  **Rebuild**: It runs `cargo build` with the `waterui_hot_reload_lib` configuration.
3.  **Broadcast**: The new binary is broadcast over WebSocket to the running app.
4.  **Load**: The app writes the binary to a temporary file and loads it using `libloading`.
5.  **Swap**: The `HotReloadView` looks up its specific symbol in the new library. If found, it calls the function to get the new view structure and replaces its current content.

## State Preservation

Because only the view construction logic is reloaded, the underlying state (held in `Binding`s or the `Environment`) is preserved. The new view structure simply re-binds to the existing state.

## Limitations

*   **Symbol Stability**: The function signature must return `impl View`.
*   **Global State**: Changes to global state initialization or `main` entry points usually require a full restart.
*   **Struct Layout**: Changing the fields of a struct that is shared between the main app and the hot-reloaded library can cause undefined behavior due to ABI mismatches. It is safest to tweak view logic and local variables.
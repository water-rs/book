# How WaterUI Renders

WaterUI renders your application by translating the Rust view tree into native platform widgets (SwiftUI on Apple platforms, Jetpack Compose on Android). This process is mediated by a C FFI layer (`waterui-ffi`) that allows the native runtime to traverse and observe the Rust view hierarchy.

## The Rendering Pipeline

1.  **Tree Construction**: When your app starts, WaterUI creates the initial view tree defined in your `app` function. This tree is composed of `View` structs.
2.  **FFI Traversal**: The native backend (written in Swift or Kotlin) holds a pointer to the root Rust view. It calls `waterui_view_id` to identify the type of the view.
3.  **Leaf vs. Composite**:
    *   **Raw Views (Leafs)**: If the view is a "Raw View" (like `Text`, `Button`, `VStack`), the backend downcasts it using `waterui_force_as_*` functions to extract the configuration (e.g., the text string, the button action). It then creates the corresponding native widget (e.g., `SwiftUI.Text`).
    *   **Composite Views**: If the view is a custom component (like your `MyView`), the backend calls `waterui_view_body`, which triggers the `View::body` method in Rust. This expands the view into its underlying structure. The backend then repeats the process for the returned body.
4.  **Reactive Updates**: When a view uses a reactive value (like a `Binding` or `Computed`), the native backend registers a watcher. When the value changes in Rust, the watcher notifies the native backend, triggering a UI update for that specific component.

## The FFI Layer

The `waterui-ffi` crate is the bridge between Rust and the host platform. It exports:

*   **Entry Points**: `waterui_init` and `waterui_app` to bootstrap the runtime.
*   **Type Identification**: A stable 128-bit type ID system (`WuiTypeId`) that allows the native side to recognize Rust types, even across dynamic library reloads.
*   **Opaque Types**: wrappers like `WuiAnyView` and `WuiEnv` that allow passing complex Rust objects as pointers.
*   **Property Accessors**: C-compatible functions to read properties from configuration structs (e.g., `waterui_read_binding_i32`).

## Talking to Native

`waterui_core::Native<T>` is the wrapper for platform-specific content. Views that implement the `NativeView` trait can be wrapped in `Native<T>`. This signals to the framework that this node should be handled by the platform backend directly, rather than being expanded further in Rust.

For example, `Text` is a `NativeView`. When you write `text("Hello")`, it creates a `Text` struct. The FFI layer exposes this to Swift/Kotlin, which then renders a system label.

## Diffing and Identity

For lists and collections, WaterUI uses the `Identifable` trait. When a list changes, the framework uses these IDs to calculate a diff. This information is passed to the native backend's list implementation (e.g., `ForEach` in SwiftUI), ensuring efficient updates and animations for insertions, deletions, and moves.

## Performance Implications

*   **Logic in Rust**: Your business logic, state management, and layout calculations happen in Rust.
*   **Rendering in Native**: The expensive work of drawing pixels, text layout, and compositing is handled by the OS's highly optimized UI toolkit.
*   **Boundary Crossing**: Crossing the FFI boundary has a small cost. WaterUI minimizes this by only crossing when necessary (initial build and reactive updates) and by using efficient pointer-based accessors.
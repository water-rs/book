# How WaterUI Renders

Every component in WaterUI implements the `View` trait. At runtime, the framework walks the view
tree, gathers metadata, and hands the resulting configuration to the active backend (SwiftUI, GTK4,
Web). Understanding the pipeline helps when you are debugging diffing issues or wiring up native
views.

## Rendering Pipeline

1. **Body expansion** – Calling `View::body` recursively resolves function components, custom
   structs, and macros until only concrete views remain (text, list, buttons, etc.).
2. **Environment merge** – Each node clones the environment, adds its own values (`.with`,
   `.metadata`), and passes it down to children.
3. **Configuration** – Views that implement `ConfigurableView` produce `FooConfig` structs. Hooks in
   the environment can intercept them (`env.insert_hook`) to adjust titles, colors, layouts.
4. **Type erasure** – Everything collapses into `AnyView`/`Native` nodes so the backend can handle a
   uniform representation.
5. **Renderer** – Platform adapters (SwiftUI representables, GTK widgets, DOM builders) convert
   configs into native UI and subscribe to bindings for updates.

## Talking to Native

`waterui_core::Native<T>` is the escape hatch for platform-specific content. The backend
implements `From<Native<T>>` and can use the payload to mount UIKit views, HTML elements, etc. When
you write custom wrappers (canvas, shaders), you are effectively defining new `Native` payloads.

## Diffing

Collections rely on the `Views` trait (`ForEach`, `List`, `NavigationPath`). Each item must expose a
stable identifier via `Identifable`. Backends receive insertion/deletion events instead of whole
array rebuilds, which keeps scrolling and animations smooth.

Understanding these internals helps you reason about performance: expensive work should happen in
signals or async tasks, not inside `body`. Let the renderer do what it does best—translating the
final configs into native widgets.

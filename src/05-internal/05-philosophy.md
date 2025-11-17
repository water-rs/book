# Philosophy

WaterUI’s design balances three goals:

1. **Native-first** – We render through native toolkits (SwiftUI, GTK4, DOM) so apps feel at home on
   every platform.
2. **Reactive without runtime magic** – Everything is ordinary Rust: views are plain functions,
   bindings are typed handles, and async uses futures/tasks. There is no bytecode interpreter, no
   reflection.
3. **Composable** – Views, environments, and plugins snap together so teams can build their own
   component libraries on top of the same primitives.

This is why APIs emphasize:

- `impl View` return types instead of heavyweight objects.
- Environment-based dependency injection rather than singletons.
- Opt-in hooks (for theming, localization, analytics) instead of invasive global state.

When contributing or building your own components, keep these principles in mind—favor explicit
Rust types, lean on the reactive system, and let platform backends do the rendering.

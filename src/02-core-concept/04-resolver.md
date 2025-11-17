# Resolvers, Environment Hooks, and Dynamic Views

The WaterUI core works because values can be resolved lazily against the `Environment` and streamed into the renderer. This chapter explains the pieces you will see throughout the book.

## The `Resolvable` Trait

```rust
use waterui_core::{Environment, Signal, constant, resolve::Resolvable};

#[derive(Debug, Clone)]
struct LocalizedTitle;

impl Resolvable for LocalizedTitle {
    type Resolved = String;

    fn resolve(&self, env: &Environment) -> impl Signal<Output = Self::Resolved> {
        let locale = env
            .get::<String>()
            .cloned()
            .unwrap_or_else(|| "en".to_string());
        constant(match locale.as_str() {
            "en" => "Hello".to_string(),
            "fr" => "Bonjour".to_string(),
            _ => "Hello".to_string(),
        })
    }
}
```

`Resolvable` types take an environment reference and produce any `Signal`. When WaterUI stores them inside `AnyResolvable<T>`, the system can defer value creation until the view tree is mounted on a backend. This is how themes, localization, and other contextual data flow without explicit parameters.

## Hooks and Metadata

`Environment::insert_hook` injects `Hook<ViewConfig>` values that can wrap every view of a given configuration type. Hooks are typically installed by plugins (e.g., a theming system) so that they can read additional metadata and rewrite the view before rendering. Because hooks remove themselves from the environment while running, recursion is avoided.

## Dynamic Views

`Dynamic::watch` bridges any `Signal` into the `View` world. The helper subscribes to the signal, sends the initial value, and keeps the view tree alive via a guard stored in `With` metadata. You will use it heavily when wiring `nami` bindings into text, lists, or network-backed UIs.

Throughout the component chapters we will point back here whenever a control takes an `impl Resolvable` or internally creates a `Dynamic` to stream updates.

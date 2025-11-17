# Best Practices for Library Authors

Building reusable components for WaterUI means working with the same primitives as the core
framework. Follow these guidelines to keep contributions consistent and ergonomic.

## Favor Function Views

Expose public APIs as `pub fn foo(...) -> impl View`. Reserve dedicated structs for cases where you
need to capture configuration for later (e.g., view modifiers, list containers). Function views keep
call sites concise and let the compiler optimize away intermediate structs.

## Embrace `ConfigurableView`

When a component needs hooks (theming, localization), implement `ConfigurableView` and expose a
`FooConfig`. This allows plugins to intercept configuration via `env.insert_hook` and keeps the
rendering path flexible.

```rust
pub struct BadgeConfig { /* … */ }

impl ConfigurableView for Badge {
    type Config = BadgeConfig;
    fn config(self) -> Self::Config { /* … */ }
}
```

## Use `Binding` and Signals

All stateful components should accept `Binding<T>` or `impl Signal<Output = T>`. Do not expose raw
`Arc<Mutex<T>>` or other patterns. This keeps your component compatible with the reactive system and
lets advanced users feed derived signals into it.

## Prefer Environment Extractors

Need shared services? Define an extractor (`impl Extractor for FooService`) so handlers and views
can pull the service from the environment without tight coupling.

## Document Crates Locally

Every component crate (controls, navigation, form) includes a README. Update it alongside code
changes so the tutorial book and API docs stay in sync.

## Test with `mdbook test`

If a chapter introduces a component, back it with a doctest or shared example in `src/lib.rs`. This
keeps snippets compiling as the API evolves.

Consistent APIs make WaterUI approachable. Build on the same primitives (View, Environment,
Bindings), offer hooks for customization, and document behaviors clearly.

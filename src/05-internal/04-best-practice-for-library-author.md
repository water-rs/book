# Best Practices for Library Authors

When building reusable component libraries for WaterUI, follow these patterns to ensure your components are idiomatic, performant, and easy to use.

## 1. Use the `configurable!` Macro

For views that have properties (like `Text`, `Button`, `Slider`), use the `configurable!` macro. This generates the boilerplate for `ConfigurableView`, `ViewConfiguration`, and the builder pattern methods.

```rust
use waterui_core::configurable;

pub struct BadgeConfig {
    pub label: AnyView,
    pub color: Computed<Color>,
}

configurable!(Badge, BadgeConfig);

impl Badge {
    pub fn new(label: impl View) -> Self {
        Self(BadgeConfig {
            label: AnyView::new(label),
            color: Color::RED.into_computed(),
        })
    }

    // Builder method
    pub fn color(mut self, color: impl IntoComputed<Color>) -> Self {
        self.0.color = color.into_computed();
        self
    }
}
```

## 2. Accept `IntoComputed` / `IntoSignal`

Allow users to pass either static values or reactive bindings by accepting `impl IntoComputed<T>` or `impl IntoSignal<T>`.

```rust
pub fn color(mut self, color: impl IntoComputed<Color>) -> Self {
    self.0.color = color.into_computed();
    self
}
```

This lets users write `.color(Color::RED)` (static) or `.color(my_binding)` (reactive) interchangeably.

## 3. Use `Environment` for Context

Avoid passing global configuration (like themes) through arguments. Use the `Environment` instead. Define a struct for your config and implement `Extractor`.

```rust
#[derive(Clone)]
struct MyTheme {
    border_radius: f32,
}

// In your view
use_env(|Use(theme): Use<MyTheme>| {
    // use theme.border_radius
})
```

## 4. Leverage `ViewExt`

The `ViewExt` trait provides common modifiers like `.padding()`, `.background()`, and `.gesture()`. Implement your component as a `View` so users can chain these modifiers.

## 5. Favor Composition

Build complex components by composing existing primitives (`VStack`, `Text`, `Shape`) rather than trying to implement custom rendering logic, unless absolutely necessary. Composition ensures your component benefits from the native backend's optimizations and accessibility features.

## 6. Testing

Use the `waterui-macros` crate to create testable forms and view structures. Ensure your component works with the reactive system by testing it with `Binding` updates.

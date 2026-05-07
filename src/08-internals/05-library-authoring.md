# Library Authoring

> **In this chapter, you will:**
>
> - Use `configurable!` and `raw_view!` to create hookable and simple views
> - Apply the `Type::new` / free-function constructor split that WaterUI uses everywhere
> - Accept `IntoText`, `IntoLabel`, `IntoSignal<T>`, and `IntoComputed<T>` for ergonomic APIs
> - Pass context through the `Environment` and the `Plugin` trait
> - Follow best practices for composition, testing, and API design

WaterUI is designed for extensibility. Whether you are building a shared component
library for your team or an open-source package for the community, the framework provides patterns and macros that help you create clean, composable, and type-safe APIs. This chapter covers the tools and best practices that separate a good WaterUI library from a great one.

## The `configurable!` Macro

The `configurable!` macro is the standard way to create views that support both
builder-pattern configuration and environment-based hooking. This is the pattern you want when your view should be customizable by downstream consumers:

```rust,ignore
configurable!(Button, ButtonConfig);
configurable!(Slider, SliderConfig, StretchAxis::Horizontal);
configurable!(Progress, ProgressConfig, |config| match config.style {
    ProgressStyle::Linear => StretchAxis::Horizontal,
    ProgressStyle::Circular => StretchAxis::None,
});
```

This macro generates:

1. A **wrapper struct** (e.g., `Button`) that holds the config.
2. **`NativeView` impl** on the config type, declaring the stretch axis.
3. **`ConfigurableView` impl** on the wrapper, exposing `config()`.
4. **`ViewConfiguration` impl** on the config, with a `render()` method.
5. **`View` impl** that checks for environment hooks before falling through to
   native rendering.

The hook mechanism allows library consumers to globally customize how a view
renders without modifying the library code:

```rust,ignore
let mut env = Environment::new();
env.insert_hook(|env: &Environment, config: ButtonConfig| {
    // Return a completely custom button implementation
    custom_button(config.label, config.action)
});
```

> **Tip:** Think of `configurable!` as "I am defining this view, but I want consumers to be able to override it." If you do not need that override capability, use `raw_view!` instead.

### Three Stretch Axis Modes

The macro supports three patterns for declaring stretch behavior:

```rust,ignore
// Static: Always the same stretch axis
configurable!(MyView, MyConfig);                          // StretchAxis::None
configurable!(MyView, MyConfig, StretchAxis::Horizontal); // Always horizontal

// Dynamic: Depends on configuration at runtime
configurable!(MyView, MyConfig, |config| {
    if config.is_expanded { StretchAxis::Both } else { StretchAxis::None }
});
```

## The `raw_view!` Macro

For simpler leaf views that do not need hookability, use `raw_view!`:

```rust,ignore
raw_view!(Divider, StretchAxis::CrossAxis);
raw_view!(Spacer, StretchAxis::MainAxis);
raw_view!(Image);  // StretchAxis::None by default
```

This implements `NativeView` and `View` without the `ConfigurableView` / `Hook`
machinery. Use `raw_view!` when:

- The view has no meaningful configuration to hook.
- You want the simplest possible implementation.
- The view is internal to your library and not meant to be customized.

## The Constructor Split

WaterUI is consistent about how public APIs expose construction, and your library
should follow the same convention:

- **`Type::new(...)` is the general constructor.** It takes the most general
  shape the component can render -- typically a fully open `impl View` for the
  label slot, plus all the dials a power user might need.
- **Free function constructors like `button(...)` are ergonomic entry points.**
  They accept narrower semantic input types (`IntoLabel`, `IntoText`) so that
  string literals and i18n-friendly text fall into the right semantic pipeline
  with sensible default accessibility.

Do not introduce parallel `Type::custom(...)` shapes -- if `Type::new(...)` is
not flexible enough, fix `Type::new`.

```rust,ignore
// General: arbitrary visual composition for the label, action chained after.
let custom = Button::new(my_view).action(|env: Environment| { /* ... */ });

// Ergonomic: literal flows into the i18n-aware semantic text pipeline,
// and accessibility defaults are inherited automatically.
let ergonomic = button("Save");
```

## Flexible Input Types

A great library API does not force users to think about type conversions. WaterUI
provides traits that accept the widest reasonable input types so callers can pass
whatever is most natural.

### `IntoText` and `IntoLabel`

Prefer `IntoText` for semantic text and `IntoLabel` for labelled controls
(buttons, toggles, fields). These traits route literals, `String`, `Str`,
`StyledStr`, and reactive `Computed<T>` through WaterUI's i18n-aware semantic
text pipeline -- so accessibility and localization come for free:

```rust,ignore
use waterui::text::IntoText;

pub fn caption(content: impl IntoText) -> Text {
    Text::new(content).font(Font::caption())
}

caption("Saved");                  // &'static str -> SemanticText
caption(String::from("Saved"));    // String
caption(text!("Saved at {now}")); // reactive content via text! macro
```

Only fall back to a raw `impl View` when the slot really is "arbitrary visual
composition," not a textual label.

### `IntoSignal<T>` and `IntoComputed<T>`

For non-textual reactive inputs, accept `IntoSignal<T>` (or `IntoComputed<T>`
when you specifically need a derived value) so callers can pass either a
constant or a reactive source without wrapping it in `Computed::constant()`:

```rust,ignore
pub fn opacity(value: impl IntoComputed<f32>) -> Opacity {
    Opacity { value: value.into_computed() }
}

opacity(0.5);            // f32 constant
opacity(my_binding);     // Binding<f32>
opacity(computed_value); // Computed<f32>
```

### `IntoSignalF32`

A specialized trait for `f32` values that also accepts integers:

```rust,ignore
pub fn spacing(value: impl IntoSignalF32) -> f32 {
    value.into_signal_f32()
}

spacing(8)      // i32 -> f32
spacing(8.0)    // f32
spacing(8u32)   // u32 -> f32
```

## Environment for Context Passing

The `Environment` is a type-indexed key-value store. Libraries can define custom
environment keys to pass context through the view hierarchy without threading parameters through every function call:

```rust,ignore
use waterui_core::env::Store;

// Define a theme token
pub struct AccentColor;

// Install into environment
let mut env = Environment::new();
env.insert(Store::<AccentColor, Color>::new(Color::blue()));

// Read in a child view
pub fn themed_button() -> impl View {
    use_env(|env: &Environment| {
        let color = env.query::<AccentColor, Color>()
            .unwrap_or(&Color::blue());
        button("Tap me").tint(*color)
    })
}
```

### The Plugin Trait

For libraries that need to install multiple values, implement `Plugin`:

```rust,ignore
pub trait Plugin {
    fn install(&self, env: &mut Environment) {
        // Default: no-op
    }
}

pub struct MyLibraryPlugin {
    pub theme: MyTheme,
}

impl Plugin for MyLibraryPlugin {
    fn install(&self, env: &mut Environment) {
        env.insert(self.theme.clone());
        env.insert_hook(|env, config: ButtonConfig| {
            // Custom button styling
        });
    }
}

// Usage
let mut env = Environment::new();
env.install(MyLibraryPlugin { theme: MyTheme::default() });
```

> **Tip:** The `Plugin` trait is the recommended way to distribute a library's setup logic. Instead of asking users to call five different `env.insert(...)` lines, give them a single `env.install(MyPlugin { ... })`.

## ViewExt Composition Patterns

WaterUI's modifier system uses extension traits. When creating library components,
prefer composition over wrapping:

```rust,ignore
// Prefer: compose with existing modifiers
pub fn card(content: impl View) -> impl View {
    content
        .padding(EdgeInsets::all(16.0))
        .background(Color::surface())
        .corner_radius(12.0)
        .shadow(Shadow::default())
}

// Avoid: creating a new view type just for styling
pub struct Card<V> { content: V }
impl<V: View> View for Card<V> {
    fn body(self, env: &Environment) -> impl View {
        self.content
            .padding(EdgeInsets::all(16.0))
            .background(Color::surface())
            // ... same thing but more code
    }
}
```

The function approach is simpler and composes naturally with the rest of the
framework.

### When to Create a Custom View Type

Create a dedicated struct when:

- The component has internal state (use `Binding<T>`).
- It needs to participate in FFI (native rendering).
- It has multiple configuration options (use `configurable!`).
- It needs to intercept environment values.

## The Extractor Pattern

The `Extractor` trait lets views declare their dependencies declaratively:

```rust,ignore
use waterui_core::extract::Extractor;

// Use use_env with tuple extraction
let view = use_env(|(nav, db): (NavigationController, Database)| {
    // Both values extracted from environment
    button("Load").on_tap(move || {
        let data = db.fetch();
        nav.push(detail_view(data));
    })
});
```

Library views should use `use_env` to access environment values rather than
requiring users to pass them explicitly. This keeps APIs clean and enables
dependency injection.

## Testing Strategies

Good libraries are well-tested libraries. WaterUI supports several testing approaches.

### Unit Testing Views

Test view construction without rendering:

```rust,ignore
#[cfg(test)]
mod tests {
    use super::*;
    use waterui_core::Environment;

    #[test]
    fn button_config_has_correct_defaults() {
        let btn = button("Tap me", || {});
        let config = btn.config();
        assert_eq!(config.style, ButtonStyle::Default);
    }

    #[test]
    fn view_body_produces_expected_tree() {
        let env = Environment::new();
        let view = my_component();
        let body = view.body(&env);
        // Assert on the resulting view type
    }
}
```

### Testing Reactive Behavior

Test that signals propagate correctly:

```rust,ignore
#[test]
fn counter_increments() {
    let count = Binding::i32(0);
    let view = counter_view(count.clone());

    // Simulate action
    count.set(1);
    assert_eq!(count.get(), 1);
}
```

### Visual Testing with Preview

Use the `#[preview]` macro to render views to PNG for visual regression testing:

```rust,ignore
#[preview]
fn my_card_preview() -> impl View {
    card(text("Preview content"))
}
```

Then run:

```bash
water preview my_card_preview --platform macos --path ./app --output card.png
```

## Best Practices

### Prefer Composition Over Inheritance

Rust does not have inheritance, and WaterUI leans into this. Build complex
components by composing simple ones:

```rust,ignore
// Good: composition
pub fn labeled_field(label: &str, field: impl View) -> impl View {
    vstack((text(label).font(Font::caption()), field)).spacing(4.0)
}

// Bad: trying to "inherit" from TextField
pub struct LabeledTextField { /* reimplements TextField internals */ }
```

### Leverage the Type System

Use Rust's type system to enforce correctness at compile time:

```rust,ignore
// Good: type-safe builder
pub struct FormBuilder<S: FormState> {
    state: S,
    fields: Vec<AnyView>,
}

// Bad: stringly-typed API
pub fn add_field(form: &mut Form, name: &str, field_type: &str) { /* ... */ }
```

### Keep Views Stateless

Views should be lightweight, stateless descriptions. Put mutable state in
`Binding<T>` values that live outside the view tree:

```rust,ignore
// Good: state separate from view
pub fn counter() -> impl View {
    let count = Binding::i32(0);
    vstack((
        text(count.map(|c| format!("Count: {c}"))),
        button("+", {
            let count = count.clone();
            move || count.set(count.get() + 1)
        }),
    ))
}
```

### Document with Previews

Every public component should have a `#[preview]` function in its module:

```rust,ignore
#[preview]
fn button_styles() -> impl View {
    vstack((
        button("Default", || {}),
        button("Destructive", || {}).style(ButtonStyle::Destructive),
        button("Plain", || {}).style(ButtonStyle::Plain),
    ))
    .spacing(8.0)
}
```

This serves as both documentation and a visual test.

### Minimize Public API Surface

Export only what users need. Keep internal types private:

```rust,ignore
// lib.rs
pub use button::{button, Button, ButtonStyle};
// ButtonConfig, ButtonInner, etc. stay private
```

Use `#[doc(hidden)]` for types that must be public for technical reasons (macro
expansion) but should not appear in documentation.

## What's Next

You have now seen WaterUI from the inside out -- rendering, FFI, layout, backends, and library authoring. The [next chapter](../09-philosophy.md) steps back from the code to explore the design philosophy that ties all these pieces together.

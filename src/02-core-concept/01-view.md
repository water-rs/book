# Understanding Views

The View system is the heart of WaterUI. Everything you see on screen is a View, and understanding how Views work is crucial for building efficient and maintainable applications. In this chapter, we'll explore the View trait in depth and learn how to create custom components.

## What is a View?

A View in WaterUI represents a piece of user interface. It could be as simple as a text label or as complex as an entire application screen. The beauty of the View system is that simple and complex views work exactly the same way.

### The View Trait

Every View implements a single trait:

```rust
# use waterui::env::Environment;
pub trait View: 'static {
    fn body(self, env: &Environment) -> impl View;
}
```

This simple signature enables powerful composition patterns. Let's understand each part:

- **`'static` lifetime**: Views can't contain non-static references, ensuring they can be stored and moved safely
- **`self` parameter**: Views consume themselves when building their body, enabling zero-cost moves
- **`env: &Environment`**: Provides access to shared configuration and dependencies
- **`-> impl View`**: Returns any type that implements View, enabling flexible composition

## Building Views from Anything

The trait is deliberately broad. Anything that implements `View` (including functions and closures) can return any other `View` in its `body`. Two helper traits make this ergonomic:

- `IntoView`: implemented for every `View` plus tuples, so `vstack(("A", "B"))` works without wrapping strings manually.
- `TupleViews`: converts tuples/arrays into `Vec<AnyView>` so layout containers can iterate over heterogeneous children.

This is why simple function components are the preferred way to build UI—`fn header() -> impl View` automatically conforms to the trait.

## Built-in Views

WaterUI provides many built-in Views for common UI elements:

### Text Views
```rust
# use waterui::prelude::*;
# use waterui::layout::stack::vstack;
# use waterui::reactive::binding;
# use waterui::Binding;
pub fn text_examples() -> impl View {
    let name: Binding<String> = binding("Alice".to_string());
    vstack((
        // Static text
        "Hello, World!",
        // Reactive text
        text!("Hello, {name}!"),
        // Styled text
        waterui_text::Text::new("Important!").size(24.0),
    ))
}
```

### Control Views
```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# use waterui::layout::stack::vstack;
pub fn control_examples() -> impl View {
    let enabled = binding(false);
    vstack((
        button("Click me").action(|| println!("Clicked!")),
        toggle(text("Enable notifications"), &enabled),
    ))
}
```

### Layout Views
```rust
# use waterui::prelude::*;
# use waterui::layout::stack::{vstack, hstack, zstack};
pub fn layout_examples() -> impl View {
    vstack((
        vstack(("First", "Second", "Third")),
        hstack((button("Cancel"), button("OK"))),
        zstack((text("Base"), text("Overlay"))),
    ))
}
```

## Creating Custom Views

The real power of WaterUI comes from creating your own custom Views. Let's explore different patterns:

### Function Views (Recommended)

```rust
use waterui::prelude::*;

pub fn welcome_message(name: &str) -> impl View {
    vstack((
        waterui_text::Text::new("Welcome!").size(24.0),
        waterui_text::Text::new(format!("Hello, {}!", name)),
    ))
}

let lazy_view = || welcome_message("Bob");
```

Functions automatically satisfy `View`, so prefer them for stateless composition or whenever you can lean on existing bindings (as we did in `examples::counter_view` inside this book’s crate).

### Struct Views (For Components with State)

Only reach for a custom struct when the component needs to carry configuration while building its child tree or interact with the `Environment` directly:

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# use waterui::Binding;

pub struct CounterWidget {
    pub initial_value: i32,
    pub step: i32,
}

impl View for CounterWidget {
    fn body(self, _env: &Environment) -> impl View {
        let count = binding(self.initial_value);

        vstack((
            text!("Count: {count}"),
            button("+").action_with(&count, move |state: Binding<i32>| {
                state.set(state.get() + self.step);
            }),
        ))
    }
}
```

## Type Erasure with `AnyView`

When you need to store different view types in the same collection (navigation stacks, list diffing, etc.), use `AnyView`:

```rust
# use waterui::AnyView;
# fn welcome_message(name: &str) -> &'static str { "hi" }
let screens: Vec<AnyView> = vec![
    AnyView::new(welcome_message("Alice")),
    AnyView::new(welcome_message("Bob")),
];
```

`AnyView` erases the concrete type but keeps behaviour intact, letting routers or layout engines manipulate heterogeneous children uniformly.

## Configurable Views and Hooks

Many built-in controls implement `ConfigurableView`, exposing a configuration struct that can be modified globally through hooks:

```rust,no_run
# use waterui::prelude::*;
# use waterui::env::Environment;
# use waterui::AnyView;
# use waterui::component::button::ButtonConfig;
# use waterui::layout::stack::hstack;
# use waterui_text::Text;
# use waterui::view::ViewConfiguration;
pub fn install_button_theme(env: &mut Environment) {
    env.insert_hook(|_, mut config: ButtonConfig| {
        config.label = AnyView::new(hstack((
            Text::new("🌊"),
            config.label,
        )));
        config.render()
    });
}
```

Hooks intercept `ViewConfiguration` types before renderers see them, enabling cross-cutting features like theming, logging, and accessibility instrumentation. Plugins install these hooks automatically, so understanding `ConfigurableView` prepares you for the advanced chapters on styling and resolver-driven behaviour.

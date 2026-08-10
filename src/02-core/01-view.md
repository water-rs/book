# The view system

> **In this chapter, you will:**
> - Read the `View` trait and understand why `body` consumes `self`
> - Build views as plain functions and as structs
> - Know which standard Rust types are already views
> - Tell raw (leaf) views apart from composite views, and know when `AnyView` is worth its cost

Every piece of UI in WaterUI -- a label, a button, a card, a whole page -- is a `View`. A view is a description, not a widget: you build a value that says what the screen should contain, and the backend turns it into native widgets.

## The View trait

```rust,ignore
pub trait View: 'static {
    fn body(self, env: &Environment) -> impl View;
}
```

`body` consumes the view and returns another view. The framework calls it recursively until it reaches a **raw view** -- a leaf the backend knows how to render, such as `Str`, `Color`, or `ButtonConfig`.

Three consequences follow from that signature:

- **`self` by value.** Views are cheap descriptors, created and consumed once. There is no persistent widget object to mutate.
- **`&Environment`.** Every view receives the ambient context: theme tokens, locale, injected services. See [the Environment chapter](03-environment.md).
- **`'static`.** A view owns its data and cannot hold borrowed references. Share mutable data through a `Binding` instead.

## Function views

Any `FnOnce() -> V where V: View` is itself a view, so the shortest component is a function:

```rust,ignore
use waterui::prelude::*;

fn greeting() -> impl View {
    "Hello, World!" // &'static str is a View
}
```

Function views compose naturally and need no boilerplate:

```rust,ignore
use waterui::prelude::*;

fn counter(count: Binding<i32>) -> impl View {
    vstack((
        text!("Count: {count}"),
        button("Increment")
            .action(|State(count): State<Binding<i32>>| *count.get_mut() += 1)
            .state(&count),
    ))
}
```

Two things are happening here. `text!("Count: {count}")` captures the `count` binding by name and re-renders only that label when the value changes -- no `watch`, no manual subscription. And `.state(&count)` injects the binding into the button's environment so the handler can pull it back out with the `State<T>` extractor; `.get_mut()` returns a guard that writes back on drop, which is the idiomatic way to mutate a binding.

Start with function views. Most components never need to be anything else.

## Struct views

Reach for a struct when a component has several named parameters or wants builder methods:

```rust,ignore
use waterui::prelude::*;
use waterui::widget::condition::when;

struct ProfileCard {
    name: Binding<String>,
    bio: Binding<String>,
    show_bio: bool,
}

impl View for ProfileCard {
    fn body(self, env: &Environment) -> impl View {
        let Self { name, bio, show_bio } = self;
        vstack((
            text!("{name}").bold(),
            when(show_bio, || text!("{bio}")),
        ))
    }
}
```

Destructuring `self` up front is the usual first line: `body` takes ownership, so you may as well move the fields out.

## Types that are already views

You do not have to wrap everything:

| Type | Behavior |
|------|----------|
| `()` | Renders nothing. A raw view, useful as a placeholder. |
| `&'static str`, `String`, `Cow<'static, str>` | Convert to `Str` and render as text. |
| `Option<V: View>` | Renders the inner view, or nothing for `None`. |
| `Result<V: View, E: View>` | Renders whichever side is present. |
| `(V,)` | A one-element tuple renders its content. |
| `FnOnce() -> V` | Calls the closure and renders the result. |

`Option<V>` is the cheapest conditional. For if/else-if/else, use `when(...).or(...).otherwise(...)` from `waterui::widget::condition` rather than branching into `AnyView`.

Note that a *signal* is not a view: `Computed<T>` does not implement `View`. Feed reactive values into signal-aware inputs (`text!`, `.opacity(...)`, `.background(...)`) instead of trying to render a signal directly.

## Passing several children

Layout containers do not take one child, they take a `TupleViews`:

```rust,ignore
pub trait TupleViews {
    fn into_views(self) -> Vec<AnyView>;
}
```

It is implemented for tuples up to 15 elements, and for `Vec<V>` and `[V; N]`:

```rust,ignore
use waterui::prelude::*;

// Heterogeneous: every element may be a different type
vstack((
    text!("Title"),
    button("Click me").action(|| {}),
    Color::red().height(2.0),
));

// Homogeneous: one element type, so erase to AnyView if the types differ
let rows: Vec<_> = (0..5).map(|i| text!("Row {i}").anyview()).collect();
vstack(rows);
```

For a collection whose *membership* changes at runtime, neither of these is right -- use `ForEach` or `List` so the framework can diff by identity. That is covered in [Reactive state](02-reactive.md).

## `AnyView`: type erasure

Rust requires both arms of an `if` to have the same type. `AnyView` boxes a view so heterogeneous arms unify:

```rust,ignore
use waterui::prelude::*;

fn detail(show_detail: bool) -> AnyView {
    if show_detail {
        text!("Detailed information here").anyview()
    } else {
        text!("Summary").anyview()
    }
}
```

`AnyView::new` unwraps a nested `AnyView`, so erasing twice costs nothing extra. The wrapper also supports inspection, which backends and tests use:

```rust,ignore
use core::any::TypeId;
use waterui::prelude::*;
use waterui::text::Text;

let view = text("hello").anyview();

assert!(view.is::<Text>());
assert_eq!(view.type_id(), TypeId::of::<Text>());

if let Some(text_view) = view.downcast_ref::<Text>() {
    let _ = text_view;
}
```

Each `AnyView` is a heap allocation plus dynamic dispatch. Prefer `when(...).otherwise(...)`, which keeps the concrete types.

## Raw views and composite views

**Raw views** are leaves. Their `body()` wraps the value in `Native<T>`, which the renderer intercepts before recursing, and the backend maps them onto a platform widget. `Str`, `Color`, `Spacer`, `Divider`, and configuration structs such as `ButtonConfig` are raw views.

The `raw_view!` macro implements `NativeView` and `View` for a type, optionally declaring how it stretches:

```rust,ignore
raw_view!(MyCustomLeaf);                  // content-sized (StretchAxis::None)
raw_view!(Color, StretchAxis::Both);      // fills available space
raw_view!(Spacer, StretchAxis::MainAxis); // fills along the stack axis
```

**Composite views** are everything else: their `body()` returns other views, and the framework expands them until only raw views remain. Every function view and every hand-written `impl View` is composite.

If it helps, think HTML: raw views are `<input>` and `<img>`, composite views are your own components.

## Hookable views

Some raw views can be restyled globally without touching their call sites. Such a view implements `ConfigurableView`, and its configuration implements `ViewConfiguration`:

```rust,ignore
pub trait ConfigurableView: View {
    type Config: ViewConfiguration;
    fn config(self) -> Self::Config;
}

pub trait ViewConfiguration: 'static {
    type View: View;
    fn render(self) -> Self::View;
}
```

When such a view's `body()` runs, it extracts its `Config`, looks for `Hook<Config>` in the environment, and hands the configuration to the hook if one is installed; otherwise it falls through to the default native rendering. A theme is exactly a bundle of hooks for `ButtonConfig`, `ToggleConfig`, and friends -- see [Hooks](03-environment.md#hooks-intercepting-view-configuration).

The `configurable!` macro writes that boilerplate:

```rust,ignore
// Content-sized
configurable!(Button, ButtonConfig);

// Explicit stretch axis
configurable!(Slider, SliderConfig, StretchAxis::Horizontal);

// Stretch axis derived from the configuration
configurable!(Progress, ProgressConfig, |config| match config.style {
    ProgressStyle::Linear => StretchAxis::Horizontal,
    ProgressStyle::Circular => StretchAxis::None,
});

// Resolve the configuration against the environment before it reaches the backend
configurable!(Toggle, ToggleConfig, StretchAxis::Horizontal, resolve |config, env| config.resolve(env));
```

That last form is how a control folds environment state -- an enclosing `.disabled(...)` scope, for instance -- into the configuration the backend receives.

You will rarely call `configurable!` in application code; it is for component libraries and backends.

## Putting it together

```rust,ignore
use waterui::prelude::*;
use waterui::widget::condition::when;

fn header(title: &'static str) -> impl View {
    text(title)
        .padding()
        .background(Color::blue())
        .foreground(Color::srgb(255, 255, 255))
}

struct ItemRow {
    label: Str,
    count: Binding<i32>,
    highlighted: bool,
}

impl View for ItemRow {
    fn body(self, env: &Environment) -> impl View {
        let Self { label, count, highlighted } = self;
        hstack((
            text(label),
            Spacer::flexible(),
            text!("{count}"),
        ))
        .padding()
        .background(when(highlighted, || Color::yellow().with_opacity(0.3)))
    }
}

fn shopping_list() -> impl View {
    vstack((
        header("Shopping List"),
        ItemRow { label: "Apples".into(),  count: Binding::i32(3), highlighted: true  },
        ItemRow { label: "Bananas".into(), count: Binding::i32(7), highlighted: false },
    ))
}
```

Add an "Oranges" row and the layout absorbs it with no other change.

Next: [Reactive state](02-reactive.md), where `Binding` and `Computed` make those counts change on screen.

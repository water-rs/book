# The view system

> **In this chapter, you will:**
> - Understand the `View` trait and how WaterUI builds UIs from composable pieces
> - Learn to create views using functions, structs, and built-in types
> - Discover how `AnyView` solves Rust's type system challenges for dynamic UIs
> - See how raw views and composite views work together to form the rendering tree

Every piece of UI you see on screen -- a text label, a button, a card, an entire page -- is a `View` in WaterUI. Views are composable, declarative descriptions of what the screen should look like. You describe *what* you want, and the framework figures out *how* to render it.

If you have used SwiftUI or Jetpack Compose, this will feel familiar. If not, do not worry -- the concept is straightforward, and this chapter will walk you through it from the ground up.

## The View trait

At the heart of WaterUI lies a single trait:

```rust
pub trait View: 'static {
    fn body(self, env: &Environment) -> impl View;
}
```

A `View` consumes itself and, given an `Environment`, produces another `View`. The framework calls `body()` recursively until it reaches a **raw view** -- a leaf node that the native backend knows how to render (such as `Text`, `Button`, or `Color`).

Key properties:

- **Consuming**: `body` takes `self` by value. Views are cheap descriptors, created and consumed during rendering.
- **Contextual**: The `Environment` carries dependency-injected values such as theme tokens, locale, and your own configuration.
- **Recursive**: Composite views return other views, which themselves have bodies. The recursion terminates at raw views.
- **`'static` bound**: Views own all their data. No borrowed references, which keeps the lifecycle simple.

> **Note:** `'static` does not mean "lives forever". It means views cannot hold temporary references. Wrap shared mutable data in a `Binding`.

## Function views

The simplest way to create a view is with a plain function. Any `FnOnce() -> V` where `V: View` automatically implements `View`:

```rust
use waterui::prelude::*;

fn greeting() -> impl View {
    "Hello, World!" // &'static str implements View
}
```

Function views are the recommended starting point. They compose naturally and work with Rust's type inference:

```rust
use waterui::prelude::*;
use waterui::widget::condition::when;

fn counter(count: Binding<i32>) -> impl View {
    vstack((
        text!("Count: {count}"),
        button("Increment")
            .action(|State(count): State<Binding<i32>>| count.set(count.get() + 1))
            .state(&count),
    ))
}
```

`text!("Count: {count}")` captures the `count` binding from scope and rebuilds the rendered text whenever it changes. There is no need to wrap text construction in `Dynamic::watch` -- the macro already takes care of subscribing to the signal.

> **Tip:** Start with function views. Most components never need to become structs.

## Struct views

When a component needs named configuration fields or builder-pattern ergonomics, define it as a struct:

```rust
use waterui::prelude::*;
use waterui::widget::condition::when;

struct ProfileCard {
    name: Binding<String>,
    avatar_url: Str,
    show_bio: bool,
}

impl View for ProfileCard {
    fn body(self, env: &Environment) -> impl View {
        let Self { name, avatar_url, show_bio } = self;
        vstack((
            text!("{name}"),
            when(show_bio, || text!("Bio goes here")),
        ))
    }
}
```

Struct views shine when:

- The component has multiple configuration parameters.
- You want a clear, self-documenting API surface.
- The component is reused across many call sites with varying configurations.

## Built-in view implementations

You do not always need to define your own views. Several standard types implement `View` directly:

| Type | Behavior |
|------|----------|
| `()` | Empty view (renders nothing). Useful as a placeholder. |
| `&'static str`, `String`, `Cow<'static, str>` | Render as text via `Str`. |
| `Option<V: View>` | Renders the inner view if `Some`, nothing if `None`. |
| `Result<V: View, E: View>` | Renders the `Ok` or `Err` view. |
| `(V,)` | Single-element tuple renders the contained view. |
| `FnOnce() -> V` | Calls the closure and renders the returned view. |
| `Computed<V: View>` | Re-renders whenever the computed signal emits. |

> **Tip:** `Option<V>` is the simplest way to conditionally render. For full if/elif/else, use `when(...).or(...).otherwise(...)` from `waterui::widget::condition` instead of branching to `AnyView`.

## The `IntoView` trait

`IntoView` converts arbitrary types into views within a given environment:

```rust
pub trait IntoView {
    type Output: View;
    fn into_view(self, env: &Environment) -> Self::Output;
}
```

Every `View` automatically implements `IntoView` (returning itself). The trait is useful for APIs that want to accept "anything that can become a view" while still allowing environment-aware conversions.

## The `TupleViews` trait

When you build layouts, you often want to pass multiple children of different types to a container. `TupleViews` converts tuples of views (and `Vec<V>` / `[V; N]`) into a `Vec<AnyView>`:

```rust
pub trait TupleViews {
    fn into_views(self) -> Vec<AnyView>;
}
```

It is implemented for tuples up to 15 elements:

```rust
use waterui::prelude::*;

vstack((
    text!("Title"),
    button("Click me").action(|| {}),
    Color::red().height(2.0),
))
```

Layout containers also accept `Vec<V>` and `[V; N]` as children of a uniform type:

```rust
use waterui::prelude::*;

let items: Vec<_> = (0..5).map(|i| text!("Row {i}").anyview()).collect();
vstack(items)
```

> **Note:** Tuples allow heterogeneous children (each element can be a different type). `Vec` and arrays require a single element type, so erase to `AnyView` if needed.

## `AnyView`: type erasure

Rust requires every branch of an `if`/`match` to return the same type. `AnyView` erases the concrete type so heterogeneous branches can share a return type:

```rust
pub struct AnyView(Box<dyn AnyViewImpl>);
```

Create one with `AnyView::new` or the `.anyview()` modifier from `ViewExt`:

```rust
use waterui::prelude::*;

fn conditional_view(show_detail: bool) -> AnyView {
    if show_detail {
        text!("Detailed information here").anyview()
    } else {
        text!("Summary").anyview()
    }
}
```

`AnyView::new` automatically unwraps a nested `AnyView`, so wrapping is idempotent. It also supports inspection and downcasting:

```rust
use core::any::TypeId;
use waterui::prelude::*;

let view = text("hello").anyview();

assert!(view.is::<waterui::text::Text>());
assert_eq!(view.type_id(), TypeId::of::<waterui::text::Text>());

if let Some(text_view) = view.downcast_ref::<waterui::text::Text>() {
    let _ = text_view;
}
```

> **Tip:** Prefer `when(...).otherwise(...)` over `if/else` with `.anyview()`. `AnyView` incurs a heap allocation and dynamic dispatch -- reach for it only when you really do need heterogeneous storage.

## Raw views vs composite views

WaterUI distinguishes two categories of views.

### Raw views (leaf nodes)

Raw views are recognized by the backend and mapped to platform widgets. Their `body()` wraps the value in `Native<T>`, which the renderer intercepts before recursion. Examples: `Str`, `Color`, `Spacer`, `Divider`, and configuration structs like `ButtonConfig`.

The `raw_view!` macro implements both `NativeView` and `View` for a type:

```rust,ignore
// Default stretch axis (None) -- content-sized
raw_view!(MyCustomLeaf);

// Explicit stretch axis -- fills available space
raw_view!(Color, StretchAxis::Both);
raw_view!(Spacer, StretchAxis::MainAxis);
```

### Composite views

Composite views have a meaningful `body()` that returns other views. The framework calls `body()` to expand them, recursing until it reaches raw views. Every function view and every struct that implements `View` manually is composite.

> **Tip:** Think HTML: raw views are native elements (`<div>`, `<input>`, `<img>`), composite views are your custom components.

## `ConfigurableView` and `ViewConfiguration`

Some raw views support **hook-based theming** through `ConfigurableView` and `ViewConfiguration`. This is how WaterUI lets you restyle built-in components without modifying their source.

```rust
pub trait ConfigurableView: View {
    type Config: ViewConfiguration;
    fn config(self) -> Self::Config;
}

pub trait ViewConfiguration: 'static {
    type View: View;
    fn render(self) -> Self::View;
}
```

When a configurable view's `body()` runs:

1. It extracts its `Config`.
2. It looks up `Hook<Config>` in the `Environment`.
3. If a hook is present, the hook returns the custom view.
4. Otherwise the default native rendering is used.

A theme plugin installs hooks for `ButtonConfig`, `ToggleConfig`, etc., and the rest of your app stays untouched.

## The `configurable!` macro

`configurable!` generates the boilerplate for a hookable raw view:

```rust,ignore
// Basic -- content-sized view
configurable!(Button, ButtonConfig);

// With explicit stretch axis
configurable!(Slider, SliderConfig, StretchAxis::Horizontal);

// With dynamic stretch axis based on configuration
configurable!(Progress, ProgressConfig, |config| match config.style {
    ProgressStyle::Linear => StretchAxis::Horizontal,
    ProgressStyle::Circular => StretchAxis::None,
});
```

It generates the wrapper struct, the `ConfigurableView` and `ViewConfiguration` impls, the `NativeView` impl, the `View` impl that consults `Hook<Config>`, and the `From<Config>` conversion.

> **Note:** You will rarely call `configurable!` in application code. It is primarily for building component libraries or custom backends.

## Putting it together

Here is a small example combining function views, struct views, conditionals, and reactive state:

```rust
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
    let apples = Binding::i32(3);
    let bananas = Binding::i32(7);

    vstack((
        header("Shopping List"),
        ItemRow { label: "Apples".into(),  count: apples,  highlighted: true  },
        ItemRow { label: "Bananas".into(), count: bananas, highlighted: false },
    ))
}
```

Try adding an "Oranges" row and watch the layout pick it up automatically.

Next up: reactive state. The next chapter introduces `Binding`, `Computed`, and the signal combinators that drive UI updates.

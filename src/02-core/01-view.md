# The view system

> **In this chapter, you will:**
> - Read the `View` trait and understand why `body` consumes `self`
> - Build views as plain functions and as structs
> - Pass text into views with `&'static str`, `Str`, and `impl IntoText`
> - Tell raw (leaf) views apart from composite views, and know when `AnyView` is worth its cost
> - Know what `raw_view!`, `configurable!`, and `Hook` are for

Every piece of UI in WaterUI -- a label, a button, a card, a whole page -- is a `View`. A view is a description, not a widget: you build a value that says what the screen should contain, and the backend turns it into native widgets.

## The View trait

```rust,ignore
pub trait View: 'static {
    fn body(self, env: &Environment) -> impl View;
}
```

`body` consumes the view and returns another view. The framework calls it recursively until it reaches a **raw view** -- a leaf the backend knows how to render, such as `Str` or `Color`.

(The trait has two more provided methods, `transition` and a hidden `stretch_axis`; you only implement `body`.)

Three consequences follow from that signature:

- **`self` by value.** Views are cheap descriptors, created and consumed once. There is no persistent widget object to mutate.
- **`&Environment`.** Every view receives the ambient context: theme tokens, locale, injected services. See [the Environment chapter](03-environment.md).
- **`'static`.** A view owns its data and cannot hold borrowed references. Share mutable data through a `Binding` instead.

## Function views

The simplest component is a function that returns `impl View`. (A `'static` closure `FnOnce() -> V` with `V: View` is itself a view too; the framework calls it and renders the result.)

```rust
use waterui::prelude::*;

fn greeting() -> impl View {
    "Hello, World!" // &'static str is a View
}
```

Function views compose naturally and need no boilerplate:

```rust
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

## Text parameters: `&'static str`, `Str`, `impl IntoText`

Views are `'static`, so a component cannot take a borrowed `&str` and keep it. Pick one of three parameter types instead:

| Parameter type | Use it when |
|----------------|-------------|
| `&'static str` | The caller always passes a literal. |
| `Str` | The component stores the text. `Str` is WaterUI's string type: a static reference or a ref-counted buffer, cheap to clone, dereferences to `str`. It is in the prelude. |
| `impl IntoText` | The text may be a literal, a `String`, a `Str`, or a reactive `Binding` / `Computed` of one. This is what `text(...)` and the built-in constructors accept, and it is the friendliest choice for a public component. |

```rust
use waterui::prelude::*;
use waterui::text::IntoText;

fn card(title: impl IntoText, body: &'static str) -> impl View {
    vstack((text(title).bold(), body))
}

fn page(name: Binding<Str>) -> impl View {
    vstack((
        card("Static title", "A literal"),
        card(name, "A reactive title"),
    ))
}
```

`IntoText` lives in `waterui::text`; `waterui::prelude::*` does not export it by name, so a signature that names it needs `use waterui::text::IntoText;`.

The conversion depends on the type. A `&'static str` becomes localized text: it is a key looked up in the translation catalog, shown as-is when the catalog has no entry. A `String` or `Str` is verbatim and never consults the catalog. See [Internationalization](../06-advanced/06-i18n.md).

`&'static str`, `String`, `Cow<'static, str>` and `Str` are all views themselves: they render as a verbatim text leaf, so a bare literal is valid content in a stack. They are plain text, though, and a bare literal skips the translation catalog. Reach for `text(...)` or `text!(...)` when you want a localized, styled or reactive label.

## Borrowed helpers: `impl View + use<>`

A helper that only *reads* a reference while it builds the view does not need to own it. Since the returned view must be `'static`, tell the compiler the return type does not capture the borrow with a precise-capturing bound:

```rust
use waterui::prelude::*;

struct Item {
    title: Str,
}

fn item_row(item: &Item) -> impl View + use<> {
    text(item.title.clone()).padding()
}

fn items(list: &[Item]) -> impl View + use<> {
    vstack(list.iter().map(item_row).collect::<Vec<_>>())
}
```

Without `+ use<>`, edition 2024 makes `impl View` capture the lifetime of `item`, and the result is no longer `'static`. Clone what the view keeps (`Str` clones are cheap) and let the borrow end inside the function.

## Struct views

Reach for a struct when a component has several named parameters, wants builder methods, or needs the environment:

```rust
use waterui::prelude::*;
use waterui::widget::condition::when;

struct ProfileCard {
    name: Binding<Str>,
    bio: Binding<Str>,
    show_bio: bool,
}

impl View for ProfileCard {
    fn body(self, _env: &Environment) -> impl View {
        let Self { name, bio, show_bio } = self;
        vstack((
            text!("{name}").bold(),
            when(show_bio, move || text!("{bio}")),
        ))
    }
}
```

Destructuring `self` up front is the usual first line: `body` takes ownership, so you may as well move the fields out. The `when` branch is a `'static` closure, so it takes `bio` with `move`.

## Types that are already views

You do not have to wrap everything:

| Type | Behavior |
|------|----------|
| `()` | Renders nothing. A raw view, useful as a placeholder. |
| `&'static str`, `String`, `Cow<'static, str>` | Convert to `Str` and render as verbatim text. |
| `Str` | A raw view: the text leaf itself. |
| `Option<V: View>` | Renders the inner view, or nothing for `None`. |
| `Result<V: View, E: View>` | Renders whichever side is present. |
| `(V,)` | A one-element tuple renders its content. |
| `FnOnce() -> V` (`'static`) | Calls the closure and renders the result. |

`Option<V>` is the simplest conditional, decided once when the view is built. When the condition is reactive -- a `Binding<bool>` or `Computed<bool>` -- use `when(...)`, chained with `.or(...)` and `.otherwise(...)`, from `waterui::widget::condition`: it switches branches whenever the signal changes.

Note that a *signal* is not a view: neither `Binding<T>` nor `Computed<T>` implements `View`. Feed reactive values into signal-aware inputs (`text!`, `.opacity(...)`, `.visible(...)`, `when(...)`) instead of trying to render a signal directly.

## Passing several children

Layout containers do not take one child, they take a `TupleViews`:

```rust,ignore
pub trait TupleViews {
    fn into_views(self) -> Vec<AnyView>;
    fn stretch_axes(&self) -> Vec<StretchAxis>;
}
```

It is implemented for tuples (every element a different type allowed), and for `Vec<V>` and `[V; N]`:

```rust
use waterui::prelude::*;

fn rows() -> impl View {
    vstack((
        // Heterogeneous: every element may be a different type
        text!("Title"),
        button("Click me").action(|| {}),
        // Homogeneous: every element has one type (erase to AnyView if they differ)
        vstack((0..5).map(|i| text!("Row {i}")).collect::<Vec<_>>()),
    ))
}
```

For a collection whose *membership* changes at runtime, neither of these is right -- use `ForEach` or `List` so the framework can diff by identity. That is covered in [Reactive state](02-reactive.md) and [Lists](../03-ui/05-lists.md).

## `AnyView`: type erasure

Rust requires both arms of an `if` to have the same type. `AnyView` boxes a view so heterogeneous arms unify:

```rust
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

```rust
use core::any::TypeId;
use waterui::prelude::*;
use waterui::text::Text;

fn inspect() {
    let view = text("hello").anyview();

    assert!(view.is::<Text>());
    assert_eq!(view.type_id(), TypeId::of::<Text>());

    if let Some(text_view) = view.downcast_ref::<Text>() {
        let _ = text_view;
    }
}
```

Each `AnyView` is a heap allocation plus dynamic dispatch. An `if` like this runs once, when `body` runs, so it suits a plain `bool`. When the condition is a signal, use `when(condition, ...).otherwise(...)` instead: it re-renders the branch when the signal changes, with no `watch`.

## Raw views and composite views

**Raw views** are leaves. Their `body()` returns `Native<T>`, a marker the renderer intercepts before recursing. The backend identifies the leaf by its Rust type and builds the platform view from it, entirely in Rust: the Apple backend drives AppKit and UIKit, Hydrolysis draws the leaf itself. `Str`, `Color`, and `Spacer` are raw views. A configurable control such as `Button` ends the same way: unless a hook takes over (see below), its `body()` produces a `Native<ButtonConfig>` (behind an `AnyView`), so the configuration struct is the leaf the backend receives.

`Native<T>` is not something you render yourself. If a backend does not realize `T` and no fallback was attached with `Native::with_fallback`, rendering it is a contract violation: a debug build panics, a release build logs the error and renders nothing.

The `raw_view!` macro (`waterui::raw_view!`) implements `NativeView` and `View` for a type, optionally declaring how it stretches:

```rust,ignore
raw_view!(MyCustomLeaf);                  // content-sized (StretchAxis::None)
raw_view!(Color, StretchAxis::Both);      // fills available space
raw_view!(Spacer, StretchAxis::MainAxis); // fills along the stack axis
```

A raw view only renders where a backend has a realization for it. Defining a new one in application code gives you a leaf nobody draws; that macro is for the framework, its backends, and component libraries. See [Backends](../08-internals/04-backends.md) and [Library authoring](../08-internals/05-library-authoring.md).

**Composite views** are everything else: their `body()` returns other views, and the framework expands them until only raw views remain. Every function view and every hand-written `impl View` is composite.

If it helps, think HTML: raw views are `<input>` and `<img>`, composite views are your own components.

## Hookable views

Some built-in controls can be re-rendered for a whole subtree without touching their call sites. Such a view implements `ConfigurableView`, and its configuration implements `ViewConfiguration`; both traits, and `Hook`, live in `waterui::view`:

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

When such a view's `body()` runs, it extracts its `Config`, looks for a `Hook<Config>` in the environment, and hands the configuration to the hook if one is installed; otherwise it falls through to the default native rendering. That is how an app, or a library it links, replaces the rendering of every `Button` (through `Hook<ButtonConfig>`) in a subtree. A `Theme` is a different mechanism: it supplies colors and fonts, not hooks. See [Hooks](03-environment.md#hooks-intercepting-component-configuration).

A hook is built with `Hook::new(|env: &Environment, config| view)`, or installed directly with `Environment::insert_hook`:

```rust,ignore
env.insert_hook(|_env: &Environment, config: ButtonConfig| {
    // Return any view; `config.render()` yields the default button.
    config.render()
});
```

While a hook runs, its own entry is removed from the environment it receives, so `config.render()` and nested uses of the same view cannot recurse into it.

The `configurable!` macro writes the boilerplate for a view and its configuration: it defines the view struct wrapping the configuration, implements both traits, and makes `body()` apply a hook or fall through to `Native<Config>`. These are the forms the framework itself uses:

```rust,ignore
// Content-sized
configurable!(RawText, TextConfig);

// Stretch axis derived from the configuration
configurable!(Progress, ProgressConfig, |config| match config.style {
    ProgressStyle::Linear => StretchAxis::Horizontal,
    ProgressStyle::Circular | ProgressStyle::Loading => StretchAxis::None,
});

// Explicit stretch axis, with the configuration resolved against the environment
// before it reaches the backend
configurable!(Slider, SliderConfig, StretchAxis::Horizontal, resolve |config, env| config.resolve(env));

// Derived stretch axis plus a resolver
configurable!(
    Toggle,
    ToggleConfig,
    |config| config.layout_stretch_axis(),
    resolve |config, env| config.resolve(env)
);
```

The `resolve` forms are how a control folds environment state, such as the label display mode an enclosing scope sets, into the configuration the backend receives; a hook still receives the unresolved configuration. The macro also accepts `resolve` with the default axis, and leading attributes such as doc comments on the generated struct. A view that is generic, like `Button<Action>`, implements `ConfigurableView` by hand instead.

`configurable!` is exported by `waterui-core`, not the `waterui` facade: it is for component libraries and backends, and you will rarely need it in application code.

## Putting it together

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
    fn body(self, _env: &Environment) -> impl View {
        let Self { label, count, highlighted } = self;
        hstack((
            text(label),
            spacer(),
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

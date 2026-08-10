# The Environment

> **In this chapter, you will:**
> - Store and read values in WaterUI's type-indexed dependency injection container
> - Reach shared configuration from any view with `useenv` and extractors
> - Scope values to a subtree with `.with()` and `.install()`
> - Replace the rendering of built-in components globally with hooks and plugins

A deeply nested button needs the current theme. A form field needs the locale. A detail screen needs the API client. Threading all of that through every view function's parameter list does not scale.

`Environment` is the alternative: a type-indexed container that flows down the view tree automatically. Every view receives one in `body()`, every view can read from it, and any view can extend it for its own descendants. If you know React Context or SwiftUI's `@Environment`, this is the same idea with Rust's type system as the key.

## How it works

Each type can hold at most one visible value, so there are no string keys and no registration step -- the type *is* the key. Internally the container is a structurally shared overlay chain, which makes cloning an `Rc` bump and extending an `O(1)` overlay rather than a map copy.

Inserting the same type twice replaces the earlier value. If you genuinely need two values of one type, see [`Store`](#namespaced-keys-with-store).

## Seeding an environment

```rust,ignore
use waterui::prelude::*;

let mut env = Environment::new();

// Imperative
env.insert(String::from("hello"));
env.insert(42i32);

// with() mutates in place and returns &mut Self, so it chains
env.with(String::from("hello"))
   .with(42i32);
```

`env.extending(value)` is the non-mutating variant: it returns a fresh `Environment` overlaying the new value on the original, leaving the original untouched.

> **Note:** a bare `Environment::new()` has no theme installed. Theme tokens fail fast rather than falling back -- resolving a color slot that was never installed panics with `WaterUI color token ... is not installed in the environment`. Backends and `Theme::install` set these up; if you build an environment by hand to render themed views, install a theme first.

### Namespaced keys with `Store`

`Store<K, V>` pairs a value with a zero-sized marker type so the same `V` can appear in several roles. Unlike `with`, `store` consumes the environment and returns it:

```rust,ignore
use waterui::env::Store;
use waterui::prelude::*;

struct PrimaryColor;
struct AccentColor;

let env = Environment::new()
    .store::<PrimaryColor, _>(Color::blue())
    .store::<AccentColor, _>(Color::orange());

let primary: Option<&Color> = env.query::<PrimaryColor, Color>();
let accent: Option<&Color> = env.query::<AccentColor, Color>();
```

### Reading and removing

```rust,ignore
if let Some(theme) = env.get::<MyTheme>() {
    // &MyTheme
}

let config = env.get_or_insert_with(|| AppConfig::default());

env.remove::<MyTheme>();
```

## Reaching the environment from a view

### Extractors

A type is readable from a view once it implements `Extractor`. The `impl_extractor!` macro writes that impl for any type stored directly in the environment:

```rust,ignore
use waterui::impl_extractor;

#[derive(Clone, Debug)]
struct AppConfig {
    base_url: String,
    timeout_ms: u32,
}

impl_extractor!(AppConfig);
```

Several extractors exist already:

| Type | Behavior |
|------|----------|
| `Environment` | Clones the whole environment |
| `Option<T: Extractor>` | Extraction failure becomes `None` instead of an error |
| `State<T>` | Pulls state injected with `ViewExt::state`, for action handlers |
| `(A, B, ...)` | Extracts each element, up to 8-tuples |

### `useenv`

`useenv` builds a view from extracted values:

```rust,ignore
use waterui::env::useenv;
use waterui::prelude::*;

let view = useenv(|config: AppConfig| {
    let base_url = config.base_url.clone();
    text!("API: {base_url}")
});
```

Extraction is fast-fail: if the value is missing, the view panics with a message naming the type. Wrap the parameter in `Option` when absence is legitimate:

```rust,ignore
let view = useenv(|config: Option<AppConfig>| {
    match config {
        Some(config) => {
            let base_url = config.base_url.clone();
            text!("API: {base_url}").anyview()
        }
        None => text("Not configured").anyview(),
    }
});
```

Extract several values with a tuple:

```rust,ignore
let view = useenv(|(nav, db): (NavigationController, Database)| {
    let name = db.name();
    text!("Connected to {name}")
});
```

### Scoping a value to a subtree

`ViewExt::with` injects a value for a view and everything below it:

```rust,ignore
settings_form().with(MyConfig { debug: true })
```

That is how you give the settings page a different theme without touching the rest of the app. `ViewExt::install` does the same for a plugin:

```rust,ignore
settings_form().install(HighContrastPlugin)
```

## Keeping guards alive with `retain`

Anything RAII-scoped -- a signal watcher guard, a subscription, a task handle -- dies at the end of the `body()` that created it unless you tie it to the view:

```rust,ignore
fn my_view(data: Binding<String>) -> impl View {
    let guard = data.watch(|ctx| {
        tracing::debug!("Data changed: {}", ctx.into_value());
    });

    text!("Watching {data}").retain(guard)
}
```

Retain several values by chaining `.retain(a).retain(b)` or by passing a tuple. If a side effect "does not work", check that its guard is retained -- an unretained watcher unsubscribes the moment the body returns.

## Hooks: intercepting view configuration

Hooks are how a theme replaces the rendering of every button in an app without editing a single call site. A `Hook<C>` is a function from an environment plus a view configuration to a view:

```rust,ignore
pub struct Hook<C>(Box<dyn Fn(&Environment, C) -> AnyView>);
```

Install one with `insert_hook`. `render()` comes from the `ViewConfiguration` trait, so bring it into scope:

```rust,ignore
use waterui::component::button::ButtonConfig;
use waterui::view::ViewConfiguration;

env.insert_hook(|env: &Environment, config: ButtonConfig| {
    config.render()
        .padding()
        .background(Color::blue())
});
```

When a configurable view's `body()` runs it extracts its `Config`, looks up `Hook<Config>` in the environment, and calls the hook if one is present; otherwise the default native rendering is used. See [`ConfigurableView`](01-view.md#hookable-views) for the view side of that contract.

The hook is invoked with the hook itself *removed* from the environment. That is deliberate: it means `config.render()` inside a hook produces the default rendering instead of recursing forever, so a hook can wrap the platform control rather than having to replace it.

## Plugins

A `Plugin` bundles related setup -- values, hooks, nested plugins -- behind one call:

```rust,ignore
pub trait Plugin: Sized + 'static {
    fn install(self, env: &mut Environment) {
        env.insert(self);
    }

    fn uninstall(self, env: &mut Environment) {
        env.remove::<Self>();
    }
}
```

The default `install` just stores the plugin. Override it to do real work:

```rust,ignore
use waterui::component::button::ButtonConfig;
use waterui::prelude::*;
use waterui::shape::RoundedRectangle;
use waterui::view::ViewConfiguration;

struct RoundedButtonPlugin;

impl Plugin for RoundedButtonPlugin {
    fn install(self, env: &mut Environment) {
        env.insert(self);
        env.insert_hook(|env: &Environment, config: ButtonConfig| {
            config.render()
                .padding()
                .background(Color::blue())
                .clip(RoundedRectangle::new(0.2))
        });
    }
}
```

`RoundedRectangle::new` takes a *normalized* corner radius in `0.0..=0.5`, not points, so `0.2` looks the same on a small button and a large one.

Install at the root for the whole app, or on a subtree for part of it:

```rust,ignore
// App-wide
pub fn app(mut env: Environment) -> App {
    env.install(RoundedButtonPlugin);
    App::new(main, env)
}

// One screen only
fn my_screen() -> impl View {
    vstack((
        button("Save").action(|| {}),
        button("Cancel").action(|| {}),
    ))
    .install(RoundedButtonPlugin)
}
```

Install the plugin on one section and compare it with the rest of the app: only that subtree's buttons change.

## Worked example: a custom theme

```rust,ignore
use waterui::impl_extractor;
use waterui::env::useenv;
use waterui::prelude::*;

#[derive(Clone, Debug)]
struct AppTheme {
    primary: Color,
    background: Color,
    text: Color,
}

impl_extractor!(AppTheme);

impl AppTheme {
    fn light() -> Self {
        Self {
            primary: Color::blue(),
            background: Color::srgb(255, 255, 255),
            text: Color::srgb(0, 0, 0),
        }
    }

    fn dark() -> Self {
        Self {
            primary: Color::cyan(),
            background: Color::srgb(26, 26, 26),
            text: Color::srgb(255, 255, 255),
        }
    }
}

fn themed_card(title: &'static str) -> impl View {
    useenv(|theme: AppTheme| {
        text(title)
            .foreground(theme.text)
            .padding()
            .background(theme.background)
    })
}

fn app_root() -> impl View {
    vstack((
        themed_card("Welcome"),
        themed_card("Settings"),
    ))
    .with(AppTheme::light())
}
```

Each card re-extracts `AppTheme` from its own environment, so swapping the value at the root swaps every card.

To switch themes at runtime, scope the two variants behind a condition -- `when(dark_mode, || content().with(AppTheme::dark())).otherwise(|| content().with(AppTheme::light()))` -- so the framework rebuilds only that subtree when the flag flips. For colors that should update without any rebuild, use theme tokens instead: they are signals, and the backend tracks them per property.

## Metadata

Some rendering instructions ride along with a view rather than living in the environment. `Metadata<T>` is the mandatory form: a renderer that meets a `Metadata<T>` it does not understand panics, which is what keeps environment overrides and lifecycle hooks from being silently dropped. `IgnorableMetadata<T>` is the optional form, discarded by renderers that do not implement it -- accessibility hints use it.

You rarely construct either directly. The `ViewExt` modifiers do it for you: `.with(...)`, `.retain(...)`, `.on_appear(...)`, and `.shadow(...)` produce `Metadata`, while `.a11y_label(...)` and `.a11y_role(...)` produce `IgnorableMetadata`. The types themselves live in the internal `waterui_core` foundation crate; the facade exposes the modifiers, not the wrappers.

Next: [Modifiers and ViewExt](04-modifiers.md), the chainable methods that style, position, and add behavior to any view.

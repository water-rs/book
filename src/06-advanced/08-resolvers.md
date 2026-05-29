# Resolvers and hooks

> **In this chapter, you will:**
> - Understand how the `Resolvable` trait turns design tokens into reactive signals
> - Implement your own resolvable types for custom theming
> - Use `AnyResolvable<T>` for type-erased resolution
> - Transform resolved values with the `Map` combinator
> - Intercept view rendering with `Hook<C>` for cross-cutting concerns
> - Bridge reactive signals to views with `Dynamic::watch` and `watch`

You have built views, handled errors, localized text, and organized code with
plugins. There is a deeper pattern underneath all of it: **resolvers**. When
you write `.foreground(Accent)`, how does WaterUI know what color "Accent" is?
The answer is that `Accent` is a *token* — a lightweight value that knows how
to look itself up in the `Environment` at runtime. The lookup returns a
reactive signal, so when the OS toggles dark mode, every view that read that
token updates without rebuilding the tree.

This chapter covers the `Resolvable` trait, the `AnyResolvable<T>` type-erased
wrapper, the `Map` combinator, the `Hook<C>` system for intercepting view
configurations, and `Dynamic::watch` for bridging signals to views.

## The Resolvable trait

The core abstraction is `waterui_core::resolve::Resolvable`:

```rust,ignore
pub trait Resolvable: Debug + Clone {
    type Resolved;

    fn resolve(&self, env: &Environment) -> impl Signal<Output = Self::Resolved>;
}
```

A `Resolvable` does not hold its final value. It holds enough information to
**find** the value in an `Environment` and return a reactive signal.

### The flow

End-to-end, from the native platform all the way to your view:

```text
Native Backend         Environment              View
(iOS/Android)
     |                     |                      |
     | 1. Create reactive  |                      |
     |    Computed signal  |                      |
     |-------------------->|                      |
     |                     |                      |
     | 2. Install into env |                      |
     |    via Theme        |                      |
     |-------------------->|                      |
     |                     | 3. View resolves     |
     |                     |    Accent.resolve(env)
     |                     |<---------------------|
     |                     |                      |
     |                     | 4. Returns signal    |
     |                     |--------------------->|
     |                     |                      |
     | 5. System event     |                      |
     |    (dark mode)      |                      |
     |-------------------->| 6. Signal updates    |
     |                     |--------------------->|
     |                     |    View re-renders   |
```

### Why signals?

The key point is that `resolve()` returns an `impl Signal`, not a plain value.
This means:

1. **Native backends inject reactive signals.** The iOS or Android runtime
   pushes a `Computed<ResolvedColor>` that updates when the user toggles dark
   mode.
2. **Views automatically re-render.** When the signal updates, every view that
   read the resolved value updates without any manual code.
3. **No rebuild required.** Theme changes propagate instantly through the
   existing view tree.

## Implementing Resolvable

A token is typically a zero-sized type that knows where to look in the
environment. The lookup uses `Environment::query::<K, V>()` so the token type
itself can act as the phantom key:

```rust,ignore
use waterui_core::{Computed, Environment, Signal, resolve::Resolvable};

#[derive(Debug, Clone, Copy)]
pub struct BrandColor;

impl Resolvable for BrandColor {
    type Resolved = ResolvedColor;

    fn resolve(&self, env: &Environment) -> impl Signal<Output = Self::Resolved> {
        env.query::<Self, Computed<ResolvedColor>>()
            .cloned()
            .unwrap_or_else(|| Computed::constant(ResolvedColor::default()))
    }
}
```

`env.query::<Self, Computed<T>>()` reads the `Store<Self, Computed<T>>` slot.
The theme system installs these signals during environment setup (see
`waterui::theme::install_color_signal`).

### How themes use resolvers

The theme pipeline works like this:

1. The native backend creates `Computed<ResolvedColor>` signals from the
   system palette. These signals are reactive — they fire whenever the OS
   switches between light and dark mode.
2. `Theme::install` (a `Plugin`) stores those signals in the environment,
   keyed by token type (for example `color::Foreground`, `color::Accent`).
3. Token types implement `Resolvable` to query the environment for their
   signal.
4. When you write `text("Hello").foreground(Accent)`, the `Accent` token is
   resolved into a signal that the renderer subscribes to.

> **Note:** This is why theme changes feel instant — there is no rebuild step.
> The existing view tree simply reacts to the signal update.

## AnyResolvable\<T\>

Several resolvable types can produce the same output type. A `Color`, for
example, might come from a hex literal, a theme token, or a derived
expression. `AnyResolvable<T>` provides type erasure so all of them coexist
behind one interface:

```rust,ignore
use waterui_core::resolve::AnyResolvable;
use waterui::theme;

let from_hex = AnyResolvable::new(Color::srgb(255, 0, 0));
let from_token = AnyResolvable::new(theme::color::Accent);
```

`AnyResolvable<T>` itself implements `Resolvable<Resolved = T>`, so it can be
used wherever a `Resolvable` is expected. Internally it stores a
`Box<dyn ResolvableImpl<T>>` for dynamic dispatch.

### Constructing and resolving

```rust,ignore
pub fn new(value: impl Resolvable<Resolved = T> + 'static) -> Self;
pub fn resolve(&self, env: &Environment) -> Computed<T>;
```

`AnyResolvable::resolve` returns a concrete `Computed<T>` (not `impl Signal`),
which is the type you store, clone, and feed into other reactive APIs.

## The Map combinator

When you want a variation of an existing token — a lighter accent, a scaled
font size — `Map<R, F>` transforms a resolvable's output without losing
reactivity. WaterUI's own `Color::lighten` is implemented on top of `Map`:

```rust,ignore
use waterui_core::resolve::Map;
use waterui::theme::color::Accent;

let lighter_accent = Map::new(Accent, |color| color.lighten(0.2));
```

The closure runs lazily on each emission, so when the underlying `Accent`
signal updates, the derived signal emits a new lightened value automatically.

`Map` itself implements `Resolvable`:

```rust,ignore
impl<R, F, T, U> Resolvable for Map<R, F>
where
    R: Resolvable<Resolved = T>,
    F: Fn(T) -> U + Clone + 'static,
    T: 'static,
    U: 'static,
{
    type Resolved = U;

    fn resolve(&self, env: &Environment) -> impl Signal<Output = U> {
        let func = self.func.clone();
        self.resolvable.resolve(env).map(func)
    }
}
```

This composes with the standard signal `.map()` operator, so the derived
signal re-evaluates whenever the source changes.

## Hooks: intercepting view configuration

Resolvers handle *values* — colors, fonts, strings. Hooks handle *views*.
Some views implement `ConfigurableView`, which separates the view into a
*configuration* and a *renderer*:

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

A `ConfigurableView` can be intercepted by a `Hook<Config>` stored in the
environment. When the view body runs, it checks the environment for a
matching hook. If one exists, the hook receives the configuration, the
environment (with the hook removed to prevent recursion), and returns a
modified view. Otherwise, the configuration renders normally through
`config.render()`.

### The Hook type

```rust,ignore
pub struct Hook<C>(/* boxed Fn(&Environment, C) -> AnyView */);
```

A `Hook<C>` is a function from `(&Environment, C)` to `AnyView`, where `C` is
the view's `ViewConfiguration` type.

### Installing a hook

Use `Environment::insert_hook`:

```rust,ignore
use waterui::prelude::*;
use waterui::component::button::ButtonConfig;
use waterui::Environment;

let mut env = Environment::new();
env.insert_hook(|_env, config: ButtonConfig| {
    tracing::debug!(?config, "button rendered");
    config.render()
});
```

`insert_hook` wraps the closure in a `Hook<C>` and stores it under that
configuration's type. Before calling your closure, the framework removes the
hook from the cloned environment passed in, which is how recursion is
prevented when your closure ends with `config.render()`.

### How hooks execute

For a `ConfigurableView` body:

1. It produces its `Config`.
2. It checks the environment for `Hook<Config>`.
3. If a hook is present, the hook receives the config plus the environment
   (with this hook removed) and returns a view.
4. If no hook is present, the config renders normally via `config.render()`.

This mechanism enables powerful cross-cutting concerns:

- **Theming** — wrap every button with a consistent style.
- **A/B testing** — modify certain configurations based on experiment flags.
- **Logging** — record every view configuration for debugging.

### Hooks in plugins

A plugin's `install` body is the natural place to register a hook. Here is a
plugin that switches every button in its subtree to the bordered style:

```rust,ignore
use waterui::prelude::*;
use waterui::component::button::{ButtonConfig, ButtonStyle};
use waterui_core::{Environment, plugin::Plugin};

pub struct BorderedButtonsPlugin;

impl Plugin for BorderedButtonsPlugin {
    fn install(self, env: &mut Environment) {
        env.insert_hook(|_env, mut config: ButtonConfig| {
            config.style = ButtonStyle::Bordered;
            config.render()
        });
    }
}
```

> **Try it yourself:** Build a `LoggingPlugin` that installs hooks for several
> view configurations and logs each one with `tracing::debug!`.

## Dynamic views

The final piece is how a resolved signal becomes visible on screen. Most of
the time you do not write this glue yourself — a `Computed<V>` whose value
type is itself a `View` automatically renders through `Dynamic::watch`.
For the cases where you do need it, two entry points exist.

### Dynamic::watch

`Dynamic::watch` bridges any `Signal` to the view system. It calls your
closure on each emission and swaps the rendered subtree:

```rust,ignore
use waterui::prelude::*;
use waterui_core::dynamic::Dynamic;

let theme_name = Binding::container(String::from("Default"));

let view = Dynamic::watch(theme_name, |name: String| {
    let name = Binding::container(name);
    text!("Current theme: {name}")
});
```

Internally, `Dynamic::watch`:

1. Allocates a `Dynamic` view and its `DynamicHandler`.
2. Renders the initial value with `handler.set(f(value.get()))`.
3. Subscribes to the signal with `value.watch(...)`.
4. On each update, calls `handler.set(f(new_value))` to replace the subtree.
5. Retains the watcher guard and the source signal through `Metadata<Retain>`
   so they live as long as the view does.

### The Dynamic type

`Dynamic` is the low-level updatable view. You should prefer reactive
bindings, `text!`, and `Computed<V>` over raw `Dynamic` usage. When you do
need to swap an entire subtree on demand, `Dynamic::new` returns the handler
and the view together:

```rust,ignore
use waterui::prelude::*;
use waterui_core::dynamic::Dynamic;

let (handler, view) = Dynamic::new();

handler.set(text("Initial content"));

// Later, replace the content:
handler.set(text("Updated content"));
```

The handler is `Clone` and can be moved into closures or async tasks.

### Computed\<V\> as View

Any `Computed<V>` where `V: View` automatically implements `View`. The
implementation is just `Dynamic::watch(self, |view| view)`:

```rust,ignore
use waterui::prelude::*;
use waterui::AnyView;

let show_detail = Binding::bool(false);

let view = show_detail.map(|show| {
    if show {
        AnyView::new(text("Detail view"))
    } else {
        AnyView::new(text("Summary view"))
    }
});

// `view` is a Computed<AnyView> and renders as a View directly.
```

### The `watch` function

`watch` is a thin wrapper around `Dynamic::watch`:

```rust,ignore
use waterui::prelude::*;
use waterui_core::dynamic::watch;

let count = Binding::i32(0);

let view = watch(count, |n: i32| {
    let n = Binding::container(n);
    text!("Count: {n}")
});
```

## Putting it all together

A complete example combining a resolver, a plugin that installs it, and a
view that consumes it:

```rust,ignore
use waterui::prelude::*;
use waterui::app::App;
use waterui_core::{
    Computed,
    Environment,
    Signal,
    env::Store,
    plugin::Plugin,
    resolve::Resolvable,
};

// 1. Define a resolvable token.
#[derive(Debug, Clone, Copy)]
pub struct AppTitle;

impl Resolvable for AppTitle {
    type Resolved = String;

    fn resolve(&self, env: &Environment) -> impl Signal<Output = String> {
        env.query::<Self, Computed<String>>()
            .cloned()
            .unwrap_or_else(|| Computed::constant("My App".to_string()))
    }
}

// 2. Plugin that installs a signal under the AppTitle key.
pub struct AppTitlePlugin {
    title: String,
}

impl Plugin for AppTitlePlugin {
    fn install(self, env: &mut Environment) {
        env.insert(Store::<AppTitle, Computed<String>>::new(
            Computed::constant(self.title),
        ));
    }
}

// 3. Use the resolver in a view.
fn title_bar() -> impl View {
    use_env(|env: Environment| {
        let signal = AppTitle.resolve(&env);
        Dynamic::watch(signal, |title: String| {
            let title = Binding::container(title);
            text!("{title}").headline()
        })
    })
}

// 4. Wire it up.
pub fn app(env: Environment) -> App {
    let mut env = env;
    env.install(AppTitlePlugin {
        title: "WaterUI Tutorial".to_string(),
    });
    App::new(title_bar, env)
}
```

## Summary

| API | Purpose |
|---|---|
| `Resolvable` trait | Look up a value from the environment as a signal |
| `Resolvable::resolve(env)` | Returns `impl Signal<Output = Resolved>` |
| `AnyResolvable<T>` | Type-erased resolvable wrapper |
| `AnyResolvable::new(r)` | Wrap any resolvable |
| `AnyResolvable::resolve(env)` | Returns `Computed<T>` |
| `Map::new(r, f)` | Transform a resolvable's output |
| `ViewConfiguration` trait | View config that hooks can intercept |
| `Hook<C>` | Intercepts a view configuration |
| `Environment::insert_hook(f)` | Install a hook in the environment |
| `Dynamic::new()` | Low-level updatable view |
| `Dynamic::watch(signal, f)` | Bridge a signal to a subtree |
| `watch(signal, f)` | Convenience wrapper for `Dynamic::watch` |
| `Computed<V>: View` | Reactive view from computed signals |

## Next

You have reached the end of the advanced topics. From here you can revisit any
chapter to deepen your understanding, or build your own resolvable tokens and
plugins to bend WaterUI to your application's shape.

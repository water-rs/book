# The Environment

> **In this chapter, you will:**
> - Store and read values in WaterUI's type-indexed dependency injection container
> - Reach shared configuration from any view with `use_env`, extractors and action handlers
> - Scope a value or a plugin to one subtree with `.with()` and `.install()`
> - Override the app's theme tokens with `Theme` and `ColorSettings`
> - Replace how a built-in component renders with hooks and plugins

A deeply nested button needs the current theme. A form field needs the locale. A detail screen needs the API client. Threading all of that through every view function's parameter list does not scale.

`Environment` is the alternative: a type-indexed container that flows down the view tree automatically. Every view receives one in `body()`, every view can read from it, and any view can extend it for its own descendants. If you know React Context or SwiftUI's `@Environment`, this is the same idea with Rust's type as the key.

## How it works

Each type holds at most one visible value, so there are no string keys and no registration step: the type *is* the key. Inserting the same type twice replaces the earlier value; if one type genuinely needs several roles, see [`Store`](#several-values-of-one-type-with-store).

Internally the container is a structurally shared overlay chain. Cloning an `Environment` is an `Rc` bump and `extending` is an O(1) overlay rather than a map copy, which is why every view can afford to extend it. Because of the `Rc`, an `Environment` stays on the UI thread.

## Seeding an environment

The runtime hands your crate an `Environment` in `app(env)`. Put your services in it before building the `App`:

```rust
use waterui::app::App;
use waterui::prelude::*;

#[derive(Clone)]
struct ApiClient {
    base_url: String,
}

fn root() -> impl View {
    "Hello, WaterUI!"
}

pub fn app(mut env: Environment) -> App {
    env.insert(ApiClient {
        base_url: String::from("https://crates.io/api/v1"),
    });
    App::new(root, env)
}
```

`Environment` has two in-place writers and one non-mutating one:

```rust
use waterui::prelude::*;

fn seed() -> Environment {
    let mut env = Environment::new();

    env.insert(String::from("hello"));     // in place
    env.with(42_i32).with(true);           // in place, returns &mut Self so it chains

    // `extending` leaves `env` untouched and returns an overlay on top of it.
    let scoped = env.extending(3.5_f64);
    assert!(env.get::<f64>().is_none());
    assert_eq!(scoped.get::<f64>(), Some(&3.5));
    scoped
}
```

### Reading and removing

```rust
use waterui::prelude::*;

#[derive(Default)]
struct Settings {
    verbose: bool,
}

fn inspect(env: &mut Environment) {
    // `get` returns None when the type is absent.
    let _maybe: Option<&Settings> = env.get::<Settings>();

    // `get_or_insert_with` inserts the default on first use.
    let settings = env.get_or_insert_with(Settings::default);
    let _verbose = settings.verbose;

    // `remove` also hides a value inherited from a parent overlay.
    env.remove::<Settings>();
    assert!(env.get::<Settings>().is_none());
}
```

`get_nth(index)` counts visible values of one type from the nearest overlay outwards, and `get_nth(0)` always agrees with `get`. Action handlers use it to give repeated `State<T>` parameters positional meaning:

```rust
use waterui::prelude::*;

fn layered() {
    let env = Environment::new()
        .extending(1_i32)
        .extending(2_i32)
        .extending(3_i32);

    assert_eq!(env.get_nth::<i32>(0), Some(&3));
    assert_eq!(env.get_nth::<i32>(1), Some(&2));
    assert_eq!(env.get_nth::<i32>(2), Some(&1));
}
```

`env.layered_on(&parent)` replays this environment's overlays on top of `parent`, preserving repeated same-typed entries instead of collapsing them. Hydrolysis uses it to put your environment over its default tokens; you rarely call it yourself.

### Several values of one type with `Store`

`Store<K, V>` pairs a value with a zero-sized marker type `K`, so one `V` can appear in several roles. `store` consumes the environment and returns it; `query` reads a role back:

```rust
use waterui::prelude::*;

struct Primary;
struct Secondary;

fn roles() {
    let env = Environment::new()
        .store::<Primary, _>(Color::blue())
        .store::<Secondary, _>(Color::srgb(255, 152, 0));

    let primary: Option<&Color> = env.query::<Primary, Color>();
    let secondary: Option<&Color> = env.query::<Secondary, Color>();
    assert!(primary.is_some() && secondary.is_some());
}
```

## Reaching the environment from a view

### Extractors

A type can be pulled out of an environment when it implements `Extractor`. The same trait serves `use_env` parameters and action handler parameters. Several implementations exist already:

| Extractor | Behavior |
|-----------|----------|
| `Use<T>` | Clones a `T` stored in the environment; fails if absent |
| `State<T>` | Clones a value installed with `ViewExt::state` or `App::state` |
| `Environment` | Clones the whole environment |
| `Option<E: Extractor>` | Turns an extraction failure into `None` |
| `(A, B, ...)` | Extracts each element, up to 8-tuples |

For a `Clone` type you own and install directly with `insert`, `impl_extractor!` writes the `Extractor` impl, so the type can be named as a parameter without the `Use` wrapper:

```rust
use waterui::impl_extractor;
use waterui::prelude::*;

#[derive(Clone)]
struct ApiClient {
    base_url: String,
}

impl_extractor!(ApiClient);
```

The `#[state]` attribute does the same job for a type installed through `.state(&value)` instead of `insert`; [Modifiers](04-modifiers.md) covers `.state(&value)` and [Controls](../03-ui/03-controls.md#owned-state-types-with-state) covers `#[state]`.

### `use_env`

`use_env` builds a view from extracted values. The closure runs once, when the renderer evaluates the view's body, not again on later state changes:

```rust
use waterui::env::use_env;
use waterui::impl_extractor;
use waterui::prelude::*;

#[derive(Clone)]
struct ApiClient {
    base_url: String,
}

impl_extractor!(ApiClient);

fn endpoint() -> impl View {
    use_env(|client: ApiClient| text(client.base_url))
}
```

Extraction is fail-fast. When the value is missing, the closure does not run: a debug build panics with a message naming the type, and a release build logs the failure through `tracing` and renders nothing. Take `Option<E>` when absence is legitimate, and a tuple to read several values at once:

```rust
use waterui::env::use_env;
use waterui::impl_extractor;
use waterui::prelude::*;

#[derive(Clone)]
struct ApiClient {
    base_url: String,
}

impl_extractor!(ApiClient);

fn endpoint_or_placeholder() -> impl View {
    use_env(|client: Option<ApiClient>| match client {
        Some(client) => text(client.base_url),
        None => text("Not configured"),
    })
}

fn endpoint_with_retries() -> impl View {
    use_env(|(client, Use(retries)): (ApiClient, Use<u32>)| {
        text(format!("{} (retries: {retries})", client.base_url))
    })
}
```

### From an action handler

Handlers take the same extractors as parameters, so a button can reach a service without capturing it:

```rust
use waterui::impl_extractor;
use waterui::prelude::*;

#[derive(Clone)]
struct ApiClient {
    base_url: String,
}

impl_extractor!(ApiClient);

fn send_button() -> impl View {
    button("Send").action(|client: ApiClient| {
        waterui::log::info!("sending to {}", client.base_url);
    })
}
```

A handler runs under the environment of the view that owns it, so the value has to be installed on that view or one of its ancestors.

### Scoping a value to a subtree

`ViewExt::with` injects a value for a view and everything below it, and the free function `waterui::env::with` does the same without the method syntax. The value shadows an inherited one of the same type for that subtree only:

```rust
use waterui::env::{use_env, with};
use waterui::impl_extractor;
use waterui::prelude::*;

#[derive(Clone)]
struct ApiClient {
    base_url: String,
}

impl_extractor!(ApiClient);

fn endpoint() -> impl View {
    use_env(|client: ApiClient| text(client.base_url))
}

fn staging_panel() -> impl View {
    let staging = ApiClient {
        base_url: String::from("https://staging.crates.io/api/v1"),
    };
    endpoint().with(staging)
}

fn production_panel(production: ApiClient) -> impl View {
    with(endpoint(), production)
}
```

`ViewExt::install` is the same operation for a [plugin](#plugins), described below.

## Theme tokens

Colors and fonts that follow the platform are environment values too. The color slots (`Background`, `Foreground`, `Accent`, and the rest listed in `waterui::theme::color`) and the font slots are signals stored in the environment, and a view reads one by naming the token:

```rust
use waterui::prelude::*;
use waterui::theme::color::{Background, Foreground};

fn themed() -> impl View {
    text!("Hello, World!")
        .foreground(Foreground)
        .background(Background)
}
```

**Theme tokens are always installed by the runtimes.** Each runtime builds the environment your view runs in with the color slots, the color scheme and the font slots already present: the Apple backend installs them from the platform's current appearance, Hydrolysis installs the framework's default palette and type scale and then layers a style package over it, and the offscreen test harness installs the same defaults. You never install a theme just to make `Foreground` resolve. Anything you install wins on both mainline backends: the Apple backend fills the environment before it calls `app(env)`, and Hydrolysis orders framework defaults, then the style package, then your own environment.

The failure mode only appears when you build an `Environment::new()` yourself and mount it without a runtime. Resolving a color slot nobody installed is a contract violation: a debug build panics with `WaterUI color token ... is not installed in the environment`, and a release build logs the error and resolves opaque black.

### Overriding tokens

`Theme` is a plugin that overrides only what you set. `ColorSettings` holds one optional `WorkingColor` signal per slot, and every setter takes `impl IntoSignal<WorkingColor>`, so a static `Srgb` and a reactive `Binding` both work:

```rust
use waterui::app::App;
use waterui::color::Srgb;
use waterui::prelude::*;

fn root() -> impl View {
    "Hello, WaterUI!"
}

fn brand_theme() -> Theme {
    Theme::new().colors(
        ColorSettings::new()
            .accent(Srgb::from_hex("#0066CC"))
            .accent_foreground(Srgb::from_hex("#FFFFFF")),
    )
}

pub fn app(mut env: Environment) -> App {
    env.install(brand_theme());
    App::new(root, env)
}
```

`Theme::color_scheme` accepts a static `ColorScheme` or a reactive signal, so you can pin a dark appearance or drive it from your own state. Under Hydrolysis the default palette follows the scheme your app installs. The Apple backend derives its color slots from the system appearance instead, so there a pinned scheme changes what views reading the scheme see but does not recolor the slots you leave unset:

```rust
use waterui::prelude::*;

fn pinned_dark() -> Theme {
    Theme::new().color_scheme(ColorScheme::Dark)
}
```

To restyle one screen, install a `Theme` on that subtree. Slots you leave unset keep their inherited values:

```rust
use waterui::color::Srgb;
use waterui::prelude::*;
use waterui::theme::color::Error;

fn danger_zone() -> impl View {
    vstack((
        text("Danger zone").foreground(Error),
        text("Deleting your account cannot be undone."),
    ))
    .install(Theme::new().colors(ColorSettings::new().error(Srgb::from_hex("#B00020"))))
}
```

Because the slots are signals, a token tracks changes per property without rebuilding the subtree. `Theme`, `ColorSettings` and `FontSettings` are in the prelude. The styling vocabulary, including component style tokens and Material 3, is the subject of [the Styling chapter](../03-ui/11-styling.md).

## Keeping guards alive with `retain`

Anything RAII-scoped, such as a signal watcher guard, a subscription or a task handle, is dropped at the end of the function that created it unless you tie it to the view:

```rust
use waterui::prelude::*;

fn watching(data: Binding<i32>) -> impl View {
    let guard = data.clone().watch(|value| {
        waterui::log::debug!("data changed: {}", value.into_value());
    });

    text!("Watching {data}").retain(guard)
}
```

`retain` wraps the view in `Metadata<Retain>`. Retain several values by chaining `.retain(a).retain(b)` or by passing a tuple. If a side effect "does not work", check that its guard is retained: an unretained watcher unsubscribes the moment the body returns.

## Hooks: intercepting component configuration

Hooks are how a theme replaces the rendering of every button in an app without editing a call site. A configurable component builds a `Config` value describing what it wants to show, and `Hook<Config>` is a function from an environment and that configuration to a view. `Button`'s `body` is the canonical case: it looks up `Hook<ButtonConfig>` in the environment and applies it if present, and otherwise renders the platform control.

Install one with `Environment::insert_hook`. `render()` comes from the `ViewConfiguration` trait, so bring it into scope:

```rust
use waterui::component::button::ButtonConfig;
use waterui::prelude::*;
use waterui::view::ViewConfiguration;

fn install_button_hook(env: &mut Environment) {
    env.insert_hook(|_env: &Environment, config: ButtonConfig| {
        config.render().padding().background(Color::blue())
    });
}
```

The hook runs with itself *removed* from the environment it receives. That is deliberate: `config.render()` inside a hook produces the default rendering instead of recursing forever, so a hook wraps the platform control rather than having to replace it. See [the View chapter](01-view.md) for the `ViewConfiguration` side of the contract.

## Plugins

A `Plugin` bundles related setup, such as values, hooks and nested plugins, behind one call. The trait is re-exported at the facade root as `waterui::Plugin`:

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

The default `install` stores the plugin value itself, which is how a plugin doubles as readable configuration. Override it to do more:

```rust
use waterui::Plugin;
use waterui::component::button::ButtonConfig;
use waterui::prelude::*;
use waterui::shape::RoundedRectangle;
use waterui::view::ViewConfiguration;

struct RoundedButtons;

impl Plugin for RoundedButtons {
    fn install(self, env: &mut Environment) {
        env.insert(self);
        env.insert_hook(|_env: &Environment, config: ButtonConfig| {
            config
                .render()
                .padding()
                .background(Color::blue())
                .clip(RoundedRectangle::new(0.2))
        });
    }
}
```

`RoundedRectangle::new` takes a *normalized* corner radius, a fraction of the shorter side with `0.5` fully rounded, so `0.2` looks the same on a small button and a large one.

Install at the root for the whole app with `Environment::install`, or on a subtree with `ViewExt::install`:

```rust
use waterui::Plugin;
use waterui::app::App;
use waterui::component::button::ButtonConfig;
use waterui::prelude::*;
use waterui::shape::RoundedRectangle;
use waterui::view::ViewConfiguration;

struct RoundedButtons;

impl Plugin for RoundedButtons {
    fn install(self, env: &mut Environment) {
        env.insert(self);
        env.insert_hook(|_env: &Environment, config: ButtonConfig| {
            config
                .render()
                .padding()
                .background(Color::blue())
                .clip(RoundedRectangle::new(0.2))
        });
    }
}

fn screen() -> impl View {
    vstack((
        button("Save").action(|| {}),
        button("Cancel").action(|| {}),
    ))
    .install(RoundedButtons)
}

pub fn app(env: Environment) -> App {
    // To apply it app-wide instead, declare `mut env` and call
    // `env.install(RoundedButtons)` before `App::new`.
    App::new(screen, env)
}
```

Try installing the plugin on one section and compare it with the rest of the app: only that subtree's buttons change. [The Plugins chapter](../06-advanced/07-plugins.md) goes further.

## Metadata

Some instructions ride along with a view instead of living in the environment. `Metadata<T>` is the mandatory form: a renderer that meets a `Metadata<T>` it does not handle breaks the contract, so a debug build panics and a release build logs the error and renders nothing. That is what keeps environment overrides and lifecycle hooks from being silently dropped. `IgnorableMetadata<T>` is the optional form, passed through by renderers that do not implement it; accessibility hints use it.

You rarely construct either directly. The `ViewExt` modifiers do it for you: `.with(...)` and `.install(...)` render as `Metadata<Environment>`, `.retain(...)` produces `Metadata<Retain>`, `.on_appear(...)` and `.shadow(...)` produce `Metadata`, and `.a11y_label(...)` produces `IgnorableMetadata`. A library that defines its own renderer-visible marker implements `MetadataKey` and wraps views with these types, all reachable from `waterui::metadata`.

Next: [Modifiers and ViewExt](04-modifiers.md), the chainable methods that style, position, and add behavior to any view.

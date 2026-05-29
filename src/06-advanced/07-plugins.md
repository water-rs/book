# Plugins

> **In this chapter, you will:**
> - Understand the `Plugin` trait and how it integrates with `Environment`
> - Install plugins globally or scoped to a view subtree
> - Build plugins for theming, analytics, and default error/loading views
> - Compose multiple plugins into setup functions
> - Use keyed storage with `Store<K, V>` for plugins that hold many values of one type

As your application grows, you accumulate cross-cutting concerns: theming,
analytics, default error views, loading indicators. Scattering `env.insert(...)`
calls through view code gets messy fast. WaterUI's plugin system gives you a
clean pattern: a plugin is a self-contained unit that installs itself into an
`Environment`, injecting services, configuration, or view hooks that every view
in the hierarchy can read.

## The Plugin trait

The `Plugin` trait lives in `waterui_core::plugin` and is intentionally
minimal:

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

Both methods have default bodies:

- `install` stores the plugin instance keyed by its concrete type.
- `uninstall` removes that instance.

You override `install` when the plugin needs to do more than just store
itself, for example, register a service, install a hook, or extract data into
multiple environment slots.

## A minimal plugin

The simplest plugin just stores itself in the environment:

```rust,ignore
use waterui_core::{plugin::Plugin, Environment};

struct MyPlugin;
impl Plugin for MyPlugin {}

let mut env = Environment::new();
env.install(MyPlugin);

// Later, any view can check if the plugin is active.
assert!(env.get::<MyPlugin>().is_some());
```

This is useful as a feature flag or a marker that some behavior is enabled.
Most plugins do more interesting work during installation.

## Installing plugins

There are two ways to install a plugin: globally on the application
environment, or locally on a view subtree.

### During environment setup

Install plugins when building the application's environment. `Environment::install`
calls `plugin.install(&mut self)` and returns `&mut Self` for chaining:

```rust,ignore
use waterui::prelude::*;
use waterui::app::App;

pub fn app(env: Environment) -> App {
    let mut env = env;
    env.install(ThemePlugin::dark())
        .install(AnalyticsPlugin::new("api-key"));
    App::new(main, env)
}

fn main() -> impl View {
    text("Hello")
}
```

### Per-view with ViewExt

`ViewExt::install` installs a plugin for a specific subtree by cloning the
environment, applying the plugin, and wrapping the view with the modified
environment:

```rust,ignore
use waterui::prelude::*;

fn themed_section() -> impl View {
    vstack((
        text("Dark mode section"),
        text("All children see DarkTheme"),
    ))
    .install(DarkThemePlugin)
}
```

## Building a custom plugin

A useful plugin typically:

1. Carries configuration.
2. Inserts services or values into the environment during `install`.
3. Optionally cleans up during `uninstall`.

The next sections walk through real-world examples.

### Theming plugin

This plugin installs a color palette that any view in the hierarchy can read.
Note that `Use<T>` is the extractor wrapper: any `T: 'static + Clone` becomes
extractable through `Use<T>` without you implementing `Extractor` yourself.

```rust,ignore
use waterui::prelude::*;
use waterui::graphics::color::Color;
use waterui_core::{
    Environment,
    extract::Use,
    plugin::Plugin,
};

#[derive(Debug, Clone)]
pub struct ThemeConfig {
    pub primary: Color,
    pub secondary: Color,
    pub background: Color,
}

pub struct ThemePlugin {
    config: ThemeConfig,
}

impl ThemePlugin {
    pub fn dark() -> Self {
        Self {
            config: ThemeConfig {
                primary: Color::srgb(100, 149, 237),
                secondary: Color::srgb(144, 238, 144),
                background: Color::srgb(30, 30, 30),
            },
        }
    }

    pub fn light() -> Self {
        Self {
            config: ThemeConfig {
                primary: Color::srgb(0, 122, 255),
                secondary: Color::srgb(52, 199, 89),
                background: Color::srgb(255, 255, 255),
            },
        }
    }
}

impl Plugin for ThemePlugin {
    fn install(self, env: &mut Environment) {
        env.insert(self.config);
    }
}

fn themed_card() -> impl View {
    use_env(|Use(config): Use<ThemeConfig>| {
        vstack((
            text("Themed Card").foreground(config.primary),
            text("Secondary text").foreground(config.secondary),
        ))
        .background(config.background)
    })
}
```

### Analytics plugin

A plugin that installs a service for event tracking. The service itself is
`Clone`, so views can extract it through `Use<AnalyticsService>` and move it
into action closures:

```rust,ignore
use waterui::prelude::*;
use waterui_core::{
    Environment,
    extract::Use,
    plugin::Plugin,
};

#[derive(Clone)]
pub struct AnalyticsService {
    api_key: String,
}

impl AnalyticsService {
    pub fn track(&self, event: &str) {
        tracing::info!(api_key = %self.api_key, event, "analytics event");
    }
}

pub struct AnalyticsPlugin {
    api_key: String,
}

impl AnalyticsPlugin {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self { api_key: api_key.into() }
    }
}

impl Plugin for AnalyticsPlugin {
    fn install(self, env: &mut Environment) {
        env.insert(AnalyticsService { api_key: self.api_key });
    }
}

fn tracked_button() -> impl View {
    use_env(|Use(analytics): Use<AnalyticsService>| {
        button("Purchase").action(move || {
            analytics.track("purchase_clicked");
        })
    })
}
```

### Default error view plugin

Install a `DefaultErrorView` that any error in the subtree falls back to.
See [Error handling](04-error-handling.md) for context. The `text!` macro reads
named placeholders from the surrounding scope, so bind `message` first:

```rust,ignore
use waterui::prelude::*;
use waterui::widget::error::{BoxedStdError, DefaultErrorView};
use waterui_core::{Environment, plugin::Plugin};

pub struct ErrorViewPlugin;

impl Plugin for ErrorViewPlugin {
    fn install(self, env: &mut Environment) {
        env.insert(DefaultErrorView::new(|error: BoxedStdError| {
            let message = Binding::container(error.to_string());
            vstack((
                text("Something went wrong").headline(),
                text!("{message}"),
            ))
            .padding()
        }));
    }
}
```

### Default loading view plugin

The same shape works for `Suspense` fallbacks. `loading()` returns an
indeterminate circular `Progress` indicator:

```rust,ignore
use waterui::prelude::*;
use waterui::widget::suspense::DefaultLoadingView;
use waterui_core::{Environment, plugin::Plugin};

pub struct LoadingViewPlugin;

impl Plugin for LoadingViewPlugin {
    fn install(self, env: &mut Environment) {
        env.insert(DefaultLoadingView::new(|| {
            vstack((
                loading(),
                text("Loading..."),
            ))
        }));
    }
}
```

## Plugin lifecycle

Plugins are installed once during environment setup. The lifecycle is:

1. **Installation** — `install(self, env)` runs and injects values.
2. **Active** — installed services are visible to every view that reads the
   environment.
3. **Uninstallation** — `uninstall(self, env)` removes the plugin entry. This
   is rarely needed at runtime, but is the way to undo a per-subtree install.

> **Note:** `Environment` is a type-indexed map, so installing the same plugin
> type twice replaces the first instance. Treat that as the intended way to
> override defaults, not a bug.

## Querying plugin state

The `Environment` provides two ways to look up values that plugins installed.

### Direct type lookup

```rust,ignore
let plugin = env.get::<ThemePlugin>();
let config = env.get::<ThemeConfig>();
```

### Keyed store lookup

When a plugin needs to install several values of the same type under different
logical keys, use `Environment::store` and `Environment::query`. The `K` type
acts as a phantom key; the `V` is the actual stored value:

```rust,ignore
use waterui::Environment;

struct ApiBaseUrl;
struct CdnBaseUrl;

let env = Environment::new()
    .store::<ApiBaseUrl, _>("https://api.github.com".to_string())
    .store::<CdnBaseUrl, _>("https://static.rust-lang.org".to_string());

let api_url = env.query::<ApiBaseUrl, String>();
let cdn_url = env.query::<CdnBaseUrl, String>();
```

This is the same mechanism the theme system uses to keep many color and font
slots distinct under one container.

## Composing plugins

Larger applications benefit from composing many plugins into a single setup
function. This keeps `app()` clean and makes it easy to swap configurations:

```rust,ignore
use waterui::Environment;

fn setup_production(env: &mut Environment, analytics: AnalyticsPlugin) {
    env.install(ThemePlugin::light())
        .install(analytics)
        .install(ErrorViewPlugin)
        .install(LoadingViewPlugin);
}

fn setup_development(env: &mut Environment) {
    env.install(ThemePlugin::dark())
        .install(ErrorViewPlugin)
        .install(LoadingViewPlugin);
}
```

## Plugins and view hooks

A plugin's `install` body is the natural place to register a view hook —
a function that intercepts a `ViewConfiguration` for a given component and
substitutes a new view. The full mechanics live in the
[Resolvers and hooks](08-resolvers.md) chapter; the shape inside a plugin
looks like this:

```rust,ignore
use waterui::prelude::*;
use waterui::component::button::ButtonConfig;
use waterui_core::{Environment, plugin::Plugin};

pub struct LoggingButtonsPlugin;

impl Plugin for LoggingButtonsPlugin {
    fn install(self, env: &mut Environment) {
        env.insert_hook(|_env, config: ButtonConfig| {
            tracing::debug!(?config, "button rendered");
            config.render()
        });
    }
}
```

`Environment::insert_hook` accepts any `Fn(&Environment, C) -> impl View`
where `C: ViewConfiguration`. It boxes the closure into a `Hook<C>` and stores
it under that configuration's type.

## Best practices

1. **Keep plugins focused.** Each plugin should install one logical unit.
   Prefer many small plugins over one large one.
2. **Document what gets installed.** Callers need to know which types appear
   in the environment after installation.
3. **Prefer `Use<T>` extractors** so views read services through `use_env(|Use(svc): Use<T>| ...)`
   instead of grabbing the environment directly.
4. **Avoid side effects in `install`.** Plugins should configure the
   environment, not perform I/O or spawn tasks. Defer runtime behavior to the
   services they install.
5. **Use `env.install(plugin)` instead of `env.insert(plugin)`.** The plugin
   pattern documents intent and lets the plugin run custom installation logic.

## Summary

| API | Purpose |
|---|---|
| `Plugin` trait | Core interface for environment extensions |
| `Plugin::install(self, env)` | Add functionality to the environment |
| `Plugin::uninstall(self, env)` | Remove functionality |
| `Environment::install(plugin)` | Install a plugin (chainable) |
| `ViewExt::install(plugin)` | Install a plugin for a view subtree |
| `Environment::insert(value)` | Store a typed value |
| `Environment::get::<T>()` | Retrieve a typed value |
| `Environment::store::<K, V>(value)` | Store under a phantom key |
| `Environment::query::<K, V>()` | Retrieve under a phantom key |
| `Environment::insert_hook(f)` | Install a hook over `ViewConfiguration` |
| `Environment::remove::<T>()` | Remove a typed value |

## Next

Plugins install services and configuration. But how do colors, fonts, and
other design tokens become reactive values that update when the OS toggles
dark mode? Move on to [Resolvers and hooks](08-resolvers.md) to see the
machinery underneath the theme system and `.foreground()` modifier.

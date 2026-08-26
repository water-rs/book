# Plugins

> **In this chapter, you will:**
> - Install services and configuration into an `Environment` with the `Plugin` trait
> - Scope a plugin to the whole app or to one view subtree
> - Make installed values extractable so views read them as typed parameters
> - Store several values of one type under phantom keys with `store` / `query`
> - Register a view hook from inside a plugin

Theming, analytics, default error and loading views: cross-cutting concerns accumulate, and scattering `env.insert(...)` calls through view code hides what an application actually depends on. A plugin packages one of those concerns as a value that knows how to install itself.

## The Plugin trait

`Plugin` is re-exported at the facade root as `waterui::Plugin` (it is not in the prelude):

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

Both methods have defaults: `install` stores the plugin keyed by its own concrete type, `uninstall` removes it. Override `install` when the plugin needs to inject something other than itself — a service, several values, or a hook.

The empty implementation is already useful as a feature marker:

```rust,ignore
use waterui::{Environment, Plugin};

struct DebugOverlay;
impl Plugin for DebugOverlay {}

let mut env = Environment::new();
env.install(DebugOverlay);
assert!(env.get::<DebugOverlay>().is_some());
```

## Installing

### For the whole application

`Environment::install` calls `plugin.install(&mut self)` and returns `&mut Self`, so installations chain:

```rust,ignore
use waterui::app::App;
use waterui::prelude::*;

pub fn app(env: Environment) -> App {
    let mut env = env;
    env.install(ThemePlugin::dark())
        .install(AnalyticsPlugin::new("api-key"));
    App::new(main, env)
}
```

### For one subtree

`ViewExt::install` clones the environment, applies the plugin to the clone, and attaches it to the wrapped view. Everything outside is unaffected:

```rust,ignore
use waterui::prelude::*;

fn themed_section() -> impl View {
    vstack((
        text("This section uses the dark palette"),
        text("So does everything below it"),
    ))
    .install(ThemePlugin::dark())
}
```

## Building one

### A configuration plugin

Make the installed type an extractor with `impl_extractor!` so views receive it as a typed parameter instead of reaching into the environment by hand. The macro requires the type to be `Clone`:

```rust,ignore
use waterui::env::use_env;
use waterui::prelude::*;
use waterui::{Environment, Plugin, impl_extractor};

#[derive(Debug, Clone)]
pub struct ThemeConfig {
    pub primary: Color,
    pub secondary: Color,
    pub background: Color,
}

impl_extractor!(ThemeConfig);

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
}

impl Plugin for ThemePlugin {
    fn install(self, env: &mut Environment) {
        env.insert(self.config);
    }
}

fn themed_card() -> impl View {
    use_env(|config: ThemeConfig| {
        vstack((
            text("Themed card").foreground(config.primary),
            text("Secondary text").foreground(config.secondary),
        ))
        .background(config.background)
    })
}
```

Note the shape: `ThemePlugin` is the installer and disappears after installation; `ThemeConfig` is what views actually read. Keeping them separate means a view depends on the data, not on which plugin happened to provide it.

For real color work, prefer the built-in theme tokens (`theme_color::Accent` and friends) over a bespoke palette type — they already resolve reactively and follow the system appearance. A custom config type is for values the theme system does not model.

### A service plugin

Same shape, with behavior attached. The service is `Clone`, so it can be moved into action closures:

```rust,ignore
use waterui::env::use_env;
use waterui::prelude::*;
use waterui::{Environment, Plugin, impl_extractor};

#[derive(Clone)]
pub struct AnalyticsService {
    api_key: String,
}

impl_extractor!(AnalyticsService);

impl AnalyticsService {
    pub fn track(&self, event: &str) {
        tracing::info!(api_key = %self.api_key, event, "analytics");
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
    use_env(|analytics: AnalyticsService| {
        button("Purchase").action(move || analytics.track("purchase_clicked"))
    })
}
```

Because `AnalyticsService` is an extractor, a handler can also take it directly as a parameter, exactly like `State<T>`.

### Framework defaults

`DefaultErrorView` and `DefaultLoadingView` are ordinary environment values, so a plugin is the natural place to set them:

```rust,ignore
use waterui::prelude::*;
use waterui::widget::error::{BoxedStdError, DefaultErrorView};
use waterui::widget::suspense::DefaultLoadingView;
use waterui::{Environment, Plugin};

pub struct AppChromePlugin;

impl Plugin for AppChromePlugin {
    fn install(self, env: &mut Environment) {
        env.insert(DefaultErrorView::new(|error: BoxedStdError| {
            let message = Binding::container(error.to_string());
            vstack((text("Something went wrong").headline(), text!("{message}"))).padding()
        }));

        env.insert(DefaultLoadingView::new(|| {
            vstack((loading(), text("Loading...")))
        }));
    }
}
```

`loading()` is the facade's indeterminate circular `Progress`. See [Error handling](04-error-handling.md) and [Suspense](03-suspense.md) for what consumes these.

## Lifecycle

Installation runs once; the installed values then stay visible to every view that reads that environment. `uninstall` removes the plugin entry and is mainly useful for undoing a per-subtree install.

`Environment` is a type-indexed map, so installing the same plugin type twice replaces the first instance. That is the intended way to override a default, not an accident to guard against.

## Keyed storage

When a plugin installs several values of the *same* type under different logical meanings, a type key disambiguates them. `Environment::store` takes `self` and returns the extended environment; `query` reads it back:

```rust,ignore
use waterui::Environment;

struct ApiBaseUrl;
struct CdnBaseUrl;

let env = Environment::new()
    .store::<ApiBaseUrl, _>("https://api.github.com".to_string())
    .store::<CdnBaseUrl, _>("https://static.rust-lang.org".to_string());

let api_url = env.query::<ApiBaseUrl, String>();   // Option<&String>
let cdn_url = env.query::<CdnBaseUrl, String>();
```

`ApiBaseUrl` and `CdnBaseUrl` are never constructed; they exist only as keys. This is the same mechanism the theme system uses to keep many font slots distinct in one environment.

## Composing

Grouping installations into named setups keeps `app()` readable and makes swapping configurations a one-line change:

```rust,ignore
use waterui::Environment;

fn setup_production(env: &mut Environment, analytics: AnalyticsPlugin) {
    env.install(ThemePlugin::light())
        .install(analytics)
        .install(AppChromePlugin);
}

fn setup_development(env: &mut Environment) {
    env.install(ThemePlugin::dark()).install(AppChromePlugin);
}
```

## Registering a hook

A plugin's `install` body is also where a view hook belongs — a function that intercepts a component's `ViewConfiguration` and returns a substitute view:

```rust,ignore
use waterui::component::button::ButtonConfig;
use waterui::prelude::*;
use waterui::view::ViewConfiguration;
use waterui::{Environment, Plugin};

pub struct LoggingButtonsPlugin;

impl Plugin for LoggingButtonsPlugin {
    fn install(self, env: &mut Environment) {
        env.insert_hook(|env, config: ButtonConfig| {
            tracing::debug!(?config, "button rendered");
            config.render()
        });
    }
}
```

`Environment::insert_hook` accepts any `Fn(&Environment, C) -> impl View` where `C: ViewConfiguration`, boxes it into a `Hook<C>`, and stores it under the configuration's type. [Resolvers and hooks](08-resolvers.md) covers the mechanism.

## Guidelines

- **One plugin, one concern.** Many small plugins compose; one large one does not.
- **Separate the installer from the installed.** Views should depend on `ThemeConfig`, not `ThemePlugin`.
- **Make installed types extractors.** `impl_extractor!` turns `use_env(|svc: MyService| ...)` and typed handler parameters on, and keeps `env.get::<T>()` out of view code.
- **Do not perform I/O in `install`.** Configure the environment; let the installed service do the work when something calls it.
- **Document what appears in the environment.** A plugin's public contract is the set of types it inserts.

---

Next: [Resolvers and hooks](08-resolvers.md), which is the machinery that turns a token like `theme_color::Accent` into a reactive value and lets a hook rewrite a component before it renders.

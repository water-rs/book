# Plugins

> **In this chapter, you will:**
> - Package a cross-cutting concern as a `Plugin` and install it into an `Environment`
> - Scope a plugin to the whole app or to one view subtree
> - Use the built-in `Theme` plugin with `ColorSettings`
> - Make installed values extractable so views read them as typed parameters
> - Store several values of one type under phantom keys, and register a view hook from `install`

Theming, analytics, default error and loading views: cross-cutting concerns accumulate, and scattering `env.insert(...)` calls through view code hides what an application depends on. A plugin packages one concern as a value that knows how to install itself into an [`Environment`](../02-core/03-environment.md).

## The Plugin trait

`Plugin` is re-exported at the facade root as `waterui::Plugin`. It is not in the prelude.

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

Both methods have defaults: `install` stores the plugin keyed by its own concrete type, and `uninstall` removes that entry. Override `install` when the plugin must inject something other than itself: a service, several values, or a hook. An empty `impl Plugin for T {}` is already useful as a feature marker:

```rust
use waterui::prelude::*;
use waterui::Plugin;

struct DebugOverlay;
impl Plugin for DebugOverlay {}

fn marker_roundtrip() {
    let mut env = Environment::new();
    env.install(DebugOverlay);
    assert!(env.get::<DebugOverlay>().is_some());
}
```

## Installing

### For the whole application

`Environment::install(&mut self, plugin: impl Plugin) -> &mut Self` calls `plugin.install(self)` and returns the environment, so installations chain. The `Environment` your `app` function receives is the place to do it, before you hand it to `App::new`; see [the environment chapter](../02-core/03-environment.md) and [project structure](../01-getting-started/04-project-structure.md) for where that function lives.

```rust
use waterui::app::App;
use waterui::color::Srgb;
use waterui::prelude::*;
use waterui::theme::{ColorScheme, ColorSettings, Theme};

fn home() -> impl View {
    text("Hello")
}

pub fn app(mut env: Environment) -> App {
    env.install(
        Theme::new()
            .color_scheme(ColorScheme::Dark)
            .colors(ColorSettings::new().accent(Srgb::from_hex("#0066CC"))),
    );
    App::new(home, env)
}
```

`Theme` is the framework's own plugin. It installs only the parts you set (color scheme, `ColorSettings`, `FontSettings`) and leaves every other slot as it was. Color slots are `WorkingColor` signals, so each setter takes anything that converts into a signal of one: a static color, or a reactive value that restyles the app when it changes.

### For one subtree

`ViewExt::install(self, plugin: impl Plugin) -> impl View` clones the environment, applies the plugin to the clone, and attaches the clone to the wrapped view. Everything outside the view keeps the original environment.

```rust
use waterui::prelude::*;
use waterui::color::Srgb;
use waterui::theme::{ColorScheme, ColorSettings, Theme};

fn dark_section() -> impl View {
    vstack((
        text("This section uses the dark color scheme"),
        text("So does everything inside it"),
    ))
    .install(
        Theme::new()
            .color_scheme(ColorScheme::Dark)
            .colors(ColorSettings::new().background(Srgb::from_hex("#1E1E1E"))),
    )
}
```

Other modifiers use the same mechanism: `.foreground(color)` installs a `ForegroundOverride` plugin, which resolves the color against the environment and installs it into the `Foreground` slot for the subtree. See [Styling](../03-ui/11-styling.md) for the theme tokens views read.

## Building a plugin

### A configuration plugin

When the plugin's only job is to make one value available, let the value be the plugin. The default `install` inserts it. Add `impl_extractor!` (re-exported at the facade root) so views receive it as a typed parameter instead of reaching into the environment. The macro requires the type to be `Clone`.

```rust
use waterui::env::use_env;
use waterui::prelude::*;
use waterui::{Plugin, impl_extractor};

#[derive(Clone)]
pub struct Branding {
    pub app_name: Str,
    pub support_url: Str,
}

impl Plugin for Branding {}
impl_extractor!(Branding);

fn footer() -> impl View {
    use_env(|branding: Branding| {
        vstack((
            text(branding.app_name),
            text(branding.support_url),
        ))
    })
}

fn install_branding(env: &mut Environment) {
    env.install(Branding {
        app_name: "WaterUI 水UI ウォーターUI 워터UI".into(),
        support_url: "https://github.com/water-rs/waterui/issues".into(),
    });
}
```

`use_env` runs the closure with the extracted value. If the plugin was never installed, extraction fails and the closure does not run: that is a contract violation, which panics in a debug build and is logged in a release build, where the view renders nothing. Install what your views read before the first frame, or take `Option<Branding>` when the value is genuinely optional.

For colors, prefer the built-in theme tokens in `waterui::theme::color` (`Accent`, `Foreground`, `Surface`, and the rest) installed through `Theme`. They resolve reactively and follow the system appearance. A bespoke configuration type is for values the theme system does not model.

### A service plugin

When the installed type differs from the plugin, override `install`. The plugin is a short-lived installer; the service is what views and handlers depend on.

```rust
use waterui::env::use_env;
use waterui::prelude::*;
use waterui::{Plugin, impl_extractor};

#[derive(Clone)]
pub struct AnalyticsService {
    endpoint: String,
}

impl_extractor!(AnalyticsService);

impl AnalyticsService {
    pub fn track(&self, event: &str) {
        waterui::log::info!(endpoint = %self.endpoint, event, "analytics");
    }
}

pub struct AnalyticsPlugin {
    endpoint: String,
}

impl AnalyticsPlugin {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self { endpoint: endpoint.into() }
    }
}

impl Plugin for AnalyticsPlugin {
    fn install(self, env: &mut Environment) {
        env.insert(AnalyticsService { endpoint: self.endpoint });
    }
}

fn tracked_button() -> impl View {
    use_env(|analytics: AnalyticsService| {
        button("Purchase").action(move || analytics.track("purchase_clicked"))
    })
}
```

Keeping the two types apart means a view depends on the data, not on which plugin happened to provide it, and a test can insert a fake `AnalyticsService` directly. Diagnostics go through `tracing`, which the facade re-exports as `waterui::log`; never print.

### Framework defaults

`DefaultErrorView` and `DefaultLoadingView` are ordinary environment values, so a plugin is the natural place to set them:

```rust
use waterui::prelude::*;
use waterui::widget::error::{BoxedStdError, DefaultErrorView};
use waterui::widget::suspense::DefaultLoadingView;
use waterui::Plugin;

pub struct AppChromePlugin;

impl Plugin for AppChromePlugin {
    fn install(self, env: &mut Environment) {
        env.insert(DefaultErrorView::new(|error: BoxedStdError| {
            vstack((text("Something went wrong"), text(error.to_string())))
        }));

        env.insert(DefaultLoadingView::new(|| {
            vstack((loading(), text("Loading...")))
        }));
    }
}
```

`loading()` is the platform's indeterminate loading indicator. See [Error handling](04-error-handling.md) and [Suspense](03-suspense.md) for what consumes these.

## Lifecycle

Installation runs once: at the `env.install(...)` call for an app-wide plugin, and when the wrapped view is built for a subtree install. The installed values then stay visible to every view that reads that environment. `uninstall(self, env)` runs the reverse: by default it removes the plugin's own entry. If you override `install` to insert other types, override `uninstall` to remove them too, or the removal is partial.

```rust
use waterui::prelude::*;
use waterui::Plugin;

struct FeatureFlags {
    new_checkout: bool,
}

impl Plugin for FeatureFlags {}

fn toggle_flags() {
    let mut env = Environment::new();
    env.install(FeatureFlags { new_checkout: true });
    assert!(env.get::<FeatureFlags>().is_some());

    FeatureFlags { new_checkout: true }.uninstall(&mut env);
    assert!(env.get::<FeatureFlags>().is_none());
}
```

`Environment` is a type-indexed map, so installing the same type twice replaces the first instance. That is the intended way to override a default, not something to guard against. A subtree install overrides for that subtree only, because it works on a clone.

## Keyed storage

When several values of the *same* type carry different meanings, a type key disambiguates them. `Environment::store::<K, V>(self, value) -> Self` takes the environment by value and returns the extended one; `query::<K, V>(&self) -> Option<&V>` reads it back.

```rust
use waterui::prelude::*;

struct ApiBaseUrl;
struct CdnBaseUrl;

fn urls() {
    let env = Environment::new()
        .store::<ApiBaseUrl, _>("https://crates.io/api/v1".to_string())
        .store::<CdnBaseUrl, _>("https://static.crates.io".to_string());

    assert_eq!(
        env.query::<ApiBaseUrl, String>().map(String::as_str),
        Some("https://crates.io/api/v1"),
    );
    assert!(env.query::<CdnBaseUrl, String>().is_some());
}
```

`ApiBaseUrl` and `CdnBaseUrl` are never constructed; they exist only as keys. The theme system uses the same mechanism: installing a background color also mirrors it into a keyed entry for the graphics layer's background slot, so canvas and shader code sees the same palette as views.

## Registering a hook

A plugin's `install` is also where a view hook belongs: a function that intercepts a component's `ViewConfiguration` and returns a replacement view. `Environment::insert_hook` takes `impl Fn(&Environment, C) -> V + 'static` with `C: ViewConfiguration` and `V: View`, and stores it as a `Hook<C>`. An environment holds one hook per configuration type, so inserting a second replaces the first.

```rust
use waterui::component::button::ButtonConfig;
use waterui::prelude::*;
use waterui::view::ViewConfiguration;
use waterui::Plugin;

pub struct LoggingButtonsPlugin;

impl Plugin for LoggingButtonsPlugin {
    fn install(self, env: &mut Environment) {
        env.insert_hook(|_env: &Environment, config: ButtonConfig| {
            waterui::log::debug!("button realized");
            config.render()
        });
    }
}
```

The hook runs with a copy of the environment from which the hook itself has been removed, and the view it returns renders in that copy. So `config.render()`, which turns the configuration back into a `Button`, produces the ordinary button instead of re-entering the hook. [Resolvers and hooks](08-resolvers.md) covers the mechanism in full.

## Plugins in the ecosystem

Components that live in their own repositories ship their setup the same way: a function, or a plugin, that you call on the environment at the root of the app.

- **GPU maps.** `waterui-map-gpu` exposes `install(env: &mut Environment)`, which registers a hook for `MapConfig` that draws the map on the GPU. It needs a `MapGpuOptions` value in the environment (the crate deliberately does not hard-code a tile service); the hook panics when it realizes a map without one. It installs nothing when a hook for `MapConfig` is already registered, because a backend that bridges a platform map, such as MapKit on Apple, has already supplied the realization the platform wants. See [Maps](../04-rich/02-maps.md).
- **Mermaid diagrams.** `waterui-mermaid` is a direct dependency, not a facade feature. Its `install(env: &mut Environment)` registers a hook for `CodeConfig`: a fence tagged `mermaid` becomes a diagram, and any other fence keeps the ordinary code rendering. Until you call it, a `mermaid` fence in Markdown stays plain code. Like the map installer, it does nothing when a `CodeConfig` hook is already present.

Both follow the same shape as the plugins above: configure the environment once, and let components read what they need when they render. Both also show the one-hook-per-type rule at work: an installer that finds a hook already registered yields to it instead of replacing it.
## Guidelines

- **One plugin, one concern.** Many small plugins compose; one large one does not.
- **Separate the installer from the installed** when the installer carries build-time options. Views depend on the service or config, not on the plugin.
- **Make installed types extractors.** `impl_extractor!` turns on `use_env(|svc: MyService| ...)` and keeps `env.get::<T>()` out of view code.
- **Do not perform I/O in `install`.** Configure the environment; let the installed service do the work when something calls it.
- **Document what appears in the environment.** A plugin's public contract is the set of types it inserts.

---

Next: [Resolvers and hooks](08-resolvers.md), the machinery that turns a token like `theme::color::Accent` into a reactive value and lets a hook rewrite a component before it renders.

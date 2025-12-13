# Plugin

WaterUI's plugin system is built at the top of [environment system](../02-core-concept/03-environment).

```rust
pub trait Plugin: Sized + 'static {
    /// Installs this plugin into the provided environment.
    ///
    /// This method adds the plugin instance to the environment's storage,
    /// making it available for later retrieval.
    ///
    /// # Arguments
    ///
    /// * `env` - A mutable reference to the environment
    fn install(self, env: &mut Environment) {
        env.insert(self);
    }

    /// Removes this plugin from the provided environment.
    ///
    /// # Arguments
    ///
    /// * `env` - A mutable reference to the environment
    fn uninstall(self, env: &mut Environment) {
        env.remove::<Self>();
    }
}
```

Plugins are just values stored inside the environment. Because they are regular Rust structs, you
can bundle services (network clients, analytics, feature flags) and install them once near your
entry point.

## Example: i18n

`waterui_i18n::I18n` ships as a plugin that rewrites `Text` views based on the active locale.

```rust
use waterui_i18n::I18n;
use waterui::prelude::*;
use waterui::text::locale::Locale;

fn install_i18n(env: &mut Environment) {
    let mut i18n = I18n::new();
    i18n.insert("en", "greeting", "Hello");
    i18n.insert("fr", "greeting", "Bonjour");

    i18n.install(env);            // stores translations + hook
    env.insert(Locale("fr".into())); // pick initial locale
}
```

Every `text!("greeting")` now passes through the hook registered during `install`, swapping the
content with the localized string.

## Building Your Own Plugin

1. Create a struct that owns the resources you need.
2. Implement `Plugin` (the default methods may be enough) and optionally install environment hooks.
3. Insert helper extractors so views/handlers can access the plugin at runtime.

```rust
use waterui::prelude::*;
use waterui::plugin::Plugin;
use waterui::impl_extractor;

#[derive(Clone)]
pub struct Telemetry { /* ... */ }

impl Plugin for Telemetry {
    fn install(self, env: &mut Environment) {
        env.insert(self);
    }
}

impl_extractor!(Telemetry);
```

Now handlers can request `Use<Telemetry>` exactly like bindings or environment values.

## Lifecycle Hooks

Override `uninstall` when the plugin must clean up:

```rust
use waterui::prelude::*;
use waterui::plugin::Plugin;

struct SessionManager;

impl SessionManager {
    fn shutdown(&self) {}
}

impl Plugin for SessionManager {
    fn uninstall(self, env: &mut Environment) {
        self.shutdown();
        env.remove::<Self>();
    }
}
```

Although most plugins live for the entire lifetime of the app, uninstall hooks are handy in tests or
when swapping environments dynamically (multi-window macOS apps, for example).

Plugins keep cross-cutting concerns modular. Keep their public surface small, expose access through
extractors, and leverage environment hooks to integrate with the rest of the view tree.
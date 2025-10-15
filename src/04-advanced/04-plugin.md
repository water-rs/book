# Plugin

WaterUI's plugin system is built at the top of [environment system](../02-core-concept/03-environment).

```rust,ignore
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

By intersting environment, plugin can achieve something interesting purpose.

## Implement an i18n Plugin

```rust,ignore
use waterui::component::text::{Text, TextConfig};
use waterui::prelude::*;
use waterui::view::Hook;
use waterui_core::extract::{Extractor, Use};

#[derive(Clone)]
struct Typography {
    uppercase_buttons: bool,
}

struct TypographyPlugin;

impl Plugin for TypographyPlugin {
    fn install(self, env: &mut Environment) {
        env.insert(Typography { uppercase_buttons: true });
        env.insert(Hook::new(|env: &Environment, mut config: TextConfig| {
            let Use(settings) = Extractor::extract::<Use<Typography>>(env).unwrap();
            if settings.uppercase_buttons {
                config.content = config
                    .content
                    .map(|text| text.to_uppercase().into())
                    .into_computed();
            }
            Text::from(config)
        }));
    }
}
```

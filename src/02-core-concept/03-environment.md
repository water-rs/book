# The Environment System

WaterUI's `Environment` is a type-indexed map that flows through your entire view hierarchy. It lets you pass themes, services, and configuration data without manually threading parameters through every function.

## Seeding the Environment

Create an environment at the root of your app and attach values with `.with` (for owned values), `.store` (for namespaced keys), or `.install` (for plugins):

```rust
# use waterui::env::Environment;
# use waterui::prelude::*;
# use waterui::Color;
# #[derive(Clone)]
# pub struct AppConfig {
#     pub api_url: String,
#     pub timeout_seconds: u64,
# }
# #[derive(Clone)]
# pub struct Theme {
#     pub primary_color: Color,
#     pub background_color: Color,
# }
# fn home() -> &'static str { "Home" }
pub fn entry() -> impl View {
    let env = Environment::new()
        .with(AppConfig {
            api_url: "https://api.example.com".into(),
            timeout_seconds: 30,
        })
        .with(Theme {
            primary_color: Color::srgb_f32(0.0, 0.4, 1.0),
            background_color: Color::srgb_f32(1.0, 1.0, 1.0),
        });

    home().with_env(env)
}
```

> `ViewExt::with_env` (available through `prelude::*`) applies the environment to the subtree. You can wrap entire navigators, specific screens, or even single widgets this way.

## Reading Environment Values

### Struct Views

Views that implement `View` directly receive `&Environment` in their `body` method:

```rust
# use waterui::prelude::*;
# use waterui::env::Environment;
# #[derive(Clone)]
# struct AppConfig { api_url: String, timeout_seconds: u64 }
# #[derive(Clone)]
# struct Theme {
#     primary_color: waterui::Color,
#     background_color: waterui::Color,
# }
struct ApiStatusView;

impl View for ApiStatusView {
    fn body(self, env: &Environment) -> impl View {
        let config = env.get::<AppConfig>().expect("AppConfig provided");
        let theme = env.get::<Theme>().expect("Theme provided");

        vstack((
            waterui_text::Text::new(config.api_url.clone())
                .foreground(theme.primary_color.clone()),
            waterui_text::Text::new(format!("Timeout: {}s", config.timeout_seconds))
                .size(14.0),
        ))
        .background(waterui::background::Background::color(
            theme.background_color.clone(),
        ))
    }
}
```

### Function Views with `use_env`

Functions (which already implement `View`) can still access the environment by wrapping their content in `use_env`:

```rust
# use waterui::env::{use_env, Environment};
# use waterui::prelude::*;
# use waterui::Color;
# #[derive(Clone)]
# struct Theme { primary_color: Color }
pub fn themed_button(label: &'static str) -> impl View {
    use_env(move |env: Environment| {
        let theme = env.get::<Theme>().unwrap();
        button(label).background(theme.primary_color.clone())
    })
}
```

### Event Handlers (`action_with`)

Handlers can extract typed values with `waterui::core::extract::Use<T>`:

```rust
# use waterui::prelude::*;
# use waterui::Binding;
# use waterui_core::extract::Use;
# use waterui::reactive::binding;
# #[derive(Clone)]
# pub struct Message(&'static str);
pub fn click_me() -> impl View {
    let value: Binding<String> = binding(String::new());
    vstack((
        button("Read message")
            .action_with(&value, |binding: Binding<String>, Use(Message(text)): Use<Message>| {
                binding.set(text.to_string());
            }),
        text!("{value}"),
    ))
    .with(Message("I'm Lexo"))
}
```

## Namespaced Keys with `Store`

If the same type needs to appear multiple times (e.g., two independent `Theme` structs), wrap them in `Store<K, V>` so the key includes a phantom type:

```rust
# use waterui::env::{Environment, Store};
# use waterui::Color;
# #[derive(Clone)]
# struct Theme { primary_color: Color, background_color: Color }
pub struct AdminTheme;
pub struct UserTheme;

pub fn install_themes() -> Environment {
    Environment::new()
        .store::<AdminTheme, _>(Theme {
            primary_color: Color::srgb(230, 50, 50),
            background_color: Color::srgb(0, 0, 0),
        })
        .store::<UserTheme, _>(Theme {
            primary_color: Color::srgb(0, 102, 255),
            background_color: Color::srgb(255, 255, 255),
        })
}

pub fn maybe_admin_theme(env: &Environment) -> Option<&Theme> {
    env.query::<AdminTheme, Theme>()
}
```

## Plugins and Hooks

Plugins encapsulate environment setup. Implement `Plugin` and call `.install` to register hooks, services, or other values:

```rust
# use waterui::prelude::*;
# use waterui_core::plugin::Plugin;
# use waterui::component::button::ButtonConfig;
# use waterui::AnyView;
# use waterui::layout::stack::hstack;
# use waterui_text::Text;
# use waterui::view::ViewConfiguration;
struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn install(self, env: &mut Environment) {
        env.insert_hook(|_, mut config: ButtonConfig| {
            config.label = AnyView::new(hstack((
                Text::new("🌊"),
                config.label,
            )));
            config.render()
        });
    }
}

let env = Environment::new().install(ThemePlugin);
```

Hooks intercept `ViewConfiguration` types before they render, making the environment the perfect place to implement themes, logging, or feature flags that span your entire view hierarchy.

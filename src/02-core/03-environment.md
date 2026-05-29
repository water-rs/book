# The Environment

> **In this chapter, you will:**
> - Understand how WaterUI's type-indexed dependency injection works
> - Learn to insert, read, and override values that flow through the view tree
> - Use `use_env` and extractors to access shared configuration from any view
> - Build plugins and hooks that customize component rendering globally

Picture this: you are building an app with a dark mode toggle. When the user flips the switch, every single component -- buttons, text labels, backgrounds -- needs to update its colors. You could pass a `theme` parameter to every view function... but that would be tedious and fragile. What if a deeply nested component also needs the current locale, or an API client, or a navigation controller?

The `Environment` solves this. It is WaterUI's type-indexed dependency injection system -- a shared bag of values that flows automatically through the view hierarchy. Any view can read from it, and any view can extend it for its descendants. Think of it like React Context or SwiftUI's `@Environment`, but with Rust's type system as the key.

## How It Works

`Environment` is a type-indexed map -- each unique type can store at most one visible value. When a view's `body()` is called, it receives a reference to the current environment. Child views inherit the parent's environment, and any view can extend it with additional values for its descendants.

Internally, `Environment` is implemented as a structurally-shared overlay chain (an `Rc<EnvironmentState>` linked list backed by a `BTreeMap` at the root). That means cloning an environment is an `Rc` bump, and extending one is an `O(1)` overlay -- no map copy required.

This design gives you:

- **Zero boilerplate**: No keys, strings, or registration ceremonies. The type *is* the key.
- **Automatic scoping**: Values inserted by a parent are visible to all descendants.
- **Override-friendly**: A child can shadow a parent's value for its subtree.
- **Clone-cheap**: `Environment` clones share their backing state through `Rc`.

> **Note:** Since each type can appear at most once, inserting the same type again replaces the previous value. If you need multiple values of the same type (e.g., two different `Color` values), see the `Store` section below.

## Creating and Seeding

### Empty Environment

```rust
let env = Environment::new();
```

### Inserting Values

Use `insert` for imperative insertion. Both `insert` and `with` mutate the environment in place; `with` returns `&mut Self` so successive calls can be chained on a mutable handle:

```rust
let mut env = Environment::new();

// Imperative
env.insert(String::from("hello"));
env.insert(42i32);

// Fluent chaining on a &mut Environment
env.with(String::from("hello"))
   .with(42i32);
```

Since types are keys, each type can appear at most once. Inserting the same type replaces the previous value. To extend a borrowed environment without mutation, use `env.extending(value)`, which returns a fresh `Environment` that overlays the new value on the original.

### The store() Method: Namespaced Keys

What if you need two values of the same type? Use `Store<K, V>` where `K` is a zero-sized marker type. `store` consumes the environment and returns a new one (it is the chainable cousin of `with`):

```rust
use waterui_core::env::Store;

struct PrimaryColor;
struct AccentColor;

let env = Environment::new()
    .store::<PrimaryColor, _>(Color::blue())
    .store::<AccentColor, _>(Color::orange());
```

Now the environment holds two `Color` values distinguished by their marker type. Query them with:

```rust
let primary: Option<&Color> = env.query::<PrimaryColor, Color>();
let accent: Option<&Color> = env.query::<AccentColor, Color>();
```

> **Tip:** `Store` is a lightweight pattern for when you need the same type in multiple roles. The marker types are zero-sized, so they add no runtime overhead.

### Installing Plugins

```rust
let mut env = Environment::new();
env.install(MyThemePlugin);
env.install(LocalizationPlugin::new("en"));
```

The `install` method calls the plugin's `install` method, which can insert multiple values, hooks, or other plugins. We will cover plugins in detail later in this chapter.

## Reading Values

### Direct Lookup

```rust
// Returns Option<&T>
if let Some(theme) = env.get::<MyTheme>() {
    // use theme
}
```

### get_or_insert_with

Lazily initialize a value if it does not exist:

```rust
let config = env.get_or_insert_with(|| AppConfig::default());
```

### Removing Values

```rust
env.remove::<MyTheme>();
```

Now that you know how to seed and query the environment directly, let's see how views interact with it.

## Accessing the Environment from Views

### The use_env Function

`use_env` creates a view that receives extracted values from the environment. This is the primary way your views consume shared configuration:

```rust
use waterui_core::env::use_env;

let view = use_env(|theme: MyTheme| {
    let name = theme.name.clone();
    text!("Current theme: {name}")
        .foreground(theme.text_color)
});
```

The closure parameter must implement the `Extractor` trait (more on this below). If extraction fails (the value is not in the environment), the view panics.

For optional values, wrap the extractor in `Option`:

```rust
let view = use_env(|theme: Option<MyTheme>| {
    match theme {
        Some(theme) => {
            let name = theme.name.clone();
            text!("Theme: {name}").anyview()
        }
        None => text("No theme set").anyview(),
    }
});
```

> **Note:** `text!` accepts only named placeholders captured from scope or aliased explicitly (`text!("Hello, {name}", name = greeting())`). Positional `{}` placeholders are rejected at compile time.

### Tuple Extraction

Extract multiple values at once:

```rust
let view = use_env(|(nav, db): (NavigationController, Database)| {
    let name = db.name();
    text!("Connected to {name}")
});
```

Tuple extractors are implemented for tuples up to 8 elements.

### The ViewExt::with Method

Inject a value into the environment for a view's subtree:

```rust
// All descendants of this view will see MyConfig in their environment
my_view.with(MyConfig { debug: true })
```

This creates a `With<V, T>` wrapper that clones the current environment, inserts the value, and passes the modified environment to the child.

> **Tip:** `.with()` is how you scope configuration to a subtree. For example, you can set a different theme for just the settings page without affecting the rest of the app.

### The ViewExt::install Method

Install a plugin into the environment for a subtree:

```rust
my_view.install(MyPlugin)
```

This is equivalent to `use_env(|mut env: Environment| { env.install(plugin); Metadata::new(view, env) })`.

## Extractor and Use\<T\>

The `Extractor` trait defines how to pull values from an `Environment`:

```rust
pub trait Extractor: 'static + Sized {
    fn extract(env: &Environment) -> Result<Self, Error>;
}
```

### Built-in Extractors

| Type | Behavior |
|------|----------|
| `Environment` | Clones the entire environment |
| `Use<T>` | Looks up `T` in the environment, clones it |
| `Option<T: Extractor>` | Wraps extraction -- returns `None` instead of error |
| `(A, B, ...)` | Extracts each element, up to 8-tuples |

### The Use\<T\> Wrapper

`Use<T>` is the standard way to extract a value. It implements `Deref<Target = T>`:

```rust
use waterui_core::extract::Use;

let view = use_env(|Use(config): Use<AppConfig>| {
    let debug = config.debug;
    text!("Debug: {debug}")
});
```

If the type is not found, extraction returns an error with a clear message describing the missing type.

### Custom Extractors

The `impl_extractor!` macro generates an `Extractor` impl that delegates to `Use<T>`:

```rust
impl_extractor!(MyConfig);

// Now you can write:
let view = use_env(|config: MyConfig| {
    // config is extracted directly, no Use<> wrapper needed
});
```

> **Tip:** Use `impl_extractor!` for types you access frequently. It saves you from writing `Use<MyConfig>` everywhere.

## Metadata\<T\> and IgnorableMetadata\<T\>

Metadata attaches additional rendering instructions to views. There are two kinds, depending on whether the instruction is mandatory or optional.

### Metadata\<T\> (Mandatory)

```rust
pub struct Metadata<T: MetadataKey> {
    pub content: AnyView,
    pub value: T,
}
```

If a renderer encounters `Metadata<T>` and does not handle it, calling `body()` will panic. This ensures critical rendering instructions (e.g., environment overrides, lifecycle hooks) are not silently dropped.

```rust
// Attach mandatory metadata
let view = Metadata::new(my_view, LifeCycleHook::new(LifeCycle::Appear, || {
    tracing::info!("View appeared!");
}));
```

### IgnorableMetadata\<T\> (Optional)

```rust
pub struct IgnorableMetadata<T: MetadataKey> {
    pub content: AnyView,
    pub value: T,
}
```

If a renderer does not handle this metadata, `body()` simply returns the content view -- the metadata is silently discarded. Use this for hints that improve the experience but are not required (e.g., accessibility labels).

```rust
let view = IgnorableMetadata::new(my_view, AccessibilityLabel::new("Submit button"));
```

### MetadataKey

Both metadata types require the value to implement `MetadataKey`:

```rust
pub trait MetadataKey: 'static {}
```

This is a simple marker trait. Implement it for any type you want to attach as metadata.

## Retain: Keeping Guards Alive

When you set up a manual watcher on a signal, the watcher is unsubscribed as soon as its guard is dropped. But view body functions are one-shot -- they return a view and then their local variables go out of scope. `Retain` solves this by keeping an arbitrary value alive for the lifetime of the view:

```rust
pub struct Retain {
    // private fields
}
```

Use it through `ViewExt::retain`:

```rust
fn my_view(data: Binding<String>) -> impl View {
    let guard = data.watch(|ctx| {
        tracing::debug!("Data changed: {}", ctx.into_value());
    });

    text!("Watching data")
        .retain(guard)  // guard lives as long as this view
}
```

Without `.retain()`, the guard would be dropped at the end of the function, immediately unsubscribing the watcher.

You can retain multiple values by chaining or passing a tuple:

```rust
text!("Hello")
    .retain(guard1)
    .retain(guard2)

// Or combine into one retain
text!("Hello")
    .retain((guard1, guard2, some_subscription))
```

> **Warning:** Forgetting to `.retain()` a watcher guard is a common bug. Your watcher will appear to "not work" because it gets unsubscribed immediately. If your side effect never fires, check that you are retaining the guard.

## Hooks: Intercepting View Configuration

Hooks allow global interception of configurable views. This is the mechanism that powers WaterUI's theming system. A `Hook<C>` wraps a function that receives an `Environment` and a `ViewConfiguration`, and returns a custom view:

```rust
pub struct Hook<C>(Box<dyn Fn(&Environment, C) -> AnyView>);
```

Construct one with `Hook::new(|env, config| ...)`, where the closure may return any `impl View` -- the constructor erases it to `AnyView` for you.

### Installing a Hook

```rust
env.insert_hook(|env: &Environment, config: ButtonConfig| {
    // Return a custom view instead of the default button
    custom_button(config.label, config.action)
        .padding()
        .background(Color::blue())
});
```

Or using `Hook::new` directly when you need to keep the hook value around:

```rust
use waterui_core::view::Hook;

let hook: Hook<ButtonConfig> = Hook::new(|env, config: ButtonConfig| {
    my_custom_button(config)
});
env.insert(hook);
```

### How Hooks Work

When a configurable view's `body()` is called:

1. It extracts its configuration via `ConfigurableView::config()`.
2. It checks `env.get::<Hook<Config>>()`.
3. If a hook exists, `hook.apply(env, config)` is called.
4. The hook receives a *modified* environment with itself removed (preventing infinite recursion).
5. If no hook exists, the default native rendering is used.

This is the mechanism behind WaterUI's theming system -- a theme plugin installs hooks for `ButtonConfig`, `ToggleConfig`, `SliderConfig`, etc., replacing the default platform rendering with themed versions.

> **Note:** The hook receives an environment with itself removed. This means if your hook calls `config.render()`, the default rendering will be used for the inner view -- no infinite recursion. This is intentional and allows hooks to wrap the default rendering rather than fully replace it.

## Plugins: The Plugin Trait

Plugins bundle related environment setup into a reusable unit. Instead of manually inserting values, hooks, and configurations, you define a plugin that does it all:

```rust
pub trait Plugin: Sized + 'static {
    fn install(self, env: &mut Environment) {
        env.insert(self);
    }

    fn uninstall(self, env: &mut Environment) {
        env.remove::<Self>();
    }
}
```

The default `install` implementation stores the plugin itself in the environment. Override it to perform custom setup:

```rust
struct DarkThemePlugin;

impl Plugin for DarkThemePlugin {
    fn install(self, env: &mut Environment) {
        // Store self so we can check if installed
        env.insert(self);

        // Install hooks for themed components
        env.insert_hook(|env: &Environment, config: ButtonConfig| {
            dark_themed_button(config)
        });

        env.insert_hook(|env: &Environment, config: ToggleConfig| {
            dark_themed_toggle(config)
        });

        // Store theme colors
        env.insert(ThemeColors::dark());
    }
}
```

Install plugins at the app level or per-subtree:

```rust
// App-wide: install into the root environment before constructing App.
pub fn app(mut env: Environment) -> App {
    env.install(DarkThemePlugin);
    App::new(main, env)
}

// Per-subtree: only affects descendants of `settings_form`.
fn settings_page() -> impl View {
    settings_form()
        .install(HighContrastPlugin)
}
```

## Practical Examples

Let's see the environment in action with some real-world patterns.

### Theming with Environment

```rust
use waterui::prelude::*;

#[derive(Clone, Debug)]
struct AppTheme {
    primary: Color,
    background: Color,
    text: Color,
}

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
    use_env(|theme: AppTheme| {
        text(title)
            .foreground(theme.text)
            .padding()
            .background(theme.background)
    })
}

fn app_root() -> impl View {
    // Inject a static theme into the subtree's environment. Each descendant
    // re-extracts `AppTheme` from the environment via `use_env`.
    vstack((
        themed_card("Welcome"),
        themed_card("Settings"),
    ))
    .with(AppTheme::light())
}
```

> **Tip:** To swap themes reactively, scope two subtrees behind a `when(dark_mode, || ...).otherwise(|| ...)` -- each branch installs the appropriate theme, and the framework rebuilds the relevant subtree when `dark_mode` flips.

### Configuration Injection

```rust
#[derive(Clone, Debug)]
struct ApiConfig {
    base_url: String,
    timeout_ms: u32,
}

impl Plugin for ApiConfig {
    fn install(self, env: &mut Environment) {
        env.insert(self);
    }
}

fn api_status() -> impl View {
    use_env(|config: ApiConfig| {
        let base_url = config.base_url.clone();
        let timeout_ms = config.timeout_ms;
        text!("API: {base_url} ({timeout_ms}ms timeout)")
    })
}

pub fn app(mut env: Environment) -> App {
    env.install(ApiConfig {
        base_url: "https://api.github.com".into(),
        timeout_ms: 5000,
    });
    App::new(main, env)
}
```

### Button Hook Customization

```rust
use waterui::prelude::*;
use waterui::shape::RoundedRectangle;

struct RoundedButtonPlugin;

impl Plugin for RoundedButtonPlugin {
    fn install(self, env: &mut Environment) {
        env.insert(self);
        env.insert_hook(|env: &Environment, config: ButtonConfig| {
            // Re-render the default button, then wrap it in chrome.
            config.render()
                .padding()
                .background(Color::blue())
                .clip(RoundedRectangle::new(0.2))
        });
    }
}

fn my_screen() -> impl View {
    vstack((
        // These buttons render through RoundedButtonPlugin's hook.
        button("Save").action(|| {}),
        button("Cancel").action(|| {}),
    ))
    .install(RoundedButtonPlugin)
}
```

> **Note:** `RoundedRectangle::new` takes a normalized corner radius in `0.0..=0.5`, not points. The `0.2` above produces softly rounded corners regardless of the button size.

Try installing `RoundedButtonPlugin` on just one section of your app and observe how only that subtree's buttons are affected.

## Summary

| Concept | Purpose |
|---------|---------|
| `Environment` | Type-indexed key-value store flowing through the view tree |
| `env.insert()` / `env.with()` | Store a value (type = key) |
| `env.get::<T>()` | Retrieve a value by type |
| `Store<K, V>` | Namespace multiple values of the same type |
| `use_env(closure)` | Create a view that accesses the environment |
| `ViewExt::with(value)` | Inject a value for descendants |
| `Extractor` / `Use<T>` | Type-safe extraction from environment |
| `Metadata<T>` | Mandatory rendering instructions attached to views |
| `IgnorableMetadata<T>` | Optional hints that renderers may discard |
| `Retain` | Keep RAII guards alive for a view's lifetime |
| `Hook<C>` | Intercept and customize configurable view rendering |
| `Plugin` trait | Modular environment extensions with install/uninstall |

With the environment in hand, you have a powerful tool for sharing state across your view tree. The next chapter covers **modifiers** -- the chainable methods that let you style, position, and add behavior to any view.

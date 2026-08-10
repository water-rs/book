# Error handling

> **In this chapter, you will:**
> - Return `Result` and `Option` directly from view code
> - Wrap any `std::error::Error` as a view with `Error`
> - Configure app-wide error presentation with `DefaultErrorView`
> - Shape errors at the call site with `ResultExt::error_view`
> - Scope a different error presentation to one subtree

In a library you propagate errors with `?` until someone handles them. In a UI, "handling" means *rendering*: a network failure has to become a view the user can read and act on. WaterUI does that by making errors views.

Two modules cover it:

- `waterui::widget::error` — `Error`, `DefaultErrorView`, `UseDefaultErrorView`, `ResultExt`. This is the one you configure once and use everywhere.
- `waterui::error` — a smaller `ErrorView` / `ErrorViewBuilder` pair that falls back to plain text when nothing is configured.

## Results and options are views

`Result<V, E>` implements `View` when both `V: View` and `E: View`, and `Option<V>` implements it with `None` rendering as empty. Fallible view functions need no wrapper type:

```rust,ignore
use waterui::prelude::*;

fn user_card() -> impl View {
    match load_user() {
        Ok(user) => text(user.name).anyview(),
        Err(_) => text("Failed to load user").anyview(),
    }
}
```

## The Error type

A string is rarely enough. `waterui::widget::error::Error` wraps any `std::error::Error`, keeps the concrete type recoverable, and renders through whatever the environment says errors should look like:

```rust,ignore
use std::io;
use waterui::widget::error::Error;

let error_view = Error::new(io::Error::new(io::ErrorKind::NotFound, "File not found"));
```

When rendered, it looks up `DefaultErrorView` in the environment. Without one it renders nothing at all — which is why installing one at the root is not optional.

### From a view

When the failure is not a Rust error — a validation state, a "no results" screen — build the presentation directly:

```rust,ignore
use waterui::prelude::*;
use waterui::widget::error::Error;

let custom_error = Error::from_view(vstack((
    text("Something went wrong!"),
    text("Please try again later."),
)));
```

### Recovering the original error

```rust,ignore
use std::io;
use waterui::widget::error::Error;

let error = Error::new(io::Error::new(io::ErrorKind::NotFound, "File not found"));

match error.downcast::<io::Error>() {
    Ok(io_error) => assert_eq!(io_error.kind(), io::ErrorKind::NotFound),
    Err(original) => drop(original),  // not an io::Error; handle generically
}
```

`downcast` returns `Ok(Box<T>)` on a match and gives the `Error` back unchanged on a miss, so a failed downcast costs you nothing.

## DefaultErrorView

`DefaultErrorView` holds a builder from `BoxedStdError` to a view, stored in the environment:

```rust,ignore
use waterui::prelude::*;
use waterui::widget::error::{BoxedStdError, DefaultErrorView};

let env = Environment::new().extending(DefaultErrorView::new(|error: BoxedStdError| {
    let message = Binding::container(error.to_string());
    vstack((
        text!("Error: {message}"),
        text("Please contact support if this persists.")
            .foreground(theme_color::MutedForeground),
    ))
}));
```

`text!` reads named placeholders from the surrounding scope and maps over them as signals, so the message has to be bound to an identifier first.

Note the color: `theme_color::MutedForeground` resolves against the installed theme, so the secondary line stays legible in dark mode. Hard-coding `Color::srgb(128, 128, 128)` here would be a bug on half the platforms.

`Environment::extending` is the by-value, chainable form — it returns a new environment overlaying the value. With `&mut Environment` in hand, `env.insert(value)` and the chainable `env.with(value)` mutate in place.

`UseDefaultErrorView` is the view that performs the lookup. `Error::new` creates one internally; you rarely name it directly.

## The simple module

`waterui::error` is the lighter option: `ErrorView` renders through an `ErrorViewBuilder` if one is installed, and otherwise falls back to the error's `Display` output as plain text.

```rust,ignore
use waterui::error::{ErrorView, ErrorViewBuilder};
use waterui::prelude::*;

let view = ErrorView::from(std::io::Error::new(std::io::ErrorKind::NotFound, "Not found"));

let mut env = Environment::new();
env.insert(ErrorViewBuilder::new(|error, env| {
    text(format!("Error: {error}")).anyview()
}));
```

The builder receives the environment as a second argument and must return `AnyView`. Use this module when a text fallback is genuinely acceptable; use `widget::error` when you want one deliberate presentation everywhere.

## Shaping errors at the call site

`ResultExt::error_view` converts the `Err` variant into an `Error` wrapping a view you supply, leaving `Ok` untouched:

```rust,ignore
use waterui::prelude::*;
use waterui::widget::error::ResultExt;

fn load_data() -> Result<String, std::io::Error> {
    Ok("data".to_string())
}

fn my_view() -> impl View {
    match load_data().error_view(|err| {
        let message = Binding::container(err.to_string());
        text!("Failed to load: {message}")
    }) {
        Ok(data) => text(data).anyview(),
        Err(error_view) => error_view.anyview(),
    }
}
```

Use it when one call site needs a message the global builder cannot produce — a field name, a retry affordance specific to that operation. Everything else should fall through to `Error::new` and stay consistent.

## Errors and async loading

The natural place to resolve both outcomes is inside the suspended body, so the `Suspense` sees a single view either way:

```rust,ignore
use waterui::prelude::*;
use waterui::text::Text;
use waterui::widget::error::Error;
use waterui::widget::suspense::Suspense;

async fn fetch_profile() -> AnyView {
    match api::get_profile().await {
        Ok(profile) => vstack((text(profile.name).headline(), text(profile.bio))).anyview(),
        Err(e) => Error::new(e).anyview(),
    }
}

fn profile_screen() -> impl View {
    Suspense::new(fetch_profile()).loading::<_, Text>(text("Loading profile..."))
}
```

## Scoping a different presentation

`Error` is an ordinary view, so error boundaries follow the view hierarchy. To change the presentation for one subtree, wrap the configuration in a plugin and install it there with `ViewExt::install`:

```rust,ignore
use waterui::prelude::*;
use waterui::widget::error::{BoxedStdError, DefaultErrorView};
use waterui::{Environment, Plugin};

struct TopLevelErrorStyle;

impl Plugin for TopLevelErrorStyle {
    fn install(self, env: &mut Environment) {
        env.insert(DefaultErrorView::new(|error: BoxedStdError| {
            let message = Binding::container(error.to_string());
            vstack((
                text("Application error").headline(),
                text!("{message}"),
                button("Retry").action(|| tracing::info!("retry requested")),
            ))
        }));
    }
}

fn app_shell() -> impl View {
    vstack((header(), content_area())).install(TopLevelErrorStyle)
}
```

`ViewExt::install` clones the environment, runs the plugin against the clone, and attaches it to the subtree, so the outer presentation is untouched. Nesting works the same way: the nearest installed `DefaultErrorView` wins. The [Plugins](07-plugins.md) chapter goes deeper on the pattern.

---

Next: [Accessibility](05-accessibility.md) — because an error message nobody can hear is not handled either.

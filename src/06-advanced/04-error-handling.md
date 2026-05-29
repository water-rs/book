# Error Handling

> **In this chapter, you will:**
> - Understand how `Result` and `Option` work as views in WaterUI
> - Use the `Error` type to render any `std::error::Error` visually
> - Configure app-wide error presentation with `DefaultErrorView`
> - Convert errors to custom views inline with `ResultExt`
> - Build nested error boundaries for different parts of your UI

Errors in UI applications need special treatment. In a traditional Rust program
you propagate errors with `?` until someone handles them. But in a declarative
UI, errors must become *visible* -- they need to render as views that the user
can see and act on. A network failure should not crash your app; it should show
a helpful message with a retry button.

WaterUI provides two complementary modules for this:

- `waterui::widget::error` -- The `Error` type, `DefaultErrorView`,
  `UseDefaultErrorView`, and the `ResultExt` trait.
- `waterui::error` -- A simpler `ErrorView` and `ErrorViewBuilder` for
  environment-based rendering.

## Errors as Views

The key insight is that `Result<V, E>` implements `View` when both `V: View`
and `E: View`. This means you can return a `Result` directly from a view body:

```rust,ignore
use waterui::prelude::*;

fn user_card() -> impl View {
    match load_user() {
        Ok(user) => text(user.name).anyview(),
        Err(_) => text("Failed to load user").anyview(),
    }
}
```

WaterUI also makes `Option<V>` a `View` -- `None` renders as an empty view,
`Some(v)` renders `v`. These two implementations let you write fallible view
functions with minimal boilerplate.

## The Error Type

For real applications, you want more than a string. `waterui::widget::error::Error`
is a type-erased error wrapper that implements `View`. It wraps any
`std::error::Error` and renders it using the environment's configured error
view builder:

```rust,ignore
use waterui::widget::error::Error;

let io_err = std::io::Error::new(
    std::io::ErrorKind::NotFound,
    "File not found",
);
let error_view = Error::new(io_err);
```

When rendered, `Error` looks for a `DefaultErrorView` in the environment.
If one is found, it delegates rendering to that builder. If not, it uses
`UseDefaultErrorView` which falls back to an empty view.

### Creating Errors from Views

You can also create an `Error` directly from a view. This is useful when you
want to present a rich error UI that does not originate from a Rust
`std::error::Error`:

```rust,ignore
use waterui::prelude::*;
use waterui::widget::error::Error;

let custom_error = Error::from_view(vstack((
    text("Something went wrong!"),
    text("Please try again later."),
)));
```

### Type Downcasting

`Error` preserves the original error type and supports downcasting, so you can
recover the specific error when you need to:

```rust,ignore
use waterui::widget::error::Error;
use std::io;

let error = Error::new(
    io::Error::new(io::ErrorKind::NotFound, "File not found")
);

match error.downcast::<io::Error>() {
    Ok(io_error) => {
        // Handle specific IO error
        assert_eq!(io_error.kind(), io::ErrorKind::NotFound);
    }
    Err(original) => {
        // Not an IO error, handle generically
        drop(original);
    }
}
```

## DefaultErrorView

Now let's set up consistent error presentation across your entire app.
`DefaultErrorView` is a configuration type stored in the `Environment`. It
holds a builder function that converts any boxed error into a view:

```rust,ignore
use waterui::prelude::*;
use waterui::widget::error::{BoxedStdError, DefaultErrorView};

let env = Environment::new().extending(DefaultErrorView::new(
    |error: BoxedStdError| {
        let message = Binding::container(error.to_string());
        vstack((
            text!("Error: {message}"),
            text("Please contact support if this persists.")
                .foreground(Color::srgb(128, 128, 128)),
        ))
    },
));
```

`Environment::extending` is the chainable, by-value form: it returns a new
`Environment` that overlays the inserted value on top of the previous state.
Use it for builder-style setup. If you already own `&mut Environment`, the
shorter `env.with(value)` and `env.insert(value)` mutate in place.

Every `Error` view rendered within this environment will use this builder to
present the error. This creates a consistent error appearance throughout your
application.

### UseDefaultErrorView

`UseDefaultErrorView` is the view type that performs the environment lookup. It
queries for `DefaultErrorView` and invokes the builder:

```rust,ignore
use waterui::widget::error::UseDefaultErrorView;

let view = UseDefaultErrorView::new(some_error);
// Renders using the DefaultErrorView from the environment,
// or renders empty if none is configured.
```

In practice, you rarely use `UseDefaultErrorView` directly -- `Error::new`
creates one internally.

## The ErrorView (Simple Module)

The `waterui::error` module provides a simpler alternative when you want a
quick error display without the full `DefaultErrorView` machinery:

```rust,ignore
use waterui::error::ErrorView;

let view = ErrorView::from(
    std::io::Error::new(std::io::ErrorKind::NotFound, "Not found")
);
```

If an `ErrorViewBuilder` is present in the environment, it is used for
rendering. Otherwise, `ErrorView` falls back to rendering the error message as
plain text:

```rust,ignore
use waterui::error::ErrorViewBuilder;
use waterui::prelude::*;

let builder = ErrorViewBuilder::new(|error, env| {
    text(format!("Error: {error}")).anyview()
});

let mut env = Environment::new();
env.insert(builder);
```

## The ResultExt Trait

`ResultExt` adds the `.error_view()` method to any `Result`, letting you
convert errors to custom views inline. This is particularly useful when
different call sites need different error presentations:

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

`.error_view()` transforms the `Err` variant into an `Error` that wraps the
view you provide. The `Ok` variant passes through unchanged.

## Pairing Error Handling with Suspense

Errors and async loading go hand-in-hand. Here is a pattern that combines
`Suspense` with `Error` for a complete loading-and-error experience:

```rust,ignore
use waterui::prelude::*;
use waterui::widget::suspense::Suspense;
use waterui::widget::error::Error;

async fn fetch_profile() -> impl View {
    match api::get_profile().await {
        Ok(profile) => vstack((
            text(profile.name).headline(),
            text(profile.bio),
        )).anyview(),
        Err(e) => Error::new(e).anyview(),
    }
}

fn profile_screen() -> impl View {
    Suspense::new(fetch_profile())
        .loading(text("Loading profile..."))
}
```

When the async operation fails, the error renders using your application's
`DefaultErrorView`. When it succeeds, the profile content appears.

## Nested Error Boundaries

Because `Error` renders as a regular view, error boundaries compose naturally
with the view hierarchy. To override `DefaultErrorView` for a subtree, wrap
the configuration in a small plugin and install it with `ViewExt::install`:

```rust,ignore
use waterui::prelude::*;
use waterui::widget::error::{BoxedStdError, DefaultErrorView};
use waterui_core::{Environment, plugin::Plugin};

struct TopLevelErrorStyle;

impl Plugin for TopLevelErrorStyle {
    fn install(self, env: &mut Environment) {
        env.insert(DefaultErrorView::new(|error: BoxedStdError| {
            vstack((
                text("Application Error").headline(),
                text(error.to_string()),
                button("Retry").action(|| { /* retry logic */ }),
            ))
        }));
    }
}

fn app_shell() -> impl View {
    vstack((
        header(),
        content_area(),
    )).install(TopLevelErrorStyle)
}
```

Different parts of the view tree can install different `DefaultErrorView`
plugins to customize error presentation per-section.

## Best Practices

1. **Always install a `DefaultErrorView`** in your root environment. This
   ensures that any uncaught error has a visible representation rather than
   rendering as an empty view.

2. **Use `.error_view()` for localized error handling** when a specific call
   site needs a custom error presentation.

3. **Use `Error::from_view()` for rich error UIs** that include retry buttons,
   contact links, or contextual information.

4. **Prefer `Error::new()` over `Error::from_view()`** when you want
   consistent, centralized error styling from `DefaultErrorView`.

5. **Combine with Suspense** for async operations that can fail. The
   `SuspendedView` body is the natural place to handle both success and error
   cases.

## Summary

| API | Purpose |
|---|---|
| `Error::new(e)` | Wrap any `std::error::Error` as a view |
| `Error::from_view(view)` | Create an error from a custom view |
| `Error::downcast::<T>()` | Recover the original error type |
| `DefaultErrorView::new(builder)` | App-wide error view configuration |
| `UseDefaultErrorView::new(e)` | Render using the environment's error builder |
| `ErrorView::from(e)` | Simple error-to-view (with text fallback) |
| `ErrorViewBuilder::new(f)` | Custom error renderer for the simple module |
| `ResultExt::error_view(f)` | Convert `Result::Err` to a custom view |
| `Result<V, E>: View` | Built-in: results render as views |
| `Option<V>: View` | Built-in: `None` renders empty |

## What's Next

Your app handles errors gracefully, but does it work for *everyone*? In the
[next chapter](05-accessibility.md), you will learn how to make your WaterUI
application accessible to users who rely on screen readers, keyboard navigation,
and other assistive technologies.

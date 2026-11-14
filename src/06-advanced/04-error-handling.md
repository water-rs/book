# Error handling

> **In this chapter, you will:**
> - Tell the three things called "error" in WaterUI apart
> - Return `Result` and `Option` from view code, and turn any `std::error::Error` into a view with `widget::error::Error`
> - Configure error presentation for the whole app or one subtree with `DefaultErrorView`
> - Shape one call site's error with `ResultExt::error_view`
> - Report a broken contract with `contract_violation!`

In a library you propagate errors with `?` until someone handles them. In a UI, "handling" means *rendering*: a network failure has to become a view the user can read and act on. WaterUI does that by making errors views.

## Which "error" do you mean?

Three different types are reachable, and they are easy to confuse:

| Path | What it is | Is it a `View`? |
| --- | --- | --- |
| `waterui::Error` | Re-export of `anyhow::Error` from `waterui-core`. The error type of `Extractor::extract`; a handler whose parameter cannot be extracted fails with `waterui::handler::ExtractionError`, which carries it as the cause. | No |
| `waterui::widget::error::Error` | A type-erased error that renders itself through the environment's `DefaultErrorView`. Built with `Error::new` or `Error::from_view`. | Yes |
| `waterui::error::ErrorView` | A view built from any `std::error::Error`; renders through an `ErrorViewBuilder` in the environment, or as plain text. | Yes |

Two modules therefore carry an `ErrorView` and an `ErrorViewBuilder`, and they are different types:

- **`waterui::widget::error`** exports `Error`, `ErrorView` (a private-constructor wrapper that `Error::from_view` uses internally), `ErrorViewBuilder` (a type alias for `Box<dyn Fn(BoxedStdError) -> AnyView>`), `DefaultErrorView`, `UseDefaultErrorView`, `ResultExt` and `BoxedStdError`. This is the path you configure once and use everywhere, and every example below uses it unless it says otherwise.
- **`waterui::error`** exports a different `ErrorView` struct and a different `ErrorViewBuilder` struct. This is the lighter path, shown near the end of the chapter. The prelude brings the module `error` into scope, so `error::ErrorView` resolves to this one.

When a snippet names `waterui::widget::error` or `waterui::error` in its `use` line, that is the path it demonstrates.

## Results and options are views

`Result<V, E>` implements `View` when both `V: View` and `E: View`, and `Option<V>` implements it with `None` rendering as empty. A fallible view function whose error is already a view needs no wrapper type:

```rust
use waterui::prelude::*;

fn lookup(name: &str) -> Result<Text, Text> {
    if name.is_empty() {
        Err(text("Enter a name"))
    } else {
        Ok(text(name.to_string()))
    }
}

fn greeting() -> impl View {
    lookup("Ada")
}
```

A raw `std::error::Error` is not a view, so `Result<Text, io::Error>` is not one either. That gap is what the rest of the chapter fills.

## The Error type

`waterui::widget::error::Error` wraps any `std::error::Error`, keeps the concrete type recoverable, and renders through whatever the environment says errors should look like:

```rust
use std::io;
use waterui::widget::error::Error;

fn missing_file() -> Error {
    Error::new(io::Error::new(io::ErrorKind::NotFound, "File not found"))
}
```

Because `Error` is a view, `Result<V, Error>` is one too whenever `V` is, so a fallible view function can return it directly.

When rendered, `Error` builds a `UseDefaultErrorView`, which looks up `DefaultErrorView` in the environment. **Without one it renders nothing at all**, which is why installing one at the root is not optional for an app that can fail.

### From a view

When the failure is not a Rust error, such as a validation state or a "no results" screen, build the presentation directly:

```rust
use waterui::prelude::*;
use waterui::widget::error::Error;

fn no_results() -> Error {
    Error::from_view(vstack((
        text("Something went wrong!"),
        text("Please try again later."),
    )))
}
```

An `Error` made with `from_view` ignores `DefaultErrorView`; the view you gave it is the presentation.

### Recovering the original error

```rust
use std::io;
use waterui::widget::error::Error;

fn is_not_found(error: Error) -> bool {
    match error.downcast::<io::Error>() {
        Ok(io_error) => io_error.kind() == io::ErrorKind::NotFound,
        Err(_original) => false,
    }
}
```

`downcast` returns `Ok(Box<T>)` on a match and hands the `Error` back unchanged on a miss, so a failed downcast costs you nothing.

## DefaultErrorView

`DefaultErrorView` holds a builder from `BoxedStdError` (an alias for `Box<dyn std::error::Error>`) to a view. Insert it into the environment your `app` function receives, and every window inherits it:

```rust
use waterui::app::App;
use waterui::prelude::*;
use waterui::widget::error::{BoxedStdError, DefaultErrorView};

fn error_presentation() -> DefaultErrorView {
    DefaultErrorView::new(|error: BoxedStdError| {
        let message = error.to_string();
        vstack((
            text!("Error: {message}"),
            text("Please contact support if this persists.")
                .caption()
                .muted(),
        ))
    })
}

fn root() -> impl View {
    text("Home")
}

pub fn app(mut env: Environment) -> App {
    env.insert(error_presentation());
    App::new(root, env)
}
```

`text!` takes its placeholders from identifiers in scope, so the message is bound to `message` first. A plain `String` is a constant signal, so it needs no `Binding`.

`.muted()` renders the secondary line in the theme's `MutedForeground` token, which stays legible in dark mode where a hard-coded gray would not.

When you build an environment yourself rather than receiving one, `Environment::extending(value)` is the by-value form: it returns a new environment that overlays the value.

`UseDefaultErrorView` is the view that performs the lookup. `Error::new` creates one internally; you rarely name it directly.

## Shaping errors at the call site

`ResultExt::error_view` converts the `Err` variant into an `Error` wrapping a view you supply, leaving `Ok` untouched. It needs `E: Debug + Display + 'static`:

```rust
use waterui::prelude::*;
use waterui::widget::error::ResultExt;

fn load_data() -> Result<String, std::io::Error> {
    Ok("data".to_string())
}

fn my_view() -> impl View {
    load_data()
        .error_view(|err| {
            let message = err.to_string();
            text!("Failed to load: {message}")
        })
        .map(text)
}
```

The result is a `Result<Text, Error>`, which is already a view, so no `match` or `AnyView` is needed.

Use it when one call site needs a message the global builder cannot produce, such as a field name or a retry affordance specific to that operation. Everything else should fall through to `Error::new` and stay consistent.

## Errors and async loading

The natural place to resolve both outcomes is inside the suspended future: it resolves to a `Result<_, Error>`, which is a view either way, and `?` works as usual. See [Suspense](03-suspense.md) for the loading side.

```rust,ignore
use waterui::prelude::*;
use waterui::widget::error::Error;
use waterui::widget::Suspense;

async fn profile() -> Result<impl View, Error> {
    let profile = api::get_profile().await.map_err(Error::new)?;
    Ok(vstack((text(profile.name), text(profile.bio))))
}

fn profile_screen() -> impl View {
    Suspense::new(profile()).loading(text("Loading profile..."))
}
```

This is a fragment: `api::get_profile` stands for your own fallible client, whose error type implements `std::error::Error`.

## Scoping a different presentation

`Error` is an ordinary view, so error boundaries follow the view hierarchy. To change the presentation for one subtree, attach a different `DefaultErrorView` there with `ViewExt::with`:

```rust
use waterui::prelude::*;
use waterui::widget::error::{BoxedStdError, DefaultErrorView};

fn settings_panel(content: impl View) -> impl View {
    vstack((text("Settings").headline(), content)).with(DefaultErrorView::new(
        |error: BoxedStdError| {
            let message = error.to_string();
            vstack((
                text("Settings could not be loaded").headline(),
                text!("{message}"),
            ))
        },
    ))
}
```

`.with(value)` overlays the value on the environment of that subtree only, so the app-wide presentation outside it is untouched. Nesting works the same way: the nearest `DefaultErrorView` wins. When a subtree needs several values configured together, bundle them in a `Plugin` and apply it with `ViewExt::install`; the [Plugins](07-plugins.md) chapter covers that pattern.

## The lighter path: `waterui::error`

`waterui::error::ErrorView` is built from any `std::error::Error` with `From`. It renders through a `waterui::error::ErrorViewBuilder` if one is installed and otherwise falls back to the error's `Display` output as plain text, so it is never blank:

```rust
use waterui::error::{ErrorView, ErrorViewBuilder};
use waterui::prelude::*;

fn not_found() -> ErrorView {
    ErrorView::from(std::io::Error::new(std::io::ErrorKind::NotFound, "Not found"))
}

fn missing_page() -> impl View {
    not_found().with(ErrorViewBuilder::new(|error, _env| {
        let message = error.to_string();
        text!("Error: {message}").anyview()
    }))
}
```

The builder receives the boxed error and the environment, and must return `AnyView`. Like `DefaultErrorView`, it goes into the app environment with `env.insert` or onto a subtree with `.with`. This path has no `downcast` and no `from_view`; use it when a text fallback is genuinely acceptable, and use `widget::error` when you want one deliberate presentation everywhere. The two builders live under different environment keys, so installing one does not configure the other.

## Contract violations

Not every failure is something to show a user. A *contract violation* is a wrong state that app code, a platform or external input can hand to code that must not crash, such as an out-of-range value passed to a component. The framework reports these with `contract_violation!`, which is re-exported as `waterui::contract_violation`:

- in a **debug build** it panics with the message, so the bug is found in development;
- in a **release build** it logs the message through `tracing::error!` and the caller continues to a defined outcome, so a shipped app does not crash.

The build mode is that of the crate that invokes the macro, and your crate needs no direct `tracing` dependency for it.

```rust
use waterui::contract_violation;

/// Opacity is a fraction. Out of range is a caller bug; the defined outcome
/// in a release build is the nearest valid fraction.
fn opacity(value: f32) -> f32 {
    if !(0.0..=1.0).contains(&value) {
        contract_violation!("opacity {value} is outside 0.0..=1.0");
        return value.clamp(0.0, 1.0);
    }
    value
}
```

Use it for states your own code can be handed wrongly. A failure that is a normal outcome, such as a missing file or a dropped connection, is a `Result` and belongs in the view as described above. A state that cannot occur by construction keeps `unreachable!` or `expect`. WaterUI reports its own contract violations the same way: a button whose handler asks for a value missing from the environment does not run, and the resulting `ExtractionError` panics in a debug build and is logged in a release build.

Diagnostics go through `tracing`, which the facade re-exports as `waterui::log` (`waterui::log::debug!`, `waterui::log::error!`), never `println!`; `water run --logs debug` shows the debug-level output.

---

Next: [Accessibility](05-accessibility.md), because an error message nobody can hear is not handled either.

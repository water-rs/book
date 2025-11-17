# Error Handling

WaterUI does not force a bespoke error type. Instead, it lets you turn any `std::error::Error` into
a renderable view via `waterui::error::ErrorView`, and customize how errors look using environment
hooks.

## From Errors to Views

Wrap errors with `ErrorView::from` whenever a `Result` fails:

```rust
use waterui::prelude::*;
use waterui::error::ErrorView;

fn user_profile(id: u64) -> impl View {
    match load_user(id) {
        Ok(user) => profile_card(user),
        Err(err) => ErrorView::from(err),
    }
}
```

`ErrorView` implements `View`, so you can drop it anywhere inside a stack or navigation view. The
default renderer simply prints the error via `Display`.

## Customizing the Presentation

Inject a `ErrorViewBuilder` into the environment to override how errors render globally:

```rust
use waterui::error::ErrorViewBuilder;

let env = Environment::new().with(ErrorViewBuilder::new(|err, _env| {
    AnyView::new(
        vstack((
            text!("⚠️ Something went wrong").bold(),
            text!("{err}").foreground(Color::srgb(200, 80, 80)),
            button("Retry").action(|| task(retry_last_request())),
        ))
        .padding()
    )
}));

app_root().with_env(env)
```

Now every `ErrorView` produced inside the tree uses this layout automatically.

## Inline Result Helpers

If you prefer chaining, build a helper that maps `Result<T, E>` into either a view or an
`ErrorView`:

```rust
fn result_view<T, E>(result: Result<T, E>, render: impl FnOnce(T) -> AnyView) -> AnyView
where
    E: std::error::Error + 'static,
{
    match result {
        Ok(value) => render(value),
        Err(err) => AnyView::new(ErrorView::from(err)),
    }
}
```

Use it when composing lists or complex layouts so you do not repeat `match` expressions everywhere.

## Contextual Actions

Because error builders receive the `Environment`, you can extract services (analytics, retry
queues, offline caches) with `extract::Use<T>` just like button handlers. A typical pattern:

```rust
ErrorViewBuilder::new(|err, env| {
    let telemetry = env.get::<Telemetry>().cloned();
    if let Some(t) = telemetry {
        t.record_error(&err);
    }
    AnyView::new(text!("{err}"))
})
```

## Pairing with Suspense

When fetching data asynchronously, wrap the result inside `Suspense` and convert failures into
`ErrorView` instances. Users get a consistent loading/error pipeline without sprinkling `Result`
logic throughout the UI.

Consistent, informative error displays keep apps trustworthy. Centralize styling via
`ErrorViewBuilder` and lean on `ErrorView::from` wherever fallible operations occur.

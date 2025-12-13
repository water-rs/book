# Error Handling

WaterUI does not force a bespoke error type. Instead, it lets you turn any `std::error::Error` into
a renderable view via `waterui::widget::error::Error`, and customize how errors look using environment
hooks.

## From Errors to Views

Wrap errors with `Error::new` whenever a `Result` fails:

```rust
use waterui::prelude::*;
use waterui::widget::error::Error;

fn user_profile(id: u64) -> impl View {
    match load_user(id) {
        Ok(user) => profile_card(user),
        Err(err) => Error::new(err).anyview(),
    }
}
```

`Error` implements `View`, so you can drop it anywhere inside a stack or navigation view. The
default renderer simply prints the error via `Display`.

## Customizing the Presentation

Inject a `DefaultErrorView` into the environment to override how errors render globally:

```rust
use waterui::widget::error::DefaultErrorView;

let env = Environment::new().with(DefaultErrorView::new(|err| {
    vstack((
        text!("⚠️ Something went wrong").bold(),
        text!("{err}").foreground(Color::srgb(200, 80, 80)),
        button("Retry").action(|| task(retry_last_request())),
    ))
    .padding()
}));

app_root().with_env(env)
```

Now every `Error` produced inside the tree uses this layout automatically.

## Inline Result Helpers

If you prefer chaining, use the `error_view` helper to map `Result<T, E>` into either a view or an
`Error`:

```rust
use waterui::widget::error::ResultExt;

fn result_view<T, E>(result: Result<T, E>, render: impl FnOnce(T) -> AnyView) -> AnyView
where
    E: std::error::Error + 'static,
{
    match result {
        Ok(value) => render(value),
        Err(err) => Error::new(err).anyview(),
    }
}
```

Use it when composing lists or complex layouts so you do not repeat `match` expressions everywhere.

## Contextual Actions

Because error builders receive the `Environment` (if you use a custom view that captures it), you can extract services. Or, use `use_env` inside your error view builder:

```rust
DefaultErrorView::new(|err| {
    use_env(move |env: &Environment| {
       let telemetry = env.get::<Telemetry>().cloned();
        if let Some(t) = telemetry {
            t.record_error(&err);
        }
        text!("{err}")
    })
})
```

## Pairing with Suspense

When fetching data asynchronously, wrap the result inside `Suspense` and convert failures into
`Error` instances. Users get a consistent loading/error pipeline without sprinkling `Result`
logic throughout the UI.

Consistent, informative error displays keep apps trustworthy. Centralize styling via
`DefaultErrorView` and lean on `Error::new` wherever fallible operations occur.
# Fetching Data

Most apps talk to the network. WaterUI doesn’t impose a networking stack—you can use `reqwest`,
`surf`, or any HTTP client—but it provides ergonomic bridges between async tasks and reactive
bindings so UI updates remain declarative.

## Wiring Requests into Bindings

The pattern looks like this:

1. Store view state in bindings (`Binding<Option<T>>`, `Binding<FetchState>`).
2. Trigger an async task from a button, `on_appear`, or watcher.
3. Update the binding once the request resolves; the UI reacts automatically.

```rust
use waterui::prelude::*;
use waterui::reactive::{binding, Binding};
use waterui::task::task;

#[derive(Clone, Debug)]
enum FetchState<T> {
    Idle,
    Loading,
    Loaded(T),
    Failed(String),
}

pub fn weather_card() -> impl View {
    let state: Binding<FetchState<Weather>> = binding(FetchState::Idle);

    vstack((
        text!("Weather"),
        content(state.clone()),
        button("Refresh").action(move || fetch_weather(state.clone())),
    ))
}

fn content(state: Binding<FetchState<Weather>>) -> impl View {
    match state.get() {
        FetchState::Idle => text("Tap refresh"),
        FetchState::Loading => text("Loading…"),
        FetchState::Loaded(ref data) => text!("{}°C", data.temperature),
        FetchState::Failed(ref err) => text!("Error: {err}"),
    }
}

fn fetch_weather(state: Binding<FetchState<Weather>>) {
    state.set(FetchState::Loading);

    task(async move {
        match reqwest::get("https://api.example.com/weather").await {
            Ok(response) => match response.json::<Weather>().await {
                Ok(weather) => state.set(FetchState::Loaded(weather)),
                Err(err) => state.set(FetchState::Failed(err.to_string())),
            },
            Err(err) => state.set(FetchState::Failed(err.to_string())),
        }
    });
}
```

`task` uses the executor configured for your backend (Tokio by default). Because bindings are
clonable handles, you can move them into the async block safely.

## Caching and Suspense

Wrap network bindings in `Computed<Option<T>>` or `Suspense` for placeholder states:

```rust
use waterui::widget::suspense::Suspense;

Suspense::new(
    state.map(|state| matches!(state, FetchState::Loaded(_))),
    || content(state.clone()),
    || text("Loading…"),
);
```

## Error Handling Patterns

- **Retry buttons** – When the binding holds `Failed`, show a retry `button` next to the error text.
- **Timeouts** – Combine `tokio::time::timeout` with the `reqwest` future and set the binding to a
  descriptive error message.
- **Offline mode** – Mirror network results into a persistent store (e.g., SQLite) and hydrate the
  binding immediately on launch; fire background tasks to refresh when the network becomes
  available.

## Platform Constraints

Backend targets run inside their respective sandboxes:

- **Apple / iOS** – Requests require ATS-compliant endpoints (https by default). Update your
  Xcode-generated manifest if you need exceptions.
- **Android** – Remember to add network permissions to the generated `AndroidManifest.xml`.
- **Web** – Consider CORS. Fetching from the browser requires appropriate headers from the server.

WaterUI stays out of the way—you bring the HTTP client—but the combination of bindings and tasks
keeps the state management predictable and testable.

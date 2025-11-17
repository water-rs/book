# Suspense and Async Rendering

`waterui::widget::suspense::Suspense` bridges async work and declarative views. Wrap any async
builder and Suspense takes care of launching the future, showing a placeholder, cancelling the task
when the view disappears, and re-rendering once data arrives.

## Basic Usage

```rust
use waterui::prelude::*;
use waterui::widget::suspense::Suspense;

async fn load_profile() -> impl View {
    text("Ada Lovelace")
}

pub fn profile() -> impl View {
    Suspense::new(load_profile())
        .loading(|| text("Loading profile…"))
}
```

- `Suspense::new(future)` starts the future on first render.
- `.loading(|| …)` provides a custom placeholder. If omitted, Suspense looks for
  `DefaultLoadingView` in the environment.
- The async builder receives a cloned `Environment`, so it can read services or locale info during
  loading.

## Default Loading View

Install a global placeholder so every Suspense shares the same skeleton:

```rust
use waterui::widget::suspense::DefaultLoadingView;

let env = Environment::new().with(DefaultLoadingView::new(|| text("Please wait…")));
```

Wrap your root view with `.with_env(env)` and all suspense boundaries inherit it.

## Caching Results

Suspense rebuilds the future whenever the view is reconstructed. Cache results in bindings if you
need persistence:

```rust
let user = binding(None);

fn user_view(user: Binding<Option<User>>) -> impl View {
    when(user.map(|u| u.is_some()), || profile_body(user.clone()))
        .or(|| Suspense::new(fetch_user(user.clone())))
}
```

`fetch_user` updates the binding when the network request resolves; the `when` branch takes over and
the placeholder disappears.

## Error Handling

Wrap async results in `Result` and convert errors into `ErrorView`:

```rust
use waterui::error::ErrorView;

Suspense::new(async {
    match fetch_feed().await {
        Ok(posts) => feed(posts).anyview(),
        Err(err) => ErrorView::from(err).anyview(),
    }
})
```

You can also stack Suspense boundaries—an outer one fetching user data, an inner one fetching
related content—to keep parts of the UI responsive.

## Cancellation and Restart

Dropping a Suspense (e.g., navigating away) cancels the running future. Re-rendering recreates the
future from scratch. Use this to restart long-polling or subscribe/unsubscribe from streams based on
visibility.

Suspense keeps async ergonomic: describe loading and loaded states declaratively, and let WaterUI
handle the lifecycles.

# Suspense and async views

> **In this chapter, you will:**
> - Show a placeholder while a future runs, then swap in the view it produces
> - Customize the loading view per instance and app-wide
> - Implement `SuspendedView` to reach the environment from inside the future
> - Choose between `Suspense`, `ViewExt::task`, `action_async` and `spawn_local` by what should cancel the work
> - Reload suspended content when its input changes, and know how `water preview` waits for it

A view body is synchronous, but data usually is not. `Suspense` bridges the two: it renders a loading view immediately, spawns your future on the local executor, and replaces the placeholder with the resolved view when the future completes.

## The Suspense component

`Suspense` lives in `waterui::widget::suspense`. The prelude exports the `suspense()` shorthand but not the struct, so import `Suspense` when you want to call `.loading()` on it.

```rust
use std::time::Duration;
use waterui::prelude::*;
use waterui::task::sleep;
use waterui::widget::suspense::Suspense;

async fn fetch_greeting() -> impl View {
    sleep(Duration::from_millis(800)).await;
    text("Loaded")
}

fn greeting() -> impl View {
    Suspense::new(fetch_greeting())
}

fn greeting_shorthand() -> impl View {
    // `suspense` is `Suspense::new`.
    suspense(fetch_greeting())
}
```

Any `Future<Output = V>` with `V: View` and `'static` implements `SuspendedView`, so a plain `async fn` needs no glue. `waterui::task::sleep` is the executor's async timer; never use `std::thread::sleep` in a future, because futures run on the UI thread.

Internally `Suspense` creates a `Dynamic` node, sets the loading view into it, spawns the future with `spawn_local`, and sets the resolved view when it completes. The swap replaces that subtree on purpose: the placeholder and the content are different views.

## Loading views

### Per instance

`.loading(view)` overrides the placeholder for one `Suspense`. It is declared as `loading<Loading2: View>(self, loading: Loading2) -> Suspense<V, Loading2>`, so any view works and no type annotation is needed:

```rust
use std::time::Duration;
use waterui::prelude::*;
use waterui::task::sleep;
use waterui::widget::suspense::Suspense;

async fn fetch_data() -> impl View {
    sleep(Duration::from_millis(800)).await;
    text("Data loaded")
}

fn data_panel() -> impl View {
    Suspense::new(fetch_data()).loading(text("Loading data..."))
}
```

### App-wide

Install a `DefaultLoadingView` in the environment and every `Suspense` without an explicit `.loading()` renders it. `DefaultLoadingView::new` takes any `ViewBuilder`, which includes a `Fn() -> impl View` closure:

```rust
use waterui::prelude::*;
use waterui::widget::suspense::DefaultLoadingView;

fn install_loading_view(env: &mut Environment) {
    env.insert(DefaultLoadingView::new(|| {
        vstack((loading(), text("Please wait...")))
    }));
}
```

`loading()` is the platform's indeterminate loading indicator, a `Progress` in its `Loading` style. The framework installs no `DefaultLoadingView` of its own: without one in scope, `Suspense::new` renders an empty view while loading, so install one at the root and no async screen is ever blank. `DefaultLoadingView::from_builder` accepts an already-erased `AnyViewBuilder`. See [the Environment chapter](../02-core/03-environment.md) for where an app builds its `Environment`, and for `.with(...)`, which scopes a different default to one subtree.

`UseDefaultLoadingView` is the sentinel that performs the lookup. `Suspense::new` already uses it; name it only when you need to write the type out:

```rust
use waterui::prelude::*;
use waterui::widget::suspense::{Suspense, UseDefaultLoadingView};

async fn content() -> impl View {
    text("Content")
}

fn explicit_default() -> impl View {
    // Identical to `Suspense::new(content())`.
    Suspense::new(content()).loading(UseDefaultLoadingView)
}
```

## Implementing SuspendedView

```rust,ignore
pub trait SuspendedView: 'static {
    fn body(self, _env: Environment) -> impl Future<Output = impl View>;
}
```

Implement it directly when the future needs environment services: an API client, configuration, a locale. `Suspense` clones the environment when it mounts and hands the clone to `body`, so everything installed above it is reachable:

```rust
use waterui::log::warn;
use waterui::prelude::*;
use waterui::widget::suspense::{SuspendedView, Suspense};

/// An environment entry the app installs at its root.
struct Endpoint(Str);

struct UserLoader {
    user_id: u32,
}

impl SuspendedView for UserLoader {
    async fn body(self, env: Environment) -> impl View {
        let Some(endpoint) = env.get::<Endpoint>() else {
            warn!(user_id = self.user_id, "no Endpoint in the environment");
            return text("Service unavailable").anyview();
        };
        // A real loader would await an HTTP call against the endpoint here.
        text(format!("User {} from {}", self.user_id, endpoint.0)).anyview()
    }
}

fn profile() -> impl View {
    Suspense::new(UserLoader { user_id: 42 })
}
```

A future that needs no environment does not need the trait at all; the blanket implementation covers it.

## Failures are views too

`Result<V, E>` implements `View` when both sides do, so a fallible load can resolve to the `Result` itself. `waterui::widget::error::Error` wraps any `std::error::Error + 'static` and renders through the `DefaultErrorView` in the environment:

```rust
use waterui::prelude::*;
use waterui::widget::error::Error;
use waterui::widget::suspense::Suspense;

async fn fetch_status() -> Result<String, std::io::Error> {
    Err(std::io::Error::other("server unreachable"))
}

async fn status_view() -> Result<Text, Error> {
    let status = fetch_status().await.map_err(Error::new)?;
    Ok(text(status))
}

fn status() -> impl View {
    Suspense::new(status_view())
}
```

A failed load then looks like every other failure in the app. Like the loading view, the error view is yours to install: with no `DefaultErrorView` in scope, an `Error` renders as an empty view. [Error handling](04-error-handling.md) covers how to shape it.

## Suspense, `.task()`, `action_async` or `spawn_local`?

They differ in what ends the work.

- **`Suspense` detaches its task.** The future runs to completion even if the user navigates away before it resolves. That is fine for a read and wrong for anything with side effects.
- **`ViewExt::task(future)`** takes a `Future<Output = ()>`, spawns it on the local executor at once and retains the handle on the view (`Metadata<Retain>`). Dropping the view drops the handle, and dropping the handle cancels the future. Use it when the view already exists and the async work only fills in reactive state.
- **`Button::action_async(handler)`** takes a handler that returns a future, with the same extractors as `.action()`. Each press spawns the future on the local executor and detaches it, so it outlives the button.
- **`waterui::task::spawn_local`** is the free-standing form, for a handler that has no async variant. The returned handle cancels the future when dropped, so a fire-and-forget task must end in `.detach()` or it dies immediately. That pair is exactly what `action_async` does for you.

```rust
use std::time::Duration;
use waterui::prelude::*;
use waterui::task::sleep;

fn status_view() -> impl View {
    let status: Binding<Str> = binding("Loading...");
    let sink = status.clone();

    // Cancelled when this view is dropped.
    text!("{status}").task(async move {
        sleep(Duration::from_millis(500)).await;
        sink.set(Str::from("Ready"));
    })
}

fn refresh_button() -> impl View {
    let status: Binding<Str> = binding("Idle");

    vstack((
        text!("{status}"),
        button("Refresh")
            // Detached: keeps running if the button goes away.
            .action_async(|State(status): State<Binding<Str>>| async move {
                status.set(Str::from("Refreshing..."));
                sleep(Duration::from_millis(500)).await;
                status.set(Str::from("Done"));
            })
            .state(&status),
    ))
}
```

Reach for `Suspense` when the placeholder is a different view from the result, `.task()` when the work belongs to a view that is already on screen, and `action_async`, or `spawn_local(..).detach()` from another handler, when a user action starts work that should outlive the view that started it.

Two limits apply to all of them. Futures run on the UI thread's local executor, so they may hold non-`Send` state such as a `Binding` but must never block. CPU-bound work goes to `waterui::task::spawn`, which runs a `Send` future on a background executor; a `Binding` is not `Send`, so await the handle `spawn` returns from a local task and write the result there. And the local executor is installed only when the backend mounts the app's view, so calling `spawn_local` from `main` or while building a store panics with "Local executor not set"; because `.task()` spawns the moment it is called, the same holds for `.task()` called outside a view function. `Suspense` spawns from its body, `.task()` from the view function that calls it, and handlers when an event fires, all after mount, which is why they are the right places to start async work. [The reactive chapter](../02-core/02-reactive.md) covers the bindings these tasks write to.

## Reloading when the input changes

`Suspense` resolves once. When the input identity changes, a different user id or a different document, the correct result is a new `Suspense` instance, placeholder included. That is one of the few cases where `watch` is the right tool:

```rust
use std::time::Duration;
use waterui::prelude::*;
use waterui::task::sleep;
use waterui::widget::suspense::Suspense;

async fn load_name(id: u32) -> String {
    sleep(Duration::from_millis(300)).await;
    format!("User {id}")
}

fn user_profile(user_id: Binding<u32>) -> impl View {
    watch(user_id, |id: u32| {
        Suspense::new(async move { text(load_name(id).await) })
    })
}
```

`watch` replaces its whole child subtree and discards any state the subtree owned. That is what you want here and rarely elsewhere: for a changing scalar use `text!` or a signal-taking input, and for a changing set of rows use `ForEach` or `List`. If the screen owns editable fields, create those bindings above the `watch` so they survive a reload.

## Nesting

Inner content can suspend again, so each region appears as soon as its own data lands:

```rust
use std::time::Duration;
use waterui::prelude::*;
use waterui::task::sleep;
use waterui::widget::suspense::Suspense;

fn feed() -> impl View {
    Suspense::new(async {
        sleep(Duration::from_millis(300)).await;

        vstack((
            text("Ada"),
            Suspense::new(async {
                sleep(Duration::from_millis(600)).await;
                text("2 posts")
            })
            .loading(text("Loading posts / 投稿を読み込み中 / 게시물 불러오는 중")),
        ))
    })
    .loading(text("正在加载 / 読み込み中 / 불러오는 중"))
}
```

## Suspense in `water preview`

A preview capture would otherwise photograph the loading view. Each `Suspense` therefore records that it owes content from the moment it mounts, and clears the entry after the resolved view has been swapped in. `water preview` waits for every owed entry before capturing, up to a deadline of 10 seconds by default. If a future never resolves, the preview fails rather than hanging, naming each entry as `Suspense` followed by the full type name of its content in parentheses, so you can tell which loader is late. Raise the deadline with `--content-timeout <SECONDS>`, which takes any positive number of seconds:

```bash
water preview my_view --platform macos --path ./app --output preview.png --content-timeout 30
```

The deadline exists to catch a load that never lands. When a future is slow only because it does real network work, give the preview a deterministic input instead of raising the number. [The preview chapter](../07-tools/01-preview.md) covers the rest of the command.

---

Next: [Error handling](04-error-handling.md), where the `Error` you just rendered inside a suspended view gets its presentation layer.

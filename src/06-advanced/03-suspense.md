# Suspense and Async Views

> **In this chapter, you will:**
> - Use `Suspense` to show loading states while async operations run
> - Customize loading views per-instance or app-wide
> - Implement the `SuspendedView` trait for environment-aware loading
> - Combine `Suspense` with reactive state for data that changes over time
> - Understand task lifecycle and cancellation

Most applications need to load data asynchronously -- from a network API, a
database, or a file system. Without proper handling, your users stare at a blank
screen wondering if the app is broken. WaterUI's `Suspense` component solves
this declaratively: show a placeholder while an async operation runs, then
seamlessly swap in the loaded content.

## The Suspense Component

`Suspense` lives in `waterui::widget::suspense`. It wraps any type that
implements the `SuspendedView` trait and pairs it with a loading view:

```rust,ignore
use waterui::widget::suspense::Suspense;

async fn fetch_user() -> impl View {
    // simulate network request
    text("John Doe")
}

let view = Suspense::new(fetch_user());
```

When the view tree is built, `Suspense`:

1. Immediately renders the **loading view** (by default, whatever
   `DefaultLoadingView` is in the environment).
2. Spawns the async content on the local executor.
3. Once the future resolves, replaces the loading view with the loaded content.

Internally, `Suspense` creates a `Dynamic` view and uses its handler to swap
content when the future completes.

## Custom Loading Views

The default loading view might not fit your design. WaterUI gives you two ways
to customize it: per-instance and app-wide.

### Inline Loading View

Use `.loading()` to provide a custom loading view for a specific `Suspense`
instance. The method has two type parameters -- the loading view type and the
async output view type -- so the call site needs the turbofish to pin down the
output type:

```rust,ignore
use waterui::widget::suspense::Suspense;
use waterui::prelude::*;
use waterui::text::Text;

async fn fetch_data() -> Text {
    text("Data loaded!")
}

let view = Suspense::new(fetch_data())
    .loading::<_, Text>(text("Loading data..."));
```

The loading view can be any type that implements `View` -- a spinner, a
skeleton placeholder, or even a complex layout. Pin the output of the async
function to a concrete view type (here, `Text`) so the second turbofish slot
can match it.

### Environment-Based Default

To set a consistent loading view across your entire application, install a
`DefaultLoadingView` in the environment. `DefaultLoadingView::new` accepts any
`ViewBuilder`, which is satisfied by closures of the form `Fn() -> impl View`:

```rust,ignore
use waterui::widget::suspense::DefaultLoadingView;
use waterui::app::App;
use waterui::prelude::*;

fn app(env: Environment) -> App {
    let mut env = env;
    env.insert(DefaultLoadingView::new(|| {
        vstack((
            text("Please wait..."),
        ))
    }));
    App::new(main, env)
}
```

Any `Suspense` component that does not provide an explicit `.loading()` view
will use this default. If no `DefaultLoadingView` is installed, `Suspense`
renders an empty view while loading.

> **Tip:** Always install a `DefaultLoadingView` in your root environment. This
> ensures every `Suspense` in your app has a visible loading state, even if you
> forget to add `.loading()` at a specific call site.

### UseDefaultLoadingView

`UseDefaultLoadingView` is the sentinel type used internally. When it renders,
it queries the environment for a `DefaultLoadingView` and invokes its builder.
You can use it explicitly if you want:

```rust,ignore
use waterui::widget::suspense::{Suspense, UseDefaultLoadingView};

let view = Suspense::new(fetch_data())
    .loading::<_, ()>(UseDefaultLoadingView);
```

This is equivalent to `Suspense::new(fetch_data())`.

## The SuspendedView Trait

`Suspense` accepts anything that implements `SuspendedView`:

```rust,ignore
pub trait SuspendedView: 'static {
    fn body(self, env: Environment) -> impl Future<Output = impl View>;
}
```

### Automatic Implementation for Futures

Any `Future` whose output implements `View` automatically satisfies
`SuspendedView`. This is why the simple async function approach works out of
the box:

```rust,ignore
async fn load_profile() -> impl View {
    let data = api::get_profile().await;
    text(data.name)
}

// This works because the future implements SuspendedView
let view = Suspense::new(load_profile());
```

### Custom SuspendedView

For more control, implement `SuspendedView` directly. This gives you access to
the `Environment` during the async operation, which is useful when you need
services like API clients or configuration:

```rust,ignore
use waterui::widget::suspense::SuspendedView;
use waterui::prelude::*;

struct UserLoader {
    user_id: u32,
}

impl SuspendedView for UserLoader {
    async fn body(self, env: Environment) -> impl View {
        // Access environment services during loading
        let api_client = env.get::<ApiClient>().unwrap().clone();
        let user = api_client.fetch_user(self.user_id).await;

        vstack((
            text(user.name).headline(),
            text(user.email),
        ))
    }
}

let view = Suspense::new(UserLoader { user_id: 42 });
```

The environment is cloned when the future is spawned, so you have access to all
services, themes, and configuration that were in scope.

## The `suspense()` Function

A convenience function creates a `Suspense` with the default loading view:

```rust,ignore
use waterui::widget::suspense::suspense;

let view = suspense(async {
    let data = load_something().await;
    text(data)
});
```

## Error Handling within Suspense

Async operations can fail. Since `Result<V, E>` implements `View` when both
`V: View` and `E: View`, you can handle errors directly inside the async block:

```rust,ignore
use waterui::prelude::*;
use waterui::widget::suspense::Suspense;
use waterui::widget::error::Error;

async fn fetch_with_error() -> impl View {
    match api::get_data().await {
        Ok(data) => text(data.content).anyview(),
        Err(e) => Error::new(e).anyview(),
    }
}

let view = Suspense::new(fetch_with_error());
```

For a more ergonomic pattern, combine with the `ResultExt` trait described in
the [Error Handling](04-error-handling.md) chapter.

## Combining Suspense with Reactive State

`Suspense` is a one-shot component -- it resolves once and then shows the
result. But what if your data source can change? For example, a user profile
page where the user ID comes from navigation state. Combine `Suspense` with
`Dynamic::watch` to trigger reloads:

```rust,ignore
use waterui::prelude::*;
use waterui::widget::suspense::Suspense;

fn user_profile(user_id: Binding<u32>) -> impl View {
    Dynamic::watch(user_id, |id| {
        Suspense::new(async move {
            let user = api::get_user(id).await;
            text(user.name)
        })
    })
}
```

Every time `user_id` changes, a new `Suspense` is created, which shows the
loading view and kicks off a fresh async operation.

## Lifecycle and Cancellation

The async task spawned by `Suspense` uses `executor_core::spawn_local`. The
task handle is detached, meaning it will run to completion even if the
`Suspense` view is removed from the tree.

> **Warning:** If you navigate away from a screen while a `Suspense` task is
> running, the task will complete in the background. Be mindful of this if
> your async operation has side effects.

If you need cancellation semantics, tie the task to the view lifecycle using
`ViewExt::task` instead of `Suspense`:

```rust,ignore
use waterui::prelude::*;

fn my_view() -> impl View {
    let data = Binding::container::<Option<String>>(None);
    let data_for_task = data.clone();

    text("Loading...")
        .task(async move {
            let result = api::get_data().await;
            data_for_task.set(Some(result));
        })
}
```

The task spawned by `.task()` returns a handle that is retained by the view.
When the view is dropped, the handle is dropped and the task is cancelled.

## Nested Suspense

You can nest `Suspense` components for situations where loaded content itself
needs to fetch more data. Each inner suspense manages its own loading state
independently:

```rust,ignore
use waterui::prelude::*;
use waterui::widget::suspense::Suspense;

let view = Suspense::new(async {
    let user = api::get_user(1).await;

    vstack((
        text(user.name).headline(),
        Suspense::new(async move {
            let posts = api::get_posts(user.id).await;
            vstack(
                posts.into_iter().map(|p| text(p.title)).collect::<Vec<_>>()
            )
        }).loading(text("Loading posts...")),
    ))
}).loading(text("Loading user..."));
```

The outer suspense shows "Loading user..." while the user is fetched. Once the
user loads, the inner suspense shows "Loading posts..." while fetching posts.
This creates a progressive loading experience where content appears as it
becomes available.

## Summary

| API | Purpose |
|---|---|
| `Suspense::new(content)` | Create suspense with default loading view |
| `.loading(view)` | Set a custom loading view |
| `suspense(future)` | Convenience function |
| `SuspendedView` trait | Custom async content loading |
| `DefaultLoadingView::new(builder)` | App-wide default loading view |
| `UseDefaultLoadingView` | Render the default loading view |
| `ViewExt::task(future)` | Lifecycle-bound async task |
| `Dynamic::watch(signal, f)` | Reactive suspense reloading |

## What's Next

Async operations can fail, and when they do, your users need to see something
useful -- not a blank screen. In the [next chapter](04-error-handling.md), you
will learn how WaterUI turns errors into views and how to build consistent error
presentation across your entire application.

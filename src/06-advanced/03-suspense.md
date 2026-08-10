# Suspense and async views

> **In this chapter, you will:**
> - Show a placeholder while an async operation runs, then swap in the result
> - Customize the loading view per instance and app-wide
> - Implement `SuspendedView` to reach the environment during loading
> - Choose between `Suspense` and `ViewExt::task` based on cancellation needs
> - Reload suspended content when its input changes

A view body is synchronous, but data usually is not. `Suspense` bridges the two: it renders a loading view immediately, spawns your future on the local executor, and replaces the placeholder when the future resolves.

## The Suspense component

`Suspense` lives in `waterui::widget::suspense`, and the `suspense()` shorthand is in the prelude.

```rust,ignore
use waterui::prelude::*;
use waterui::text::Text;
use waterui::widget::suspense::Suspense;

async fn fetch_user() -> Text {
    text(api::get_user_name().await)
}

let view = Suspense::new(fetch_user());
```

Any `Future` whose output is a `View` implements `SuspendedView`, which is why a plain `async fn` works with no extra glue.

Internally `Suspense` allocates a `Dynamic` node, sets the loading view into it, spawns the future with `spawn_local`, and sets the resolved content when it completes. The swap replaces that subtree — deliberately, since the placeholder and the content are different views.

## Loading views

### Per instance

`.loading()` overrides the placeholder for one `Suspense`:

```rust,ignore
use waterui::prelude::*;
use waterui::text::Text;
use waterui::widget::suspense::Suspense;

async fn fetch_data() -> Text {
    text("Data loaded!")
}

let view = Suspense::new(fetch_data())
    .loading::<_, Text>(text("Loading data..."));
```

The turbofish is not optional. `loading` is declared as `loading<Loading2, Output: View>(self, loading: Loading2)`, and `Output` appears nowhere in the arguments or the return type, so inference has nothing to work from and the call site has to spell it. Any `View` type satisfies it; naming the async function's own output type, as here, at least keeps the intent readable.

### App-wide

Install a `DefaultLoadingView` in the environment and every `Suspense` without an explicit `.loading()` picks it up. `DefaultLoadingView::new` takes any `ViewBuilder`, which a `Fn() -> impl View` closure satisfies:

```rust,ignore
use waterui::app::App;
use waterui::prelude::*;
use waterui::widget::suspense::DefaultLoadingView;

pub fn app(env: Environment) -> App {
    let mut env = env;
    env.insert(DefaultLoadingView::new(|| {
        vstack((loading(), text("Please wait...")))
    }));
    App::new(main, env)
}
```

`loading()` is the facade's indeterminate circular `Progress`. Without a `DefaultLoadingView`, `Suspense` renders an empty view while loading — install one at the root so no async screen is ever blank.

`UseDefaultLoadingView` is the sentinel that performs that lookup. `Suspense::new(fut)` uses it already; naming it explicitly is only useful when you need to write the type out:

```rust,ignore
use waterui::widget::suspense::{Suspense, UseDefaultLoadingView};

// Identical to Suspense::new(fetch_data()).
let view = Suspense::new(fetch_data()).loading::<_, ()>(UseDefaultLoadingView);
```

## Implementing SuspendedView

```rust,ignore
pub trait SuspendedView: 'static {
    fn body(self, env: Environment) -> impl Future<Output = impl View>;
}
```

Implement it directly when the async work needs environment services — an API client, a configuration value, a locale. `Suspense` clones the environment before spawning, so everything in scope at construction is available inside the future:

```rust,ignore
use waterui::prelude::*;
use waterui::widget::suspense::{SuspendedView, Suspense};

struct UserLoader {
    user_id: u32,
}

impl SuspendedView for UserLoader {
    async fn body(self, env: Environment) -> impl View {
        let api = env
            .get::<ApiClient>()
            .expect("ApiClient must be installed before rendering UserLoader")
            .clone();
        let user = api.fetch_user(self.user_id).await;

        vstack((text(user.name).headline(), text(user.email)))
    }
}

let view = Suspense::new(UserLoader { user_id: 42 });
```

## Failures are views too

`Result<V, E>` implements `View` when both sides do, so a fallible load can resolve to either branch:

```rust,ignore
use waterui::prelude::*;
use waterui::widget::error::Error;
use waterui::widget::suspense::Suspense;

async fn fetch_with_error() -> AnyView {
    match api::get_data().await {
        Ok(data) => text(data.content).anyview(),
        Err(e) => Error::new(e).anyview(),
    }
}

let view = Suspense::new(fetch_with_error());
```

`Error::new` renders through your app's `DefaultErrorView`, so a failed load looks like every other failure in the app. The [Error handling](04-error-handling.md) chapter covers `ResultExt::error_view` for shaping the error at the call site.

## Suspense or `ViewExt::task`?

They differ in one respect that matters: cancellation.

`Suspense` detaches its task. If the user navigates away mid-flight, the future still runs to completion — fine for a read, a problem for anything with side effects.

`ViewExt::task` retains the task handle on the view. Dropping the view drops the handle, and dropping the handle cancels the task:

```rust,ignore
use waterui::prelude::*;
use waterui::reactive::binding;

fn my_view() -> impl View {
    let status: Binding<Str> = binding("Loading...");
    let sink = status.clone();

    text!("{status}").task(async move {
        sink.set(api::get_status().await);
    })
}
```

Reach for `Suspense` when the placeholder is a different view from the result. Reach for `.task()` when the view already exists and the async work only fills in reactive state.

## Reloading when the input changes

`Suspense` resolves once. When the input identity changes — a different user id, a different document — the correct behavior is a genuinely new `Suspense` instance, placeholder included, and that is one of the rare cases `watch` exists for:

```rust,ignore
use waterui::prelude::*;
use waterui::widget::suspense::Suspense;

fn user_profile(user_id: Binding<u32>) -> impl View {
    watch(user_id, |id: u32| {
        Suspense::new(async move { text(api::get_user(id).await.name) })
    })
}
```

`watch` replaces the whole child subtree and discards any state it owned. That is what you want here and almost nowhere else: for a changing scalar use `text!` or a signal-taking input, and for a changing set of rows use `ForEach` / `List`. If the profile screen owns editable fields, hoist those bindings above the `watch` so they survive the reload.

## Nesting

Inner content can suspend again, so each region appears as soon as its own data lands:

```rust,ignore
use waterui::prelude::*;
use waterui::text::Text;
use waterui::widget::suspense::Suspense;

let view = Suspense::new(async {
    let user = api::get_user(1).await;

    vstack((
        text(user.name).headline(),
        Suspense::new(async move {
            let posts = api::get_posts(user.id).await;
            vstack(posts.into_iter().map(|p| text(p.title)).collect::<Vec<_>>())
        })
        .loading::<_, Text>(text("Loading posts...")),
    ))
})
.loading::<_, Text>(text("Loading user..."));
```

Both levels need the turbofish, for the same reason as above.

---

Next: [Error handling](04-error-handling.md), where the `Error` type you just spawned into a suspended view gets a proper presentation layer.

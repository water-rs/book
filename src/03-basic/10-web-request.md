# Networking and web requests

WaterUI delegates asynchronous work to the executor supplied by your application. The `waterui::task`
module re-exports `spawn` and `spawn_local`, and the `widget::suspense` module helps you display
loading states while futures resolve.

## Injecting HTTP clients with the environment

Install reusable services—such as a `reqwest::Client`—into the environment and extract them in event
handlers with `Use<T>`.

```rust,ignore
use reqwest::Client;
use waterui::prelude::*;
use waterui_core::extract::Use;

#[derive(Clone)]
struct WeatherClient(Client);

fn app() -> impl View {
    button("Refresh forecast")
        .on_tap(|Use(client): Use<WeatherClient>| {
            println!("Client ready: {:#?}", client.0);
        })
        .with(WeatherClient(Client::new()))
}
```

`Use<T>` clones the service out of the environment, ensuring the handler always sees the latest
instance (for example after tests swap in a mock client).

## Performing background requests

Use `spawn_local` to offload work from the handler thread. Bindings capture the result and update the
UI reactively.

```rust,ignore
use reqwest::Client;
use serde::Deserialize;
use waterui::prelude::*;
use waterui::task::spawn_local;
use waterui_core::extract::Use;
use nami::binding;

#[derive(Clone)]
struct WeatherClient(Client);

#[derive(Clone, Deserialize)]
struct Forecast {
    summary: String,
}

fn forecast_view() -> impl View {
    let state = binding(String::from("Tap refresh"));

    let refresh = {
        let state = state.clone();
        move |Use(client): Use<WeatherClient>| {
            spawn_local({
                let state = state.clone();
                async move {
                    state.set(String::from("Loading…"));
                    match client.0.get("https://weather.example.com/today").send().await {
                        Ok(response) => match response.json::<Forecast>().await {
                            Ok(data) => state.set(data.summary),
                            Err(err) => state.set(format!("Decode failed: {err}")),
                        },
                        Err(err) => state.set(format!("Request failed: {err}")),
                    }
                }
            });
        }
    };

    vstack((
        text!("{}", state.clone()),
        button("Refresh").on_tap(refresh),
    ))
    .spacing(12.0)
    .with(WeatherClient(Client::new()))
}
```

The closure runs synchronously to kick off the task, but the heavy lifting happens inside the async
block. When the task completes it updates the binding, which propagates to the `text!` macro.

## Using Suspense for data-driven views

When the entire view depends on an asynchronous fetch, wrap the loader in a `SuspendedView` and pass
it to `Suspense`.

```rust,ignore
use reqwest::Client;
use serde::Deserialize;
use waterui::component::text::Text;
use waterui::prelude::*;
use waterui::widget::suspense::{Suspense, SuspendedView};
use waterui_core::extract::{Extractor, Use};

#[derive(Clone)]
struct ApiClient(Client);

#[derive(Clone, Deserialize)]
struct Profile {
    display_name: String,
}

struct ProfileLoader;

impl SuspendedView for ProfileLoader {
    async fn body(self, env: Environment) -> impl View {
        let Use(client) = Extractor::extract::<Use<ApiClient>>(&env)
            .expect("API client installed");

        match client.0.get("https://example.com/profile").send().await {
            Ok(response) => match response.json::<Profile>().await {
                Ok(profile) => vstack((
                    text!("Welcome back"),
                    text!("{}", profile.display_name.clone()),
                )),
                Err(err) => text!("Failed to decode profile: {err}"),
            },
            Err(err) => text!("Failed to load profile: {err}"),
        }
    }
}

fn profile_screen() -> impl View {
    Suspense::new(ProfileLoader)
        .loading::<_, Text>(text!("Loading profile…"))
        .with(ApiClient(Client::new()))
}
```

`Suspense` drives the future on the configured executor. You can inject a default loading view with a
`DefaultLoadingView` in the environment or provide a custom placeholder by calling
`.loading::<_, Text>(…)` as shown above.

# WebView

> **In this chapter, you will:**
> - Embed web content with the deferred, reactive `WebView::open` and drive navigation from a URL signal
> - Control a live page through `WebViewProxy`, and observe it through `on_event`
> - Run JavaScript from Rust with `eval!` and `exec!`, and inject scripts that run on every load
> - Expose Rust to the page with `#[js_api]`, and restrict which documents may reach it
> - Constrain page-initiated navigation, and bundle a web frontend into your app with `include_web!`
> - Choose the browser engine by choosing your dependencies, and know when to reach for `waterui-chromium`

Documentation pages, OAuth flows, and whole web apps already exist. `waterui-webview` embeds them in your native shell and gives you navigation, cookies, JavaScript execution, and a typed Rust bridge from the same view tree as the rest of your UI.

> **Feature flag:** `webview` is not in the default feature set. Enable it explicitly:
>
> ```toml
> waterui = { version = "0.5", features = ["webview"] }
> ```
>
> `include_web!`, covered at the end of this chapter, additionally needs `assets`, which is on by default. An app that loads remote pages also needs the internet permission in `Water.toml` (`[permissions.internet]`, `enable = true`, as `examples/webview` declares it).

## Opening a web view

```rust
use waterui::prelude::*;
use waterui::webview::WebView;

fn docs_page() -> impl View {
    WebView::open("https://waterui.dev/docs")
}
```

`WebView::open` does not create a browser. It returns a `WebViewOpen`, a deferred description: the native handle is created when the view renders and a `WebViewController` is found in the live environment. Everything you chain on the builder (`user_agent`, `redirects_enabled`, `inject`, `handler`, `on_event`, and so on) is applied to that handle the moment it exists, so a web view can be fully described before a backend is anywhere in sight.

Rendered without a `WebViewController` in the environment, a debug build panics and a release build logs the contract violation and renders nothing. A `WebView` fills the space it is offered (`StretchAxis::Both`); constrain it with the surrounding layout.

### The URL is a signal

`open` takes `impl IntoUrlSignal`: a `&'static str` literal, a `Url` or `&Url`, a `Binding<Url>` or `&Binding<Url>`, or a `Computed<Url>`. Writing a new URL navigates the existing native web view.

```rust
use waterui::prelude::*;
use waterui::webview::{Url, WebView};

fn browser() -> impl View {
    let url: Binding<Url> = binding(Url::new("https://waterui.dev"));
    WebView::open(url)
}
```

The signal is retained for the native view's lifetime, so nothing is torn down and no page state is lost. That is why you never wrap a web view in `watch` to change its address. Text typed by a person is not a `Url`: parse it with `Url::parse_user_input`, which tolerates a missing scheme and returns an `Option`, and report the failure instead of handing a bad address to the engine.

## Imperative controls with `WebViewProxy`

Refresh, stop, history, and a free-form address bar have no natural reactive input. `WebViewOpen::with_proxy` renders your controls above the web view and injects a `WebViewProxy` into their environment, so any handler inside can take one as a parameter, the same extractor machinery that supplies `State<T>` to `Button::action`.

```rust
use waterui::prelude::*;
use waterui::webview::{Url, WebView, WebViewProxy};

fn mini_browser() -> impl View {
    let address: Binding<Str> = binding("https://waterui.dev");

    WebView::open("https://waterui.dev").with_proxy(move || {
        hstack((
            button("Back").action(|proxy: WebViewProxy| proxy.go_back()),
            button("Forward").action(|proxy: WebViewProxy| proxy.go_forward()),
            button("Refresh").action(|proxy: WebViewProxy| proxy.refresh()),
            button("Stop").action(|proxy: WebViewProxy| proxy.stop()),
            TextField::new("Address", &address).hide_label(),
            button("Go")
                .action(|proxy: WebViewProxy, State(entered): State<Binding<Str>>| {
                    if let Some(url) = Url::parse_user_input(entered.snapshot().as_str()) {
                        proxy.go_to(url);
                    }
                })
                .state(&address),
        ))
    })
}
```

`with_proxy` places the web view below the controls. For any other arrangement, open the view yourself with a `WebViewController` and call `WebView::proxy_scope`, which attaches the proxy to a subtree and places nothing.

`WebViewProxy` offers `go_to`, `go_back`, `go_forward`, `refresh`, `stop`, `run_javascript`, `inject_script`, `set_user_agent`, `set_redirects_enabled`, `set_cookie`, `get_cookies`, and `handle()`. Extracting one outside a `with_proxy` scope fails with an error that names the fix.

## Owning the handle with `WebViewController`

A backend (or an installed browser engine) puts a `WebViewController` into the environment. Extract it when you need the `WebView` value itself: to read its state, hand it to several handlers as `State<WebView>`, or call methods directly.

```rust
use waterui::env::use_env;
use waterui::prelude::*;
use waterui::webview::{WebView, WebViewController};

fn custom_browser() -> impl View {
    use_env(|controller: WebViewController| -> WebView {
        let webview = controller.open(); // opens blank
        webview.go_to("https://waterui.dev");
        webview.set_user_agent("WaterUIBook/1.0");
        webview
    })
}
```

`WebView` has the same imperative methods as the proxy, plus `can_go_back()` and `can_go_forward()`, which return `Computed<bool>` tracking the native history, so they feed `.disabled(...)` directly:

```rust
use waterui::prelude::*;
use waterui::webview::WebView;

fn back_button(webview: &WebView) -> impl View + use<> {
    button("Back")
        .action(|State(webview): State<WebView>| webview.go_back())
        .state(webview)
        .disabled(webview.can_go_back().map(|can| !can))
}
```

## Events

`WebViewOpen::on_event` observes the navigation lifecycle. The subscription lives as long as the web view, so there is no guard to hold.

```rust
use waterui::prelude::*;
use waterui::webview::{WebView, WebViewEvent};

fn watched_page() -> impl View {
    let status: Binding<Str> = binding("Idle");
    let writer = status.clone();

    vstack((
        text!("{status}"),
        WebView::open("https://waterui.dev").on_event(move |event| match event {
            WebViewEvent::WillNavigate { url } => {
                writer.set(Str::from(format!("Navigating to {url}")));
            }
            WebViewEvent::Loading { progress } => {
                writer.set(Str::from(format!("Loading {:.0}%", progress * 100.0)));
            }
            WebViewEvent::Loaded => writer.set(Str::from_static("Loaded")),
            WebViewEvent::Redirect { from, to } => {
                writer.set(Str::from(format!("Redirect: {from} -> {to}")));
            }
            WebViewEvent::Error(error) => writer.set(Str::from(format!("{error}"))),
            _ => {}
        }),
    ))
}
```

| Event | Fields | Meaning |
|---|---|---|
| `WillNavigate` | `url: Url` | Navigation is about to begin |
| `Loading` | `progress: f32` | Load progress, 0.0 to 1.0 |
| `Loaded` | none | The page finished loading |
| `Redirect` | `from: Url, to: Url` | A redirect occurred |
| `Error` | `WebViewError` | Navigation or loading failed |

The enum is `#[non_exhaustive]`, so keep a wildcard arm. There is no "nothing happened" variant: `WebView::events()` is a `Signal<Output = Option<WebViewEvent>>` that is `None` before the first event, and history changes never appear here; read `can_go_back()` and `can_go_forward()` for those. `WebViewError` has three variants: `Network(Str)`, `Ssl { url, message }`, and `LoadFailed(Str)`.

`redirects_enabled(signal)` and `user_agent(signal)` on the builder take signals too, so a `Toggle` bound to a `Binding<bool>` can switch redirect following on a live page.

## Running JavaScript

### Typed evaluation: `eval!` and `exec!`

`eval!(webview, "expression")` evaluates a JavaScript expression and decodes the result into the Rust type you ask for; `exec!` runs a program for its effects. Both take the web view as their first argument and await the page's promise when the expression returns one.

```rust
use waterui::webview::{JsError, WebView};
use waterui::{eval, exec, Str};

async fn title_of(webview: &WebView) -> Result<String, JsError> {
    let title: String = eval!(webview, "document.title").await?;
    Ok(title)
}

async fn apply_theme(webview: &WebView, theme: Str) -> Result<(), JsError> {
    exec!(webview, "document.documentElement.dataset.theme = @{theme}").await
}
```

`@{expr}` holes are serialized and passed to the engine as bound arguments, never spliced into the source, so a Rust value can never be read back as JavaScript code. The sigil is `@{}` because `${}` belongs to JavaScript template literals; write `@@{` for a literal `@{`. Failures are a `JsError` rather than one opaque string: `Exception` (the script threw), `Unserializable` (a circular structure, a DOM node), `Decode` (the result is not the requested type), `Navigated` (the document was replaced first), or `Closed`.

Because `eval!` needs a `WebView`, keep one in a `#[state]` struct and take it as an action parameter:

```rust
use waterui::env::use_env;
use waterui::prelude::*;
use waterui::webview::{JsError, WebView, WebViewController};
use waterui::eval;

#[state]
#[derive(Clone)]
struct Probe {
    webview: WebView,
    title: Binding<Str>,
}

fn title_probe() -> impl View {
    use_env(|controller: WebViewController| {
        let webview = controller.open();
        webview.go_to("https://waterui.dev");
        let probe = Probe {
            webview: webview.clone(),
            title: binding(""),
        };

        vstack((
            button("Read title")
                .action_async(|probe: Probe| async move {
                    let webview = &probe.webview;
                    let result: Result<String, JsError> = eval!(webview, "document.title").await;
                    match result {
                        Ok(title) => probe.title.set(Str::from(title)),
                        Err(error) => probe.title.set(Str::from(format!("{error}"))),
                    }
                })
                .state(&probe),
            text!("{title}", title = probe.title.clone()),
            webview,
        ))
    })
}
```

### The raw path

`WebView::run_javascript` and `WebViewProxy::run_javascript` take one JavaScript expression (not a statement list) and resolve to `Result<Str, Str>` holding the page's own `JSON.stringify` of the value. A string therefore arrives quoted: `document.title` yields `"Docs"` with the quotes. `undefined` arrives as `null`, and a promise is not awaited (it encodes as `{}`). Prefer `eval!` unless you want the JSON text itself. The future is main-thread affine, so drive it from `action_async`.

### Injecting scripts

Code that must exist before the page's own scripts goes through `inject`. Injected scripts run on every page load the bridge policy admits, and the first document already runs them. A key names the script, and injecting again under the same key replaces it.

```rust
use waterui::prelude::*;
use waterui::webview::{ScriptInjectionTime, WebView};

fn marked_page() -> impl View {
    WebView::open("https://waterui.dev").inject(
        "host-marker",
        "document.documentElement.dataset.host = 'waterui';",
        ScriptInjectionTime::DocumentStart,
    )
}
```

`ScriptInjectionTime` is `DocumentStart` (before the DOM exists: global setup, interception) or `DocumentEnd` (after the document has loaded: DOM edits). Scripts added after creation use `WebView::inject_script(key, script, time)` or the proxy's twin and run on every later page load; a key that the view was opened with cannot be replaced afterwards and panics. Backends carry the combined list as `DocumentScripts` of `UserScript` values; application code never builds those.

Multi-line JavaScript belongs in a file. `js_file!("scripts/setup.js")` embeds it at compile time (the path resolves like `include_str!`, relative to the source file) as a `JsProgram` that `.inject(...)` and `webview.exec(&...)` both accept; `exec_file!(webview, "scripts/setup.js")` is the fused form of the second.

## Calling Rust from JavaScript

Every document the bridge policy admits (see [which documents may call it](#which-documents-may-call-it)) gets a page-side `waterui` object: `waterui.invoke(name, payload)` calls a handler and returns a promise. The bridge is deliberately a namespaced function, not a global per handler, so a handler called `open` or `print` cannot overwrite a page global.

### `#[js_api]`

Write the surface once, as an `impl` block. The shape of each method decides how it is exposed:

| Shape | Exposed as | Page writes it back? |
|---|---|---|
| `async fn(&self, ..) -> R` | a handler the page awaits | n/a |
| `fn(&self) -> Binding<T>` | mirrored state | yes |
| `fn(&self) -> Computed<T>` | mirrored state | no |
| `#[js(skip)]` anything | not exposed | n/a |

```rust
use waterui::js_api;
use waterui::prelude::*;
use waterui::webview::{Json, WebView, serde};

#[derive(serde::Serialize)]
#[serde(crate = "waterui::webview::serde")]
struct Greeting {
    text: String,
}

#[derive(Debug)]
enum ApiError {
    EmptyName,
}

impl core::fmt::Display for ApiError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EmptyName => f.write_str("a greeting needs a name"),
        }
    }
}

struct PageApi {
    theme: Binding<Str>,
    greetings: Binding<u32>,
}

#[js_api]
impl PageApi {
    /// Mirrored state, writable: a page assignment flows back into the binding.
    fn theme(&self) -> Binding<Str> {
        self.theme.clone()
    }

    /// Mirrored state, read-only: assigning to it in the page throws.
    fn greetings(&self) -> Computed<u32> {
        self.greetings.computed()
    }

    /// `await waterui.invoke("greet", { name: "Lexo" })` resolves to
    /// `{ text: "Hi Lexo" }`. An `Err` rejects the page's promise.
    async fn greet(&self, name: String) -> Result<Json<Greeting>, ApiError> {
        if name.is_empty() {
            return Err(ApiError::EmptyName);
        }
        self.greetings.with_mut(|count| *count += 1);
        Ok(Json(Greeting {
            text: format!("Hi {name}"),
        }))
    }
}

fn page() -> impl View {
    WebView::open("https://app.waterui.dev").serve(PageApi {
        theme: binding("light"),
        greetings: Binding::u32(0),
    })
}
```

`#[js_api(namespace = "app")]` prefixes every exposed name, so the page calls `waterui.invoke("app.greet", { name })` and reads `waterui.state["app.theme"]`; `#[js(rename = "...")]` renames one member. A method without a `self` receiver is not exposed, and a synchronous method that takes arguments is a compile error: make it `async fn` or mark it `#[js(skip)]`. The macro writes exactly the `.handler(...)` and `.expose(...)` calls you could write yourself, so the two paths are interchangeable.

On the page, state is a local property, so reading it never crosses the boundary, but it has no value until the first request has returned. Await `waterui.ready` first:

```javascript
await waterui.ready;
const theme = waterui.state.theme;
waterui.state.theme = "dark"; // writable: flows back into the Binding
const stop = waterui.watch("greetings", (count) => console.log(count));
const reply = await waterui.invoke("greet", { name: "Lexo" });
```

Reading or assigning a key before `ready` resolves throws, as does assigning a read-only key or naming an unknown one. Integers a JavaScript number cannot hold exactly cross as `BigInt`.

For a TypeScript frontend, `waterui::webview::typescript_declarations::<PageApi>()` renders a `.d.ts` for the surface. Write it next to the page sources from a test, so a surface that changes without regenerating the declarations fails on the page's type check. Argument names and arity are exact; an application's own struct type shows up as `unknown`.

### Handlers by hand

`.handler(name, closure)` is the underlying form, written the way `Button::action` is: take the extractors you need (`Json<T>`, `Bytes`, `HandlerName`, `State<T>`, `WebViewProxy`) and return anything that implements `IntoJsReply`. The closure may be `async`, and returning `Err` rejects the promise. `Json<T>`, a `String`, or `()` resolve as the value itself; `Bytes` and `Vec<u8>` resolve as a `Uint8Array`. A `Json<T>` argument needs `T: Deserialize` and a `Json<T>` reply needs `T: Serialize`. Derive them through the re-exported `waterui::webview::serde` with `#[serde(crate = "waterui::webview::serde")]`, as `Greeting` does above, or depend on `serde` yourself.

### Which documents may call it

A handler is a capability, and a web view that can navigate can end up somewhere you did not choose. By default only the origin the view was opened at, in the main frame, reaches the bridge. Widen that on purpose:

```rust
use waterui::prelude::*;
use waterui::webview::{Json, WebView};

fn two_origins() -> impl View {
    WebView::open("https://app.waterui.dev")
        .handler("greet", |Json(name): Json<String>| async move { format!("Hi {name}") })
        .bridge_origins(["https://app.waterui.dev", "https://docs.waterui.dev"])
}
```

A list of origins (each `scheme://host[:port]`) is the common case. `BridgeOrigins` also has `Initial` (the default), `LocalFiles`, and `Any`, which makes every handler reachable by any page the view loads; use it only for content you control end to end.

## Controlling navigation

Pages navigate themselves: a link, a `window.location` write, a form submission, `window.open`. `WebViewOpen::navigation_policy` answers each of those before the engine commits. Loads your application directs (the opening URL and every write to the URL signal) are never consulted, and server redirects belong to `redirects_enabled`.

A policy is a `Fn(&NavigationRequest) -> NavigationDecision`. A request reports `url()`, `from()`, `is_main_frame()`, and `user_initiated()`; the decision is `Allow`, `Cancel`, or `OpenExternally`, which cancels the load and hands the URL to the system browser.

```rust
use waterui::webview::{NavigationDecision, NavigationRequest};

fn docs_only(request: &NavigationRequest) -> NavigationDecision {
    if !request.is_main_frame() {
        return NavigationDecision::Allow;
    }
    if request.url().as_str().starts_with("https://waterui.dev/") {
        NavigationDecision::Allow
    } else {
        NavigationDecision::OpenExternally
    }
}
```

`stay_on_origin` is the ready-made policy: main-frame navigations stay on the origin of the document they leave, any other main-frame target opens externally, and subframes are allowed. Install one with `.navigation_policy(docs_only)`; a later call replaces an earlier policy.

```rust,ignore
WebView::open("https://waterui.dev")
    .navigation_policy(docs_only)
```

On Apple platforms the call compiles only when the deployment target is OS 27 or newer, because `WKNavigationAction.mainFrameNavigation` is public from there. Raise the platforms you ship under `[apple.deployment-targets]` in `Water.toml` to `27.0`; other targets are unaffected. On Android the policy does not see `POST` form submissions or `http(s)` subframe loads, because the platform never offers them to its hook.

## Cookies

```rust
use waterui::webview::{Cookie, WebView, cookie_expiry};

fn set_session_cookie(webview: &WebView, token: String) -> Result<(), waterui::Error> {
    let cookie = Cookie::build(("session", token))
        .domain("waterui.dev")
        .path("/")
        .secure(true)
        .expires(cookie_expiry("session", 1_900_000_000.0)?)
        .build();
    webview.set_cookie(cookie);
    Ok(())
}

async fn log_cookies(webview: &WebView) -> Result<(), waterui::Error> {
    for cookie in webview.get_cookies().await? {
        waterui::log::info!(name = cookie.name(), "cookie present");
    }
    Ok(())
}
```

`Cookie` is the `cookie` crate's type re-exported as `waterui::webview::Cookie`. `cookie_expiry` converts seconds since 1970 into the date a cookie carries and fails, naming the cookie, for a value that is not a representable date. Reading is asynchronous so the UI thread never blocks on the native store, and a store that cannot be read is an `Err`, not an empty list. Cookie APIs are absent from the browser build, where the page is a cross-origin frame.

## Bundling a web frontend

When the web content is yours, ship it inside the app and serve it from a local origin instead of the network. `include_web!` is the one-macro form: it takes the `ResourceContext` and the path of a web project, and expands to a `WebViewOpen`, so everything above chains onto it.

```rust,ignore
use waterui::app::App;
use waterui::env::use_env;
use waterui::prelude::*;
use waterui::{ResourceContext, include_web, js_api};

struct Api;

#[js_api]
impl Api {
    /// `await waterui.invoke("greet", { name })` resolves to the string.
    async fn greet(&self, name: String) -> String {
        format!("Hello, {name} from Rust")
    }
}

fn main_view() -> impl View {
    use_env(|env: Environment| {
        ResourceContext::from_environment(&env)
            .map(|resources| include_web!(resources, "web").serve(Api))
    })
}

pub fn app(env: Environment) -> App {
    App::new(main_view, env)
}
```

The second argument is the web project root relative to `CARGO_MANIFEST_DIR`, and it must contain a `package.json`: the macro points at the project, not at its build output. Four named arguments are the whole configuration surface:

| Argument | Default | Effect |
|---|---|---|
| `out_dir = "build"` | `dist` | The build output inside the root |
| `entry = "app.html"` | `index.html` | The entry document |
| `spa = true` | `false` | Unresolved extensionless paths fall back to `index.html` |
| `csp = "..."` | strict `DEFAULT_CSP` | Replaces the `Content-Security-Policy` on HTML responses |

The macro embeds no frontend bytes and never runs a bundler. Building the frontend and staging its output into the platform bundle is the CLI's job (`water run`, `water package`). A development build (`water run`, `water preview` and `water build` enable the crate's `dev` feature for it) is handed the package manager's dev server URL through `WATERUI_DEV_URL` or a `--waterui-dev-url=` argument, which `waterui::webview::dev_url()` reads, and the view loads from that server instead of the bundle; a packaged build always serves the staged bundle.

The page is served from the engine's local asset origin: `waterui://localhost` on `WKWebView`, WebKitGTK, WPE and CEF, and `https://waterui.localhost` on Android's `WebView` and CDP-driven Chromium. Either way the page is a secure context, so `isSecureContext`, module scripts, `fetch`, and WASM work without `file://`. Any method other than GET or HEAD is a `405`, a path that escapes the bundle is a `404`, and the default CSP (`waterui::webview::DEFAULT_CSP`) allows same-origin scripts and WASM, same-origin and inline styles, `data:` and `blob:` images, no plugins, and no framing of the page.

The expansion installs `stay_on_origin`, so links that leave the app open in the system browser; calling `.navigation_policy(..)` replaces it. Because the expansion itself calls `navigation_policy`, every `include_web!` carries the Apple OS 27 deployment-target requirement above. One application has one `include_web!`: the CLI reports a second as an error when it reads the build.

Without `include_web!`, `WebView::open_assets(server, entry)` serves any `Fn(&AssetRequest) -> AssetResponse + Send + Sync` under the same origin. `DirectoryServer::new(dir)` is the directory-backed server `include_web!` uses (with `.spa(..)` and `.csp(..)`), and `.into_server_fn()` turns it into that closure. An engine with no local asset origin panics when such a view renders.

### Scaffolding

Two CLI entry points set the project up (see [the CLI chapter](../01-getting-started/01-cli.md)):

```bash
# A new project whose root view is include_web! plus a Vite frontend in web/
water create "My App" --template web
# Optional: --package-manager bun|pnpm|npm|yarn, --vite-template vanilla-ts

# Add a frontend to the current directory
water init --web new                       # scaffold a Vite project into web/
water init --web ../my-site --web-mode copy       # copy an existing project into web/
water init --web ../my-site --web-mode reference  # leave it in place
```

`create --template web` writes a `[web]` table in `Water.toml` that declares the package manager (bun by default), `web/` with the Vite project, and the Apple deployment targets that `include_web!` requires. `water init` decides its steps from the directory: a directory that already holds a `package.json` has its frontend moved into `web/` and gets the Rust shell around it. The shell it writes is the one above, with a small `#[js_api]` surface. The project layout is in [the project structure chapter](../01-getting-started/04-project-structure.md), and resources in [the assets chapter](05-assets.md).

## Choosing a browser engine

Which engine draws a `WebView` follows your dependencies, not a `Water.toml` key. Link nothing and the platform's own engine is used: `WKWebView` on Apple platforms, Android's `WebView`, `WebKitGTK` on GTK, and a cross-origin `<iframe>` in the browser. The Hydrolysis backend bridges a platform engine only on macOS, Android and the web; on Linux and Windows it has none, so the application links an engine crate. `examples/webview` does this on Linux by depending on `waterui-browser-wpe` (with its `webview` feature) and calling `waterui_browser_wpe::install(&mut env)` in `app`.

The browser build's `<iframe>` can be navigated and nothing more, so the history, redirect, script, handler, and cookie APIs are not compiled for the web build; gate code that uses them with `#[cfg(not(target_arch = "wasm32"))]`, as `examples/webview` does.

To use WaterUI's bundled Chromium Embedded Framework runtime instead, depend on `waterui-browser-cef` with its `webview` feature (as `examples/webview-cef` declares it) and install it in `app` before `App::new`:

```rust,ignore
use waterui::app::App;
use waterui::prelude::*;
use waterui::webview::WebView;
use waterui_browser_cef::install;

fn page() -> impl View {
    WebView::open("https://waterui.dev")
}

pub fn app(mut env: Environment) -> App {
    install(&mut env);
    App::new(page, env)
}
```

That is the whole selection: `install` starts the packaged runtime, supplies the `WebViewController` that opens CEF pages, and registers the realization that draws them. The view code is unchanged, which `examples/webview-cef` shows by reusing `examples/webview` verbatim. A browser engine plugs in through a `Hook<WebView>` in the environment, the same seam video and maps use for their self-drawn realizations. Engines belong to the repositories that ship them, so check the `waterui-browser-cef` documentation for the platforms it supports before you depend on it.

## When you want Chromium itself

`waterui-chromium` is a separate crate, neither a `WebView` engine nor re-exported by the `waterui` facade. Depend on it when you need a full Chromium surface, headless pages, screenshots, or the Chrome DevTools Protocol. It is paired with `waterui-browser-cef`'s `chromium` feature, and `install_chromium(&mut env)` puts a `ChromiumController` in the environment:

```rust,ignore
use waterui::prelude::*;
use waterui_chromium::{ChromiumConfiguration, ChromiumController, ScreenshotFormat};

fn scene(controller: &ChromiumController) -> impl View + use<> {
    let chromium = controller.open(ChromiumConfiguration::new("https://waterui.dev"));
    let page = chromium.page().clone(); // clone before the view moves into a layout
    // page.cdp() runs raw or typed DevTools commands;
    // page.screenshot(ScreenshotFormat::Png).await captures the page.
    chromium
}
```

`controller.headless(config).await` gives an off-screen page that you close yourself, and `page.watch(|event: ChromiumEvent| ..)` observes its lifecycle. Typed CDP comes through the re-exported `cdp` and `cdp_types` modules. Keeping all of this out of `waterui-webview` keeps the Chromium dependency graph out of applications that only wanted an embedded page. `examples/chromium` exercises each of these.

## Downcasting the handle

Backend authors and platform integrations can recover the concrete handle with `webview.handle().downcast_ref::<T>()`. Application code should not need it; if it does, the missing capability belongs on `WebViewHandle`.

---

Next: [Barcodes and QR codes](04-barcode.md), where the content is generated locally instead of fetched from the network.

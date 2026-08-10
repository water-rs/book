# WebView

> **In this chapter, you will:**
> - Embed web content with the deferred, reactive `WebView::open` entry point
> - Drive navigation from a URL binding instead of imperative calls
> - Reach the imperative surface — refresh, history, JavaScript — through `WebViewProxy`
> - Bridge web code back into Rust with script injection and message handlers
> - Pick a browser engine in `Water.toml`, including the bundled Chromium runtime

Documentation pages, OAuth flows, and existing web apps are already written. `waterui-webview` embeds them in your native shell and gives you navigation, cookies, JavaScript execution, and a Rust bridge from the same view tree as the rest of your UI.

> **Feature flag:** `webview` is **not** in the default feature set (`gpu`, `assets`, `media`, `flow-markdown`). Enable it explicitly:
>
> ```toml
> waterui = { version = "0.2", features = ["webview"] }
> ```
>
> The `waterui::webview` module — and its prelude re-export — only exist with that feature on.

## Opening a web view

```rust,ignore
use waterui::prelude::*;
use waterui::webview::WebView;

fn docs_page() -> impl View {
    WebView::open("https://waterui.dev/docs")
}
```

`WebView::open` does not create a browser. It returns a `WebViewOpen`, a deferred description; the native handle is created when the view renders and a `WebViewController` can be pulled from the live environment. That deferral is what lets a web view be embedded in a gallery cell or a `water preview` run without an application-level hand-off.

### The URL is a signal

`open` takes `impl IntoComputed<Str>`, so navigation is a state write rather than a method call:

```rust,ignore
use waterui::prelude::*;
use waterui::reactive::binding;
use waterui::webview::WebView;

fn browser() -> impl View {
    let url: Binding<Str> = binding("https://waterui.dev");

    vstack((field("Address", &url), WebView::open(url)))
}
```

Writing a new URL navigates the **existing** native web view. The signal is retained for the native view's lifetime, so nothing is torn down and no page state is lost — which is exactly why you must not wrap a web view in `watch` to change its address.

`WebViewOpen` carries two builders before it renders: `.redirects_enabled(signal)` for a reactive redirect policy, and `.with_proxy(...)`, covered next.

## Imperative controls with `WebViewProxy`

Refresh, stop, history navigation, and ad-hoc JavaScript have no natural reactive input. `WebViewOpen::with_proxy` renders your chrome above the web view and injects a `WebViewProxy` into that subtree's environment, so any handler inside can take one as a parameter — the same extractor machinery that supplies `State<T>` to `Button::action`.

```rust,ignore
use waterui::prelude::*;
use waterui::webview::{WebView, WebViewProxy};

fn mini_browser() -> impl View {
    WebView::open("https://waterui.dev").with_proxy(|| {
        hstack((
            button("Back").action(|p: WebViewProxy| p.go_back()),
            button("Forward").action(|p: WebViewProxy| p.go_forward()),
            button("Refresh").action(|p: WebViewProxy| p.refresh()),
            button("Stop").action(|p: WebViewProxy| p.stop()),
        ))
    })
}
```

The proxy exposes `go_back`, `go_forward`, `refresh`, `stop`, `run_javascript`, `inject_script`, `set_user_agent`, `set_redirects_enabled`, `set_cookie`, `get_cookies`, and `handle()`. Extracting one outside a `with_proxy` scope fails with an error that names the fix, so a forgotten wrapper surfaces as a handler error instead of a silent no-op.

## Owning the handle with `WebViewController`

Native backends install a `WebViewController` factory into the environment. Extract it when the surrounding view needs to hold the `WebView` value itself — to observe its event signal, or to hand it to several handlers as `State<WebView>`:

```rust,ignore
use waterui::env::use_env;
use waterui::prelude::*;
use waterui::webview::{WebView, WebViewController};

fn custom_browser() -> impl View {
    use_env(|controller: WebViewController| {
        let webview = controller.open();          // opens blank
        webview.go_to("https://book.waterui.dev");
        webview.set_user_agent("WaterUIBook/1.0");
        webview
    })
}
```

`use_env` panics when extraction fails, and backends without web support never install the controller. To degrade gracefully, look the controller up yourself inside a `View::body` and branch:

```rust,ignore
use waterui::prelude::*;
use waterui::webview::WebViewController;

struct Docs;

impl View for Docs {
    fn body(self, env: &Environment) -> impl View {
        match env.get::<WebViewController>().cloned() {
            Some(controller) => AnyView::new(controller.open()),
            None => AnyView::new(text("WebView is unavailable on this backend.")),
        }
    }
}
```

## Navigation state

`can_go_back()` and `can_go_forward()` return `Computed<bool>` that track the native history, so they feed `.disabled(...)` directly:

```rust,ignore
use waterui::prelude::*;
use waterui::webview::WebView;

fn back_button(webview: &WebView) -> impl View {
    button("Back")
        .action(|State(w): State<WebView>| w.go_back())
        .state(webview)
        .disabled(webview.can_go_back().map(|ok| !ok))
}
```

## Events

`WebView::event()` returns `impl Signal<Output = WebViewEvent>` covering the navigation lifecycle:

| Event | Fields | Meaning |
|---|---|---|
| `None` | — | Initial state, before anything happens |
| `WillNavigate` | `url: Url` | Navigation is about to begin |
| `Loading` | `progress: f32` | Load progress, 0.0 to 1.0 |
| `Loaded` | — | The page finished loading |
| `Redirect` | `from: Url, to: Url` | A redirect occurred |
| `Error` | `WebViewError` | Navigation or loading failed |

History-state changes are handled internally and never reach this signal; read `can_go_back()` / `can_go_forward()` for that.

`WebViewError` has three variants: `Network(Str)`, `Ssl { url, message }`, and `LoadFailed(Str)`.

## Running JavaScript

`run_javascript` executes in the loaded page and resolves to `Result<Str, Str>`. It is async and main-thread affine, so drive it from `action_async`:

```rust,ignore
use waterui::prelude::*;
use waterui::webview::WebViewProxy;

fn title_probe(output: &Binding<Str>) -> impl View {
    button("Get title")
        .action_async(|p: WebViewProxy, State(out): State<Binding<Str>>| async move {
            match p.run_javascript("document.title").await {
                Ok(title) => out.set(title),
                Err(err) => out.set(Str::from(format!("JS error: {err}"))),
            }
        })
        .state(output)
}
```

### Script injection

`run_javascript` runs after load. For code that must be present before the page's own scripts, inject it instead — injected scripts re-run on every page load:

```rust,ignore
use waterui::webview::ScriptInjectionTime;

// Runs before the DOM exists: bridges, global setup, request interception.
proxy.inject_script(include_str!("bridge.js"), ScriptInjectionTime::DocumentStart);

// Runs after the document is ready: DOM edits, event listeners.
proxy.inject_script(
    "document.body.dataset.wateruiHost = 'native';",
    ScriptInjectionTime::DocumentEnd,
);
```

Keep non-trivial JavaScript in its own `.js` file and pull it in with `include_str!` rather than as an inline multi-line literal.

## Calling Rust from JavaScript

Message handlers live on the type-erased handle, reachable as `proxy.handle()` or `webview.handle()`:

```rust,ignore
webview.handle().add_handler("greet", Box::new(|data: &[u8]| {
    let name = String::from_utf8_lossy(data);
    tracing::info!(%name, "greet called from JavaScript");
    format!("Hello, {name}!").into_bytes()
}));

// Later:
webview.handle().remove_handler("greet");
```

The handler takes bytes and returns bytes. The JavaScript side differs per platform:

```javascript
// Apple (WKWebView)
window.webkit.messageHandlers.greet.postMessage("World");

// Android
window.greet.postMessage("World");
```

Hide that asymmetry by shipping a small `DocumentStart` script that defines one API for your web code and dispatches to whichever host object exists.

## Cookies

```rust,ignore
use waterui::webview::{Cookie, WebView};

fn set_session_cookie(webview: &WebView, session_token: String) {
    let cookie = Cookie::build(("session", session_token))
        .domain("book.waterui.dev")
        .path("/")
        .secure(true)
        .build();

    webview.set_cookie(cookie);
}
```

`Cookie` is the `cookie` crate's type, re-exported as `waterui::webview::Cookie` (the crate itself is not re-exported, so import the type, not the module). Reading is asynchronous so the UI thread never blocks on the native cookie store:

```rust,ignore
for c in webview.get_cookies().await {
    tracing::info!(name = c.name(), value = c.value(), "cookie");
}
```

## Choosing a browser engine

`Water.toml` selects the engine that backs `WebView`:

```toml
webview_backend = "default"   # default | system | wpe | cef
```

| Value | Engine | Where it works |
|---|---|---|
| `default` | Bundled WPE on Linux, system engine everywhere else | — |
| `system` | Platform web view (WKWebView, Android WebView, WebKitGTK) | Apple platforms, Android, Linux + GTK4, web |
| `wpe` | WaterUI's bundled WPE WebKit runtime | Linux, with GTK4 or Hydrolysis |
| `cef` | WaterUI's bundled Chromium Embedded Framework runtime | macOS, Linux, Windows — any backend except Dew |

CEF is independent of the WaterUI rendering backend: the engine choice and the renderer choice are separate axes. Dew excludes it deliberately, because a constrained-device target cannot carry the Chromium runtime.

An unsupported combination is a build error, not a silent downgrade. Selecting an engine also never adds a runtime on its own — the CLI links one only if your application actually links `waterui-webview`. On macOS the CLI generates the required `waterui-cef-helper` binaries and helper `Info.plist` for a CEF app automatically.

The shipped `examples/webview-cef` proves the point: it reuses `examples/webview` unchanged and differs only by `webview_backend = "cef"` in `Water.toml`.

## When you want Chromium itself

`waterui-chromium` is a separate crate — not a `WebView` engine and not re-exported through the `waterui` facade. Depend on it directly when you need headless pages, screenshots, or the Chrome DevTools Protocol: it exposes `Chromium`, `chromium()`, `ChromiumPage`, `ChromiumConfiguration`, `ChromiumProfile`, `ChromiumProxy`, `ScreenshotFormat`, `CdpSession`, and typed CDP through the re-exported `cdp` / `cdp_types` modules. Keeping it out of `waterui-webview` keeps the Chromium dependency graph out of applications that only wanted an embedded page.

## Sizing

`WebView` is a raw view declared with `StretchAxis::Both`, so it fills the space it is given. Constrain it with `.size(width, height)`, `.width(...)`, `.height(...)`, or the surrounding layout.

## Downcasting the handle

Backend authors and platform integrations can recover the concrete handle:

```rust,ignore
if let Some(native) = webview.handle().downcast_ref::<MyNativeHandle>() {
    // Configure platform-specific preferences.
}
```

Application code should not need this; if it does, the missing capability belongs on `WebViewHandle`.

---

Next: [Barcodes and QR Codes](04-barcode.md), where the content is generated on the GPU instead of fetched from the network.

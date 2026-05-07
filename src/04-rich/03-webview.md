# WebView

> **In this chapter, you will:**
> - Embed web content directly inside your WaterUI application
> - Navigate, inject scripts, and execute JavaScript from Rust
> - Set up bidirectional communication between Rust and web code
> - Manage cookies and redirect behavior programmatically
> - Build a minimal in-app browser with back/forward controls

Sometimes the best tool for the job is the web. Maybe you need to display documentation, embed an OAuth login flow, or wrap an existing web app inside your native shell. The `waterui-webview` crate makes this seamless -- you get a full-featured embedded browser with navigation, JavaScript execution, cookie management, and a Rust-to-JS bridge, all from Rust.

> **Feature flag:** WebView lives behind the `webview` feature on `waterui`. Enable it in `Cargo.toml` (`waterui = { version = "...", features = ["webview"] }`) before importing `waterui::webview`.

## Architecture

The WebView system follows a layered design:

| Layer | Type | Role |
|---|---|---|
| Trait | `WebViewHandle` | Imperative API that native backends implement |
| Type-erased wrapper | `AnyWebViewHandle` | Wraps any `WebViewHandle` with downcast support |
| Factory | `WebViewController` | Environment-injected factory for creating web views |
| Reactive view | `WebView` | Combines `AnyWebViewHandle` with `Binding` state |

Native backends (Apple, Android) implement `CustomWebViewController` and
inject a `WebViewController` into the `Environment` at startup. Your
application code then obtains the controller and creates web views through it.

---

## Quick Start

The simplest way to embed web content is with `WebView::open`:

```rust,ignore
use waterui_webview::WebView;

fn docs_page() -> impl View {
    WebView::open("https://waterui.dev/docs")
}
```

`WebView::open` pulls the `WebViewController` from the environment
automatically, creates a new web view handle, navigates to the URL, and
returns a `View` that renders the embedded browser.

That is all it takes -- one line to go from URL to rendered web content.

---

## Creating a WebView Manually

For more control, obtain the controller from the environment, open a fresh
`WebView`, and configure it before placing it in the view hierarchy:

```rust,ignore
use waterui::prelude::*;
use waterui_webview::{WebView, WebViewController};

fn custom_browser() -> impl View {
    use_env(|controller: WebViewController| {
        let webview = controller.open();
        webview.go_to("https://example.com");
        webview.set_user_agent("MyApp/1.0");
        webview
    })
}
```

`WebViewController::open()` returns a fresh `WebView` already wrapped with
reactive event state. Use the `WebView` itself as the imperative handle.

### `open_then` for Post-Creation Configuration

When you want to configure the underlying handle immediately after creation
but still build the view in a single expression, use `WebView::open_then`:

```rust,ignore
use waterui_webview::WebView;

fn configured_webview() -> impl View {
    WebView::open_then("https://example.com", |handle| {
        handle.set_user_agent("MyApp/1.0");
        handle.set_redirects_enabled(false);
    })
}
```

The closure receives an `AnyWebViewHandle`, which exposes the same imperative
API as `WebView` (navigation, user agent, cookie store, script injection).

---

## Navigation

Once you have a `WebView` instance, control navigation imperatively:

```rust,ignore
// Navigate to a URL
webview.go_to("https://example.com");

// Refresh the current page
webview.refresh();

// Stop loading
webview.stop();

// History navigation
webview.go_back();
webview.go_forward();
```

### Reactive Navigation State

`WebView` exposes reactive signals for history state:

```rust,ignore
// Returns Computed<bool>
let can_back = webview.can_go_back();
let can_forward = webview.can_go_forward();
```

These update automatically as the user navigates. Use them to enable/disable
back and forward buttons in your custom browser chrome.

---

## Events

Subscribe to navigation lifecycle events through the reactive `event()` signal:

```rust,ignore
use waterui_webview::{WebView, WebViewEvent};

let webview = WebView::new(handle);

// Watch events reactively
webview.event(); // returns impl Signal<Output = WebViewEvent>
```

### `WebViewEvent` Variants

| Event | Fields | Description |
|---|---|---|
| `None` | -- | Initial state before any event fires |
| `WillNavigate` | `url: Url` | Navigation is about to begin |
| `Loading` | `progress: f32` | Page load progress (0.0 to 1.0) |
| `Loaded` | -- | Page finished loading |
| `Redirect` | `from: Url, to: Url` | A redirect occurred during navigation |
| `Error(WebViewError)` | -- | An error occurred |

### Error Types

```rust,ignore
use waterui_webview::WebViewError;
```

| Error | Description |
|---|---|
| `WebViewError::Network(msg)` | A network error occurred |
| `WebViewError::Ssl { url, message }` | An SSL/TLS verification failure |
| `WebViewError::LoadFailed(msg)` | The page failed to load |

---

## JavaScript Execution

One of the most powerful features of the WebView is the ability to run JavaScript from Rust and get results back.

### Running Scripts

Execute JavaScript in the context of the loaded page:

```rust,ignore
let result = webview.run_javascript("document.title").await;
match result {
    Ok(title) => tracing::info!("Page title: {title}"),
    Err(err) => tracing::error!("JS error: {err}"),
}
```

`run_javascript` is async and returns `Result<Str, Str>`. It executes
**after** the page has loaded. For scripts that must run before the DOM is
constructed, use script injection instead.

### Script Injection

Inject scripts that run automatically on every page load:

```rust,ignore
use waterui_webview::ScriptInjectionTime;

// Run before DOM construction
webview.inject_script(
    r#"window.APP_VERSION = "1.0.0";"#,
    ScriptInjectionTime::DocumentStart,
);

// Run after DOM is ready
webview.inject_script(
    r#"document.body.style.backgroundColor = "#f0f0f0";"#,
    ScriptInjectionTime::DocumentEnd,
);
```

| Injection Time | Description | Use Cases |
|---|---|---|
| `DocumentStart` | Before the DOM is constructed | Native bridges, global object setup, request interception |
| `DocumentEnd` | After the document finishes loading | DOM manipulation, event listeners |

---

## Rust-to-JavaScript Bridge

The WebView supports bidirectional communication through message handlers. This is how you connect your Rust business logic to your web UI.

### Setting Up a Handler

Register a Rust function that JavaScript can call:

```rust,ignore
webview.handle().add_handler("greet", Box::new(|data: &[u8]| {
    let name = String::from_utf8_lossy(data);
    tracing::info!("Greeting requested for: {name}");
    format!("Hello, {name}!").into_bytes()
}));
```

### Calling from JavaScript

The JavaScript API depends on the platform:

```javascript
// Apple (WKWebView)
window.webkit.messageHandlers.greet.postMessage("World");

// Android
window.greet.postMessage("World");
```

### Setting Up a Convenient Bridge

Combine `inject_script` and `add_handler` for a clean API that hides platform differences from your web code:

```rust,ignore
use waterui_webview::ScriptInjectionTime;

// Inject a friendly JavaScript API
webview.inject_script(r#"
    window.myApp = {
        greet: function(name) {
            window.webkit.messageHandlers.greet.postMessage(name);
        }
    };
"#, ScriptInjectionTime::DocumentStart);

// Register the native handler
webview.handle().add_handler("greet", Box::new(|data: &[u8]| {
    let name = String::from_utf8_lossy(data);
    tracing::info!("JS called greet({name})");
    Vec::new()
}));
```

### Removing a Handler

```rust,ignore
webview.handle().remove_handler("greet");
```

---

## Cookies

Manage cookies programmatically -- useful for authentication flows or session management:

```rust,ignore
use waterui_webview::cookie::Cookie;

// Set a cookie
let cookie = Cookie::build(("session", "abc123"))
    .domain("example.com")
    .path("/")
    .secure(true)
    .build()
    .unwrap();

webview.handle().set_cookie(cookie);

// Retrieve all cookies
let cookies = webview.handle().get_cookies();
for c in &cookies {
    tracing::info!("Cookie: {} = {}", c.name(), c.value());
}
```

The `cookie` crate (re-exported as `waterui_webview::cookie`) provides the
`Cookie` type.

---

## Redirect Control

Enable or disable HTTP redirect following:

```rust,ignore
// Imperatively
webview.set_redirects_enabled(false);

// Reactively via a binding
let allow_redirects = Binding::bool(true);
let webview = WebView::new(handle)
    .redirects_enabled(allow_redirects.into_computed());
```

The `redirects_enabled` builder method watches the signal and syncs the
setting automatically when the value changes.

---

## User Agent

Customize the user agent string sent with requests:

```rust,ignore
webview.set_user_agent("MyApp/1.0 (WaterUI)");
```

---

## Complete Example

Let's put it all together. Here is a minimal in-app browser with back/forward buttons:

```rust,ignore
use waterui::prelude::*;
use waterui_webview::{WebView, WebViewController};

fn mini_browser() -> impl View {
    use_env(|controller: WebViewController| {
        let webview = controller.open();
        webview.go_to("https://waterui.dev");

        let can_back = webview.can_go_back();
        let can_forward = webview.can_go_forward();

        let back = webview.clone();
        let forward = webview.clone();
        let reload = webview.clone();

        vstack((
            hstack((
                button("Back")
                    .action(move || back.go_back())
                    .disabled(can_back.map(|ok| !ok)),
                button("Forward")
                    .action(move || forward.go_forward())
                    .disabled(can_forward.map(|ok| !ok)),
                button("Refresh").action(move || reload.refresh()),
            )),
            webview,
        ))
    })
}
```

The `disabled` modifier accepts the inverted reactive signal, so the back and
forward buttons grey out automatically as the navigation history changes.

---

## Platform Considerations

| Feature | Apple | Android | Desktop |
|---|---|---|---|
| Engine | WKWebView (WebKit) | Platform WebView | WIP |
| JavaScript execution | Full support | Full support | -- |
| Script injection | DocumentStart / DocumentEnd | DocumentStart / DocumentEnd | -- |
| Message handlers | `webkit.messageHandlers` | `window.<name>` | -- |
| Cookies | Full support | Full support | -- |
| Redirect control | Full support | Backend-dependent | -- |

`WebView` is declared as a *raw view* with `StretchAxis::Both`, so it
expands to fill all available space by default. Wrap it in a `.frame()` modifier
or constrain it with layout containers to control its size.

### Downcasting the Handle

When you need access to the platform-specific handle (for example, to configure
WKWebView preferences on Apple), you can downcast:

```rust,ignore
if let Some(native) = webview.handle().downcast_ref::<MyNativeHandle>() {
    // Access platform-specific APIs
}
```

This is primarily useful for backend authors and advanced platform integration.

---

## What's Next

You have seen how to embed the entire web inside your app. Next, let's look at something more focused: [Barcodes and QR Codes](04-barcode.md), where you will generate scannable codes entirely on the GPU.

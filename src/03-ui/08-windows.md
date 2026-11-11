# Windows and the app lifecycle

> **In this chapter, you will:**
>
> - Declare an application with one window, several windows, or none, using `App`.
> - Decide what happens when the last window closes with `LastWindowPolicy`.
> - Shape a window with the `Window` builder: size limits, placement, level, style, background, toolbar and icon.
> - Open, close and drive windows at runtime with `WindowState`, `conditional_window`, `WindowManager` and `WindowHandle`.
> - Intercept a window close and an application quit, and run shutdown work, with `CloseReply`, `QuitReply` and `on_terminate`.

A view describes what is inside a window. Everything around it, meaning the
title bar, the size limits, when a second window opens and what happens when
the user quits, belongs to the application. WaterUI keeps that in two types:
`App` declares the program, `Window` declares one top-level window.

The types in this chapter live in `waterui::app` and `waterui::window`. The
prelude brings in those two module names but none of their types, so import
what you name.

## The smallest app

An app with one window needs a root view and the environment the runner hands
you:

```rust
use waterui::app::App;
use waterui::prelude::*;

pub fn app(env: Environment) -> App {
    App::new(|| text!("Hello, window"), env)
}
```

`App::new` opens a single window that shows the content immediately. The window
has no title of its own, which means the platform shows it under the
application's own name. To title it, declare the window yourself.

## Declaring windows

`App::new_with_windows` takes the windows and the environment. A `Window` is
built with `Window::new(title, state, content)`:

- `title` is anything that converts into a `Computed<Str>`, so a reactive title
  is free.
- `state` is a `Binding<WindowState>` that you pass by value. It is both the
  window's current state and the switch that opens it.
- `content` is any `ViewBuilder`, usually a closure or a function item.

```rust
use waterui::app::App;
use waterui::prelude::*;
use waterui::window::{Window, WindowState};

fn editor() -> impl View {
    text!("Editor")
}

fn inspector() -> impl View {
    text!("Inspector")
}

pub fn app(env: Environment) -> App {
    App::new_with_windows(
        [Window::new("Editor", binding(WindowState::Normal), editor)],
        env,
    )
    .window(Window::new(
        "Inspector",
        binding(WindowState::Normal),
        inspector,
    ))
}
```

`App::window` appends one more window to the list. A runner opens every
declared window at startup, in declaration order, and none of them is special.

`WindowState` has five variants: `Normal`, `Closed`, `Minimized`, `Maximized`
and `Fullscreen`. Its `Default` is `Closed`. That is why `binding(WindowState::Normal)`
makes a window that is open from the first frame, while
`binding::<WindowState>(WindowState::default())` makes one that waits to be
opened. `binding` accepts any `impl Into<T>`, so write the turbofish wherever
nothing downstream pins `T`, such as a binding stored in a `let` before it
reaches `Window::new`.

## What happens when the last window closes

Platforms disagree about this. A Linux or Windows application ends with its
last window. A macOS application usually stays in the Dock with its menu bar.
WaterUI makes the application say which it wants, with `LastWindowPolicy`:

| Policy | Behavior |
| --- | --- |
| `Quit` (default, every platform) | The application ends when its last window closes, and at startup if it declares none. |
| `StayResident` | The application keeps running with no window until it quits explicitly or the system asks it to. |

A tray or status-item application, or one that opens its windows on demand,
declares no window and stays resident:

```rust
use waterui::app::{App, LastWindowPolicy};
use waterui::prelude::*;
use waterui::window::Window;

pub fn app(env: Environment) -> App {
    App::new_with_windows(Vec::<Window>::new(), env)
        .on_last_window_closed(LastWindowPolicy::StayResident)
}
```

iOS and Android have no windowless foreground state. Their runners reject an
application that declares no window, whatever its policy. A resident
application's commands usually live in the menu bar, covered in
[the menus chapter](09-menus.md).

## Shaping a window

`Window` is a builder. Each method consumes the window and returns it, so you
chain them. The ones you will use most:

| Method | What it sets |
| --- | --- |
| `.title(..)` | The title, `impl IntoComputed<Str>`. |
| `.resizable(bool)` | Whether the user can resize it. Default `true`. |
| `.min_size(..)` / `.max_size(..)` | One `impl IntoComputed<Size>` each. |
| `.style(..)` | A `WindowStyle` or a `Binding<WindowStyle>`. |
| `.background(..)` | A `Color`, a `Material`, a `WindowBackground` or a `Binding<WindowBackground>`. |
| `.toolbar(..)` | Window-level chrome, any `View`. |
| `.level(..)` | `WindowLevel::Normal` or `WindowLevel::AlwaysOnTop`, or a signal of one. |
| `.placement(..)` | Which monitor, and the frame on it. |
| `.activation(..)` | Whether showing or clicking takes keyboard focus. |
| `.resize_increments(..)` | Snap the content size to steps while resizing. |
| `.icon(..)` | A `WindowIcon`, an `Option<WindowIcon>` or a `Binding<Option<WindowIcon>>`. |

```rust
use waterui::background::Material;
use waterui::layout::{Rect, Size};
use waterui::prelude::*;
use waterui::window::{
    Monitor, MonitorSelector, Window, WindowIcon, WindowLevel, WindowState, WindowStyle,
};

fn palette_window(icon: WindowIcon) -> Window {
    Window::new("Palette", binding(WindowState::Normal), || text!("Palette"))
        .style(WindowStyle::FullSizeContentView)
        .background(Material::UltraThin)
        .min_size(Size::new(320.0, 240.0))
        .max_size(Size::new(640.0, 480.0))
        .level(WindowLevel::AlwaysOnTop)
        .placement(MonitorSelector::Pointer, |monitor: &Monitor| {
            let area = monitor.visible_frame();
            Rect::new(area.origin(), Size::new(480.0, 320.0))
        })
        .toolbar(button("Refresh").action(|| waterui::log::info!("refresh palette")))
        .icon(icon)
}
```

`WindowIcon::new(width, height, rgba)` takes straight-alpha RGBA8 pixels, row
by row, `width * height * 4` bytes in total. It returns a `Result` whose
`WindowIconError` names the problem (no pixels, too large, or a buffer of the
wrong length), so decode the icon where you load the asset and handle the error
there. `app` itself must return an `App`, not a `Result`.

A few details that matter in practice.

**Style is not background.** `WindowStyle` has exactly three variants: `Titled`
(the default), `Borderless`, and `FullSizeContentView`, where content extends
under the title bar. A frosted or transparent window is a *background*, not a
style. A translucent `Color` makes the window transparent, and a `Material`
makes it the window's material. Within-window material levels (`Regular`,
`Thick`, `UltraThick`) frost the window's own opaque background. Behind-window
levels (`UltraThin`, `Thin`) let the desktop show through, blurred only where
the platform supports it. `WindowBackground::Opaque`, the default, is the
theme background.

**Size limits.** Without `.min_size`, the backend derives a minimum by
measuring each axis of your content with a zero proposal, so the window can
never be resized smaller than its content. Explicit limits are enforced on
desktop backends. iOS and iPadOS have no per-window size control, and Android's
single-activity model has no runtime size limits, so both ignore them.

**Placement.** `.placement(monitor, place)` resolves the `MonitorSelector`
(`Primary`, `Pointer` or `Focused`) each time the window is shown, hands your
closure the resolved `Monitor`, and uses the returned `Rect` as the frame. A
`Monitor` exposes `frame()`, `visible_frame()` (minus menu bar, dock and
panels), `scale_factor()` and `name()`. On Wayland, compositors do not expose
absolute window positions, so only the size part of the rect applies.
Placement and `.activation(..)` are realized by the Hydrolysis desktop backend;
the AppKit backend does not apply them.

**Icon.** A per-window icon shows on X11 and on Windows. Wayland, macOS, iOS and
Android always show the application icon, so `None`, the default, is the right
choice there.

**Toolbar.** `.toolbar(..)` installs `LabelDisplayMode::IconOnly` for its
items, so a semantic label collapses to its icon while keeping its
accessibility text. A single `Label` opts back in with `.title_only()` or
`.title_and_icon()`.

## Opening and closing windows at runtime

Because `WindowState` lives in a binding, opening, closing, minimizing and
maximizing are ordinary reactive state changes. For a window that opens on
demand, create two pieces together, at the same level as the state: the
binding and a `WindowPresentation` that guards against duplicate opens. Then
place `conditional_window` in the tree.

```rust
use waterui::prelude::*;
use waterui::window::{Window, WindowPresentation, WindowState, conditional_window};

fn inspector_content() -> impl View {
    text!("Inspector")
}

pub fn launcher() -> impl View {
    let state = binding::<WindowState>(WindowState::default());
    let presentation = WindowPresentation::new(&state);

    zstack((
        button("Open inspector")
            .action(|State(state): State<Binding<WindowState>>| state.set(WindowState::Normal))
            .state(&state),
        // An invisible view: it must still be placed in the tree.
        conditional_window(&presentation, |state| {
            Window::new("Inspector", state, inspector_content)
        }),
    ))
}
```

`conditional_window` creates the native window when the state leaves `Closed`
and tears it down when the state returns to `Closed`. Moving `Normal` to
`Minimized` does not create a duplicate, and it re-arms after the window
closes. Create the binding and the `WindowPresentation` outside any subtree
that rebuilds, or the open-once guard resets.

The presentation lives only as long as the window hosting the
`conditional_window` view. A window that must outlive every other window, such
as one reopened under `StayResident` after the last window closed, is opened
with `Window::show(env)`. It looks up the `WindowManager` service in the
environment and asks it to show the window. `Environment` is an extractor, so
an action can take it directly:

```rust
use waterui::prelude::*;
use waterui::window::{Window, WindowState};

pub fn open_button() -> impl View {
    button("New window").action(|env: Environment| {
        Window::new("Scratch", binding(WindowState::Normal), || text!("Scratch")).show(&env);
    })
}
```

An environment without a `WindowManager` breaks the contract: a debug build
panics, and a release build logs the error and shows nothing.
`conditional_window` shows its window through the same service. On iOS the
installed `WindowManager` panics on every request, because iOS opens windows
as scenes (see below), not on request, so both `Window::show` and
`conditional_window` panic there. Leave a "new window" command out of an iOS
build.

## Driving a window with its handle

`Window::handle()` returns a `WindowHandle` that stays valid after the window is
shown. It is cheap to clone. It can `close()`, `minimize()`, `maximize()`,
`fullscreen()` and `restore()`, which write the matching `WindowState`; set the
`frame`, `style`, `background` and `icon` after the fact; ask for attention with
`request_attention(UserAttention::Informational | UserAttention::Critical)`
and withdraw it with `cancel_attention()`; and bring the window forward with
`focus()`.

The handle only exists once the `Window` does, but a window's own content often
needs it. Replace the content builder after taking the handle:

```rust
use waterui::handler::AnyViewBuilder;
use waterui::prelude::*;
use waterui::window::{UserAttention, Window, WindowHandle, WindowState};

fn controls(handle: &WindowHandle) -> impl View {
    vstack((
        button("Maximize").action({
            let handle = handle.clone();
            move || handle.maximize()
        }),
        button("Restore").action({
            let handle = handle.clone();
            move || handle.restore()
        }),
        button("Bring to front").action({
            let handle = handle.clone();
            move || handle.focus()
        }),
        button("Ask for attention").action({
            let handle = handle.clone();
            move || handle.request_attention(UserAttention::Informational)
        }),
    ))
}

pub fn controlled_window() -> Window {
    let mut window = Window::new("Controls", binding(WindowState::Normal), || ());
    let handle = window.handle();
    window.content = AnyViewBuilder::new(move || AnyView::new(controls(&handle)));
    window
}
```

Replacing `content` replaces everything `Window::new` put into the builder,
including the snackbar and full-screen overlay managers it wires around your
view. A `SnackbarManager` or `FullScreenOverlayManager` handler inside this
window's content finds no manager, so use this pattern only for content that
needs neither.

`focus()` is the classic "show me the existing settings window" request. It is
best-effort on some desktop window managers, and on Wayland it needs an
activation token that exists only when the request follows a user gesture. On
macOS it deminiaturizes a minimized window first, and the system may still
decline the activation; on iOS it makes the window its scene's key window. On
headless and semantic runners, and when the window has
not been realized yet, `focus()` panics, so file it from an action that runs
after the window is on screen.

`request_attention` lasts until the user focuses the window, at which point the
backend clears it. The `multi_window` example in the WaterUI repository drives
`maximize`, `restore`, `request_attention` and `cancel_attention` through a
handle this way.

## Confirming a close

A terminal with a running shell or an editor with unsaved buffers must be able
to refuse a close. `Window::on_close_request` takes a handler that answers a
`CloseReply`, either `Close` or `Cancel`, from its future. The handler extracts
from the environment like any action handler, and its future runs on the
runner's local executor.

```rust
use waterui::prelude::*;
use waterui::window::{CloseReply, Window, WindowState};

pub fn editor_window(unsaved: Binding<bool>, confirm_discard: Binding<bool>) -> Window {
    Window::new("Editor", binding(WindowState::Normal), || text!("Editor")).on_close_request(
        move || {
            let ask = unsaved.snapshot();
            // The editor shows a "Discard changes?" prompt while this is set.
            confirm_discard.set(ask);
            async move {
                if ask {
                    CloseReply::Cancel
                } else {
                    CloseReply::Close
                }
            }
        },
    )
}
```

The title-bar close button, the window manager's close, a Close Window menu
command and `WindowHandle::request_close()` all go through the handler. Writing
`WindowState::Closed` to the binding, or calling `WindowHandle::close()`, closes
without asking, which is how the window closes itself once the user confirms
the prompt. A request that arrives while a question is already open is dropped.
Quitting the whole application asks `on_quit_request`, not the windows.

A window declared with `closable` false drops the request before the handler
runs. `closable` is a public field, not a builder method. On macOS and Windows
the close button is disabled. On Linux the button stays enabled but its request
is ignored.

The handler runs on Hydrolysis desktop and AppKit. Hydrolysis on Android, UIKit
and the web have no window close request, so the handler never runs there and
`request_close()` panics.

## Quitting and shutdown work

The same pattern covers the application. `App::on_quit_request` is asked
before a *cancellable* termination: the user choosing Quit, the platform's quit
gesture, a declared `MenuItem::Quit`, or `Quit::request`. It answers a
`QuitReply`, either `Quit` or `Cancel`. A *required* termination never asks: a
termination signal, the last window closing under `LastWindowPolicy::Quit`, or
a Windows session ending.

`App::on_terminate` runs exactly once when the app is actually ending. The
runner waits for its future to finish before tearing down, so this is where you
persist state and release resources.

Both handlers run under the application's environment, not a view's. `App::state`
installs a value there, so a handler can extract it the way view handlers
extract `#[state]` types:

```rust
use waterui::app::{App, QuitReply};
use waterui::prelude::*;

#[waterui::state]
#[derive(Clone)]
struct Document {
    unsaved: Binding<bool>,
}

pub fn app(env: Environment) -> App {
    let document = Document {
        unsaved: binding(false),
    };

    App::new(|| text!("Document"), env)
        .state(&document)
        .on_quit_request(|document: Document| {
            let unsaved = document.unsaved.snapshot();
            async move {
                if unsaved {
                    QuitReply::Cancel
                } else {
                    QuitReply::Quit
                }
            }
        })
        .on_terminate(|document: Document| async move {
            waterui::log::info!(unsaved = document.unsaved.snapshot(), "shutting down");
        })
}
```

A cancelled quit gives the user no feedback by itself, so pair `Cancel` with UI
that says why, as the `menu` example does with a status line. Declare
`MenuItem::Quit` to place the standard Quit item: it renders once, with the
platform's label and chord, never appears in the macOS menu bar (whose
application menu already carries Quit), and is omitted on platforms with no
application quit. Do not rebuild it as a plain command, which would bypass the
hooks; see [Quit and Close Window](09-menus.md#quit-and-close-window). To quit
from your own button, extract `Quit`; it files the same cancellable request, so
`on_quit_request` still decides:

```rust
use waterui::app::Quit;
use waterui::prelude::*;

pub fn quit_button() -> impl View {
    button("Quit").action(|quit: Quit| quit.request())
}
```

`Quit` exists only where the runner has an application quit. iOS, Android and
the web end the process without notice and never call either hook. They, and
the headless hosts, install no `Quit`, so extracting it there fails and the
action does not run.

## iPadOS scenes

On iPadOS you do not open windows; the system does. Each scene the system
connects builds a window of its own.

The Apple backend assigns declarations in order. The first scene takes the first
declared window, the second takes the second, and any scene beyond the declared
windows realizes the first declaration again. Each scene builds its content
from the builder again, so anything created inside the builder, such as the
snackbar and overlay managers `Window::new` wires in, belongs to that scene
alone: a snackbar shown from one scene never appears in another. A binding the
builder captures from outside is the same binding in every scene, so a change to
it shows in all of them.

The `multi_scene` example shows the shape. Its snackbar appears only in the
scene whose button was pressed:

```rust
use waterui::app::App;
use waterui::prelude::*;
use waterui::snackbar::{Snackbar, SnackbarManager};

fn scene() -> impl View {
    vstack((
        text!("Every scene builds this content fresh."),
        button("Show snackbar").action(|manager: SnackbarManager| {
            manager.show(Snackbar::new("Only this scene's snackbar"));
        }),
    ))
    .padding()
}

pub fn app(env: Environment) -> App {
    App::new(scene, env)
}
```

Because iOS owns window creation, several window features do nothing there:
size limits and close requests are ignored, and `Window::show` and
`conditional_window` panic. Plan the iPad layout around adaptive views from
[the layout chapter](02-layout.md), not around window frames.

## Where to go next

Window chrome pairs with the menu bar and toolbar commands in
[the menus chapter](09-menus.md), and with the navigation containers in
[the navigation chapter](07-navigation.md). Dialogs, popovers and the
snackbars each window carries are in [the overlays chapter](10-overlays.md). For running one app across devices, see
[the platforms chapter](../01-getting-started/05-platforms.md).

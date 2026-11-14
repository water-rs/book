# Dialogs, popovers, and snackbars

> **In this chapter, you will:**
> - Ask for confirmation with a `Dialog` driven by a `Binding<bool>`, using default, destructive, and cancel actions
> - Show transient messages with `SnackbarManager`, including an undo action and a dismissal callback
> - Present a popover or tooltip next to any view with `.anchored_overlay`
> - Put your own window-filling content above a window's content with `FullScreenOverlayManager`

Some UI exists only for a moment and sits above everything else: a "Delete this draft?" question, a "File saved" toast, a popover hanging off a button. You never place any of these in your layout. A dialog or a popover is a value you attach to a view; a snackbar or a full-screen overlay goes through a manager the window installs. You say what to present and when; the backend decides where and how it looks. On Apple platforms a `Dialog` becomes a native alert, while the self-drawn Hydrolysis backend (which also covers Android) draws its own card on a modal layer.

| You want | Use | Feature |
|---|---|---|
| A question the user must answer | `Dialog` with `.dialog(...)` | `dialog` (default) |
| A brief message that disappears | `Snackbar` through `SnackbarManager` | `snackbar` (default) |
| Content attached to a view | `AnchoredOverlay` with `.anchored_overlay(...)` | always available |
| Your own full-window content | `FullScreenOverlayManager` | always available |

All four are presented from state you own. If you have not met bindings yet, read [the reactive chapter](../02-core/02-reactive.md) first.

## Dialogs

A `Dialog` is presented while a `Binding<bool>` is `true`. Attach it to a view with the `.dialog` modifier; the view it is attached to does not matter for placement, because the dialog is window-modal wherever it is declared.

```rust
use waterui::prelude::*;

fn discard_button() -> impl View {
    let confirm = binding(false);

    button("Discard draft")
        .action({
            let confirm = confirm.clone();
            move || confirm.set(true)
        })
        .dialog(
            Dialog::new(&confirm, "Discard changes?")
                .message("Your edits will be lost.")
                .action(DialogAction::destructive("Discard", || {
                    waterui::log::debug!("draft discarded");
                }))
                .action(DialogAction::cancel("Keep editing", || {})),
        )
}
```

`Dialog::new(&Binding<bool>, impl IntoText)` takes the binding by reference and clones it. The title is `IntoText`, so a `&'static str` goes through the translation catalog and `text!` keeps the title live while the dialog is up (see [the text chapter](01-text.md)). `.message(...)` adds supporting text under the title.

### Actions and roles

Each `.action(...)` takes a `DialogAction`. There are three constructors, one per `DialogRole`:

| Constructor | Role | Behavior |
|---|---|---|
| `DialogAction::new(title, handler)` | `DialogRole::Default` | An ordinary confirmation. The first one is the primary action, bound to Return where a hardware Return key exists. |
| `DialogAction::destructive(title, handler)` | `DialogRole::Destructive` | A destructive confirmation, drawn with the error emphasis on Hydrolysis and as a destructive button on Apple. |
| `DialogAction::cancel(title, handler)` | `DialogRole::Cancel` | The dismissive answer. Escape, the Android back gesture, and a tap on the Hydrolysis scrim all run this action's handler. |

A few rules follow from how the dialog is realized:

- Titles are plain text (`IntoText`), not labels. Native alert buttons carry a title only, so an icon would be dropped on Apple.
- Handlers take the same arguments a button action does: no arguments, or values extracted from the environment.
- Choosing an action runs its handler, then the backend writes `false` to your binding. You do not reset it yourself.
- Declaration order is kept within a role, but the backend lays roles out in platform order: dismissive first, affirmative last. You cannot reorder across roles.
- Declare at most one `Cancel` action. A second one panics in a debug build and is logged in a release build.
- A dialog with no actions shows a single acknowledgement button, with role `Cancel`.
- A dialog with no `Cancel` action must be answered: Escape and the scrim do nothing.

Setting the binding to `false` from your own code dismisses the dialog without running any handler. Only one dialog is presented per window at a time; a second one waits behind the first.

While a dialog is up, the content beneath it is inert to pointer, keyboard, and accessibility input, and focus moves into the dialog and returns to your content afterwards.

### Reacting to the answer

A handler is ordinary Rust, so the way to carry an answer back into the UI is another binding:

```rust
use waterui::prelude::*;

fn paste_with_confirmation() -> impl View {
    let confirm = binding(false);
    let status: Binding<Str> = binding("Nothing pasted yet");

    vstack((
        button("Paste").action({
            let confirm = confirm.clone();
            move || confirm.set(true)
        }),
        text(status.clone()),
    ))
    .dialog(
        Dialog::new(&confirm, "Paste multiple lines?")
            .message("The clipboard holds more than one line.")
            .action(DialogAction::new("Paste", {
                let status = status.clone();
                move || status.set(Str::from("Pasted"))
            }))
            .action(DialogAction::cancel("Cancel", {
                let status = status.clone();
                move || status.set(Str::from("Cancelled"))
            })),
    )
}
```

## Snackbars

A snackbar is a short message with an optional action, shown for a few seconds and then removed. You do not place it in your view tree. Every `Window` installs a `SnackbarManager` into its environment, and a handler asks for it by type:

```rust
use waterui::prelude::*;

fn save_button() -> impl View {
    button("Save").action(|manager: SnackbarManager| {
        manager.show(Snackbar::new("File saved"));
    })
}
```

`SnackbarManager` is a framework state type, so it extracts bare, without a `State(...)` wrapper. `Snackbar::new` takes anything convertible to `Str`. The message and the action label are shown as written; unlike a `&'static str` passed to `text`, they are not looked up in the translation catalog.

### Builder options

| Method | Effect |
|---|---|
| `.action(label, handler)` | Sets the action button; a second call replaces the first. The handler takes the same arguments as a button action. |
| `.duration(Duration)` | Auto-dismiss delay. The default is 3 seconds; `Duration::ZERO` disables auto-dismiss. |
| `.position(SnackbarPosition)` | One of `BottomCenter` (default), `TopCenter`, `BottomLeading`, `BottomTrailing`. |
| `.closeable()` | Adds a trailing close button. Pair it with `Duration::ZERO` for a snackbar that stays until closed. |
| `.icon(view)` | A leading icon. Any cloneable view works; use an icon-set crate for portable apps (see [the icons chapter](12-icons.md)). |
| `.state(&value)` | Injects a cloneable value that the `action` and `on_dismiss` handlers can extract with `State<T>`. |
| `.on_dismiss(handler)` | Called once per presentation with a `SnackbarDismissReason`. |

```rust
use core::time::Duration;

use waterui::prelude::*;

fn sticky_notice() -> impl View {
    button("Connect").action(|manager: SnackbarManager| {
        manager.show(
            Snackbar::new("Connection restored")
                .position(SnackbarPosition::TopCenter)
                .duration(Duration::ZERO)
                .closeable(),
        );
    })
}
```

Snackbars at the same position stack and reflow, up to three at a time; when a fourth arrives, the oldest one there is dismissed to make room. Different positions are independent stacks, so a top snackbar never evicts a bottom one. `manager.dismiss()` removes the most recently shown snackbar.

### Undo with injected state

An undo action usually needs to touch app state. Inject it with `.state(...)` and extract it in the handler, the same way a button action extracts environment values. Each `.state(...)` call injects one value, and values of different types are extracted independently. `on_dismiss` receives a `SnackbarDismissReason` first, then any injected state, so you can finalize work when the user did not press the action:

```rust
use waterui::prelude::*;

fn delete_all_with_undo(items: &Binding<Vec<String>>) -> impl View {
    let items = items.clone();
    button("Delete all").action(move |manager: SnackbarManager| {
        // Empty the list now; the snackbar keeps the removed items for undo.
        let removed = items.take();
        manager.show(
            Snackbar::new("Items deleted")
                .state(&items)
                .state(&removed)
                .action(
                    "Undo",
                    |State(items): State<Binding<Vec<String>>>,
                     State(removed): State<Vec<String>>| {
                        items.set(removed);
                    },
                )
                .on_dismiss(
                    |reason: SnackbarDismissReason, State(removed): State<Vec<String>>| {
                        if reason != SnackbarDismissReason::Action {
                            waterui::log::debug!("{} items deleted for good", removed.len());
                        }
                    },
                ),
        );
    })
}
```

`SnackbarDismissReason` has five variants: `Action`, `Timeout`, `Closed`, `Replaced`, and `Programmatic`. The handler runs exactly once per presentation, after the exit animation finishes.

### Theming

The overlay reads a `SnackbarTheme` from the environment and falls back to `SnackbarTheme::default()`. The default is the compact Apple-style banner on Apple platforms and the Material 3 bar everywhere else; `SnackbarTheme::material()` and `SnackbarTheme::apple()` name each one explicitly. Its public fields (container color, corner radius, animations, padding, and so on) are plain data.

The window draws its snackbars beside your root view rather than inside it, so a theme attached to your root view with a modifier never reaches them. Install it in the application's environment instead (see [the environment chapter](../02-core/03-environment.md)):

```rust
use waterui::app::App;
use waterui::prelude::*;

fn root() -> impl View {
    "Hello, WaterUI!"
}

pub fn app(mut env: Environment) -> App {
    env.insert(SnackbarTheme {
        corner_radius: 12.0,
        ..SnackbarTheme::material()
    });
    App::new(root, env)
}
```

## Anchored overlays

A popover, tooltip, or dropdown belongs to a specific view. `.anchored_overlay(...)` attaches an `AnchoredOverlay` to the view it modifies. While the overlay's binding is `true`, the backend presents the content next to that view, above everything else in the window. The backend, not your layout, does the placement, because only it knows where the anchor sits in the window and how big the window is.

```rust
use waterui::metadata::anchored_overlay::{AnchorEdge, AnchoredOverlay, Clamp};
use waterui::prelude::*;

fn options_button() -> impl View {
    let open = binding(false);

    button("Options")
        .action({
            let open = open.clone();
            move || open.set(true)
        })
        .anchored_overlay(
            AnchoredOverlay::new(&open, move || text("Rename, duplicate, archive"))
                .edge(AnchorEdge::Bottom)
                .gap(6.0)
                .clamp(Clamp::Window { margin: 8.0 }),
        )
}
```

The second argument to `AnchoredOverlay::new` is a builder closure, not a view. Every presentation calls it and gets a fresh view. State shared between presentations therefore lives in bindings you own and the closure captures; nothing inside the overlay survives a dismissal.

### Placement

`AnchoredOverlay::new` starts with the overlay below the anchor, centered, with no gap, flipping when there is no room, clamped inside the window, and dismissed by outside interaction. Each of these has a builder method:

| Method | Values | Meaning |
|---|---|---|
| `.edge(...)` | `AnchorEdge::Top`, `Bottom`, `Leading`, `Trailing` | The anchor edge the overlay is placed against. `Leading` and `Trailing` follow the layout direction. |
| `.alignment(...)` | `EdgeAlignment::Start`, `Center`, `End` | How the overlay lines up along that edge. Along `Top` and `Bottom`, start is the leading side; along `Leading` and `Trailing`, start is the top. |
| `.gap(f32)` | points | Distance between anchor and overlay. |
| `.flip(bool)` | `true`, `false` | Move to the opposite edge when the preferred one has no room. |
| `.clamp(...)` | `Clamp::Off`, `Clamp::Window { margin }` | Keep the overlay inside the window, at least `margin` points from its edges. |
| `.dismissal(...)` | `Dismissal::OutsideInteraction`, `Dismissal::Manual` | What besides the binding closes it. |

With `OutsideInteraction`, a pointer or touch down outside the overlay writes `false` to the binding, and the same event still reaches its target. With `Manual`, only your binding closes it, which suits an overlay with its own close button.

### A tooltip that knows where it landed

After a flip, the overlay may sit on the opposite side of the anchor from the one you asked for. The backend reports the edge it actually used into a binding you pass to `.placed_edge(...)`, so the content can point at the anchor from the right side:

```rust
use waterui::metadata::anchored_overlay::{AnchorEdge, AnchoredOverlay, Clamp};
use waterui::prelude::*;

fn help_icon() -> impl View {
    let shown = binding(false);
    let placed = binding(AnchorEdge::Top);

    text!("?")
        .on_tap({
            let shown = shown.clone();
            move || shown.set(true)
        })
        .anchored_overlay(
            AnchoredOverlay::new(&shown, {
                let placed = placed.clone();
                move || {
                    // Point the arrow back at the anchor from whichever side it landed on.
                    text(
                        placed
                            .map(|edge| match edge {
                                AnchorEdge::Bottom => "↑ Opens the help page",
                                _ => "Opens the help page ↓",
                            })
                            .computed(),
                    )
                }
            })
            .edge(AnchorEdge::Top)
            .gap(4.0)
            .clamp(Clamp::Window { margin: 2.0 })
            .placed_edge(&placed),
        )
}
```

The backend writes `placed` on every placement. Until the first one, it keeps the value your binding started with, so start it at the edge you asked for.

## Full-screen overlays

`FullScreenOverlayManager` is the lowest-level of the four. Each window holds one slot above its content: `show(view)` puts any view there, replacing whatever was shown, and `hide()` clears it. Snackbars still draw above it. It does no dimming, trapping of input, or dismissal for you, so reach for `Dialog` when you want a modal question and use this when you are building your own, such as a progress screen.

Like the snackbar manager, every `Window` installs one, and a handler extracts it by type. The view you show is inside the window too, so a button in it can extract the same manager to hide itself:

```rust
use waterui::prelude::*;

fn import_button() -> impl View {
    button("Import").action(|overlay: FullScreenOverlayManager| {
        overlay.show(vstack((
            text("Importing..."),
            button("Cancel").action(|overlay: FullScreenOverlayManager| overlay.hide()),
        )));
    })
}
```

## Choosing between them

- The user must decide before continuing: `Dialog`.
- The user should be told something happened and can ignore it: `Snackbar`.
- Extra content belongs to one control: `AnchoredOverlay`.
- Anything else that must cover the window: `FullScreenOverlayManager`.

Menus are a related but separate mechanism with their own API; they are covered in [the menus chapter](09-menus.md). For the surrounding styling of the surfaces these overlays draw, continue with [the styling chapter](11-styling.md).

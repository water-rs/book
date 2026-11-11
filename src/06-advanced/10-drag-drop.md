# Drag and drop

> **In this chapter, you will:**
> - Make a view a drag source with `.draggable(..)` and a drop target with `.drop_destination(..)`
> - Mark your own types `Transferable` and know which ones leave the process
> - Show hover feedback with `.drop_hover(..)`, `.on_enter(..)` and `.on_exit(..)`
> - Receive files dragged in from the operating system as `Files`
> - Read a `DragPayload` directly with `downcast_ref` and `platform_representation`

Moving data between views is a different job from recognizing a pointer
gesture. The [`DragGesture`](02-gestures.md) only reports how far a pointer
traveled; it moves nothing. Drag and drop carries one typed value from a
source view to a destination view. On macOS and iOS, a value with a pasteboard
form also travels to and from other applications.

The model has two modifiers and one marker trait:

- `.draggable(payload)` turns a view into a drag source.
- `.drop_destination(handler)` turns a view into a drop target.
- `Transferable` marks the types a drag can carry.

## A first drag

The smallest useful pair moves text. `Str` is transferable, so the source
carries a `Str` and the destination takes one as its handler's first argument.

```rust
use waterui::Str;
use waterui::prelude::*;

fn text_transfer() -> impl View {
    let received: Binding<Str> = binding("Nothing yet");

    vstack((
        text("Drag me")
            .padding()
            .draggable(Str::from("你好 / こんにちは / 안녕하세요")),
        text!("{received}")
            .padding()
            .drop_destination(|text: Str, State(received): State<Binding<Str>>| {
                received.set(text);
            })
            .state(&received),
    ))
}
```

The backend owns the gesture that starts a drag, such as click and drag on
macOS or long-press and drag on iOS, so you add no recognizer for it.

## The destination's type is its filter

A destination accepts exactly the drags whose value has the type of its
handler's first argument. A drag carrying any other type never enters it: its
enter and exit callbacks do not fire and its handler is not called.
`|text: Str| ..` accepts text only, `|files: Files| ..` accepts files only, and
`|fruit: Fruit| ..` accepts your own `Fruit` and nothing else.

Because the handler is an ordinary [handler](02-gestures.md), the arguments
after the dropped value are extractors. State reaches them through
`State<T>` and `.state(&binding)`, the same way it reaches a button action.

## Transferable types

```rust,ignore
pub trait Transferable: Clone + 'static {}
```

The framework implements it for three types, and each has a form the platform
pasteboard understands:

| Type    | Pasteboard form                  |
| ------- | -------------------------------- |
| `Str`   | plain text                       |
| `Url`   | a URL                            |
| `Files` | file URLs, one item per file     |

On macOS and iOS these travel between applications. Your own types do not: a
type you mark `Transferable` travels inside the process only. It never reaches
the pasteboard, and a drag from another application never produces it.

Marking a type takes an empty implementation. A plain value passed to
`.draggable(..)` is a constant signal, so the type also needs
`impl_constant!`, which comes from `waterui::reactive`.

```rust
use waterui::drag_drop::Transferable;
use waterui::prelude::*;
use waterui::reactive::impl_constant;

#[derive(Debug, Clone, PartialEq)]
struct Fruit(&'static str);

impl Transferable for Fruit {}
impl_constant!(Fruit);

fn fruit_card(name: &'static str) -> impl View {
    text(name).padding().draggable(Fruit(name))
}
```

### Carrying a value read at drag time

`.draggable(..)` accepts a plain transferable value or any signal of one, a
`Binding` or a `Computed`. A signal is read when the drag begins, so the
payload reflects the state at that moment instead of the state when the view
was built.

```rust
use waterui::Str;
use waterui::prelude::*;

fn draft_chip() -> impl View {
    let draft: Binding<Str> = binding("Draft");
    text!("{draft}").padding().draggable(draft.clone())
}
```

## Hover feedback

A destination draws no hover state of its own; it only reports when an
accepted drag enters and leaves it. To show feedback, chain
`.drop_hover(&binding)` directly on the `.drop_destination(..)` call. It sets
the binding to `true` when an accepted drag enters the view and `false` when
it leaves.

```rust
use waterui::drag_drop::Transferable;
use waterui::prelude::*;
use waterui::reactive::impl_constant;

#[derive(Debug, Clone, PartialEq)]
struct Fruit(&'static str);

impl Transferable for Fruit {}
impl_constant!(Fruit);

fn basket() -> impl View {
    let hovering: Binding<bool> = binding(false);
    let collected: Binding<Vec<String>> = binding(Vec::new());

    vstack((
        text!("{title}", title = hovering.select("Release to drop", "Basket")),
        text!("{count} items", count = collected.map(|items| items.len())),
    ))
    .padding()
    .drop_destination(
        |fruit: Fruit, State(collected): State<Binding<Vec<String>>>| {
            collected.get_mut().push(fruit.0.to_string());
        },
    )
    .drop_hover(&hovering)
    .state(&collected)
}
```

`.drop_hover(..)`, `.on_enter(..)` and `.on_exit(..)` belong to
`DropDestinationExt`, which the prelude exports. They exist on the value
`.drop_destination(..)` returns, so they must chain directly on it; putting
another modifier in between is a compile error. In the sample, `.state(..)`
comes after them for that reason.

`.on_enter(f)` and `.on_exit(f)` take plain `FnMut()` closures and add a
handler rather than replacing one, so you can combine them with
`.drop_hover(..)`.

```rust
use waterui::Str;
use waterui::prelude::*;

fn logged_target() -> impl View {
    text("Drop text here")
        .padding()
        .drop_destination(|text: Str| {
            waterui::log::info!(dropped = %text, "text dropped");
        })
        .on_enter(|| waterui::log::debug!("drag entered"))
        .on_exit(|| waterui::log::debug!("drag left"))
}
```

Diagnostics go through `tracing`, which the facade re-exports as
`waterui::log`.

## Files dragged in from the OS

A file dragged in from Finder or another application arrives as `Files`, a
list of URLs that name local files. `Files::urls()` borrows them as `&[Url]`,
and `into_urls()` consumes the list.

```rust
use waterui::drag_drop::Files;
use waterui::prelude::*;

fn file_drop_zone() -> impl View {
    let names: Binding<Vec<String>> = binding(Vec::new());
    let hovering: Binding<bool> = binding(false);

    vstack((
        text!("{title}", title = hovering.select("Release to add", "Drop files here")),
        text!("{count} files", count = names.map(|names| names.len())),
    ))
    .padding()
    .drop_destination(
        |files: Files, State(names): State<Binding<Vec<String>>>| {
            for url in files.urls() {
                names.get_mut().push(url.filename().unwrap_or("untitled").to_string());
            }
        },
    )
    .drop_hover(&hovering)
    .state(&names)
}
```

`Url::filename()` returns the last component of the URL's path, or `None`
when the path ends in `/`.

## Reading a payload directly

Most code never touches the erased payload, because `.drop_destination(..)`
unwraps it for you. Backend authors and tools that build drags by hand work
with `DragPayload`.

```rust
use waterui::Str;
use waterui::drag_drop::{DragPayload, PlatformRepresentation, TransferKind};

fn describe(payload: &DragPayload) -> &'static str {
    match payload.platform_representation() {
        PlatformRepresentation::Text(_) => "text",
        PlatformRepresentation::Url(_) => "url",
        PlatformRepresentation::Files(_) => "files",
        PlatformRepresentation::InProcess => "in-process value",
    }
}

fn demo() {
    let payload = DragPayload::new(Str::from("hello"));
    assert_eq!(payload.kind(), TransferKind::Text);
    assert!(payload.downcast_ref::<Str>().is_some());
    assert_eq!(describe(&payload), "text");
}
```

- `DragPayload::new(value)` wraps any `Transferable`.
- `kind()` returns a `TransferKind`: `Text`, `Url`, `Files`, or
  `InProcess(TypeId)`. `TransferKind::of::<T>()` computes it for a type, and
  `is_platform()` says whether the kind can reach the platform pasteboard.
- `downcast_ref::<T>()` returns the value if it has type `T`.
- `platform_representation()` returns a `PlatformRepresentation` that borrows
  the value in the form the pasteboard expects, or `InProcess` when there is
  none.

`DropDestination::new(handler)` is the constructor behind
`.drop_destination(..)`, and `accepted_kind()` reports the `TransferKind` its
handler's type selects. `.draggable(..)` likewise wraps a `Draggable`. Reach for
these types only when you build metadata by hand; the modifiers are the API for
application code.

## Platform notes

- **macOS and iOS.** The Apple backend maps drag and drop onto
  `NSDraggingSource` and `NSDraggingDestination` on macOS and onto
  `UIDragInteraction` and `UIDropInteraction` on iOS. `Str`, `Url` and `Files`
  go out to other applications and come in from them; your own types stay in
  the app.
- **Hydrolysis** (Android, Windows, Linux). Drags travel between views inside
  the app and never reach another application. In a desktop window, files
  dropped from the operating system arrive as one `Files` value at the drop.

## Where to go next

The `examples/drag_drop` project in the WaterUI repository is a complete
fruit-basket demo with a spring animation on drop. For recognizing pointer
movement without moving data, read [Gestures, hover, and haptics](02-gestures.md);
for keyboard-driven interaction, read
[Keyboard input and focus](09-keyboard-focus.md).

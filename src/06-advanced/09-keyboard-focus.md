# Keyboard input and focus

> **In this chapter, you will:**
> - Handle key presses with `.on_key_press` and decide whether a key is consumed with `KeyHandling`
> - Read a `KeyPress`: the layout-aware `Key`, the physical `Code`, modifiers, and auto-repeat
> - Drive and observe which control has focus through a single `Binding<Option<T>>`
> - Keep a bottom panel above the software keyboard, and let content flow under it on purpose
> - Tell the keyboard what a text field holds with `KeyboardType` and `ContentType`

Pointer input has gestures; the keyboard has two separate mechanisms that are
easy to confuse. *Focus* decides which control receives typed characters.
*Key handlers* catch the keys that control did not use. A text field keeps
focus while an overlay still closes on Escape, because the field ignores
Escape and the key bubbles outward to the overlay's handler.

This chapter covers both, plus the keyboard's effect on layout and the hints a
text field gives the system keyboard. Global shortcuts that appear in menus
are a different mechanism and live in [the menus chapter](../03-ui/09-menus.md).

## Key handlers

`ViewExt::on_key_press` attaches an `OnKeyPress` handler to a view. The
handler returns a `KeyHandling`:

- `KeyHandling::Handled` consumes the key; it stops bubbling.
- `KeyHandling::Ignored` leaves it for the next ancestor.

```rust
use waterui::key::{Key, KeyHandling, KeyPress, NamedKey};
use waterui::prelude::*;

fn search_box() -> impl View {
    let open = binding::<bool>(true);
    let query = binding::<Str>(Str::default());

    field("Search", &query)
        .prompt("Search / 搜索 / 検索 / 검색")
        .on_key_press(
            |Use(press): Use<KeyPress>, State(open): State<Binding<bool>>| {
                if press.key() == &Key::Named(NamedKey::Escape) {
                    open.set(false);
                    KeyHandling::Handled
                } else {
                    KeyHandling::Ignored
                }
            },
        )
        .state(&open)
}
```

The handler is an ordinary [handler](../02-core/04-modifiers.md#stateful-event-handlers): its
parameters are extractors resolved from the environment. `Use<KeyPress>` reads
the key; `State<T>` reads a value injected with `.state(&value)` on the same
view, so the closure captures nothing and the data flow stays visible at the
call site. The same pattern appears in [the gestures chapter](02-gestures.md).

The handler's return type is `KeyHandling`, so every path must answer.
`KeyHandling` is also `#[must_use]`: calling a helper that returns it, such
as `step` in the next section, and dropping the answer is a compiler warning
rather than a silent bug.

### How a key travels

The platform delivers each key to the focused view first. A text field uses
text-editing keys, Left and Right to move the caret, and Return when the field
has a line limit and an `.on_submit` action. Escape, and in a single-line
field Up, Down and a Return with no `on_submit`, are not used, so they
bubble: to the field's ancestors, nearest first, stopping at
the first handler that returns `Handled`. A handler near the root therefore
sees every key that nothing below it claimed.

If a handler's parameters cannot be extracted, it does not run. That is a
contract violation: a debug build panics naming the missing value, and a
release build logs it and treats the key as `Ignored`, so it keeps bubbling.

Return `Ignored` for every key you do not care about. A handler that returns
`Handled` for everything swallows keys that ancestors, and the platform, would
have used.

## Reading a key press

`KeyPress` carries four facts, each behind a getter:

| Getter | Type | Meaning |
| --- | --- | --- |
| `key()` | `&Key` | What the key means under the current layout and modifiers |
| `code()` | `Code` | Where the key sits on the keyboard, independent of layout |
| `modifiers()` | `Modifiers` | The modifiers held with the key |
| `repeat()` | `bool` | Whether this press is an auto-repeat of a held key |

`Key`, `Code`, `Modifiers` and `NamedKey` are the W3C UI Events types from the
`keyboard_types` crate, re-exported from `waterui::key`. Choose between `Key`
and `Code` by what you are modeling.

Use `Key` for meaning. `Key::Named(NamedKey::Escape)` is Escape on every
layout; `Key::Character("k".into())` is whatever produces `k` on the user's
layout.

Use `Code` for position. A game that binds movement to the keys at W, A, S and
D should match `Code::KeyW`, not the character `w`: on a French layout the same
physical key produces `z`, and the character would put the control in the
wrong place.

```rust
use waterui::key::{Code, Key, KeyHandling, KeyPress, Modifiers, NamedKey};
use waterui::prelude::*;

fn step(selected: &Binding<i32>, delta: i32) -> KeyHandling {
    selected.set(selected.snapshot().saturating_add(delta));
    KeyHandling::Handled
}

fn on_key(
    Use(press): Use<KeyPress>,
    State(selected): State<Binding<i32>>,
) -> KeyHandling {
    // Held keys repeat; a repeat should keep stepping.
    match (press.key(), press.code()) {
        (Key::Named(NamedKey::ArrowDown), _) | (_, Code::KeyS) => step(&selected, 1),
        (Key::Named(NamedKey::ArrowUp), _) | (_, Code::KeyW) => step(&selected, -1),
        (Key::Character(c), _)
            if c.as_str() == "0" && press.modifiers().contains(Modifiers::META) =>
        {
            selected.set(0);
            KeyHandling::Handled
        }
        _ => KeyHandling::Ignored,
    }
}

fn picker_panel() -> impl View {
    let selected = binding::<i32>(0);

    text!("Row {selected}")
        .padding()
        .on_key_press(on_key)
        .state(&selected)
}
```

A named function works as a handler for the same reason a closure does: the
extractors are in its signature. Check `press.repeat()` when a key should fire
once per physical press, such as toggling a panel, rather than once per
auto-repeat tick.

Modifiers are a bitset. Test one with `modifiers().contains(Modifiers::SHIFT)`;
the common ones besides `SHIFT` are `CONTROL`, `ALT` and `META`. On Apple
platforms Command arrives as `META` and Option as `ALT`.

### Keys versus shortcuts

An `on_key_press` handler only runs for keys that reach the view through the
focus chain. A shortcut on a command in the application menu bar is
registered with the menu and works whichever view has focus. Use shortcuts for commands the user expects in a
menu bar, and `on_key_press` for keys that mean something only inside one
panel: Escape to close it, arrows to move within it.

## Focus

Focus is state you own. A `Binding<Option<T>>` holds which field is focused,
where `None` means none and `T` identifies a field, usually a small enum.
`ViewExt::focused(&binding, value)` ties one view to one value of that
binding:

```rust
use waterui::prelude::*;

#[derive(Clone, PartialEq, Eq)]
enum Field {
    Username,
    Password,
}

fn sign_in() -> impl View {
    let focus = binding(None::<Field>);
    let username = binding::<Str>(Str::default());
    let password = binding::<Str>(Str::default());

    vstack((
        field("Username", &username)
            .on_submit({
                let focus = focus.clone();
                move || focus.set(Some(Field::Password))
            })
            .focused(&focus, Field::Username),
        field("Password", &password).focused(&focus, Field::Password),
        button("Start with the username").action({
            let focus = focus.clone();
            move || focus.set(Some(Field::Username))
        }),
    ))
}
```

The two-way sync is what makes this work. When you write
`Some(Field::Password)`, the view tied to `Field::Password` becomes the focused
control; the one tied to `Field::Username` stops being focused, because both
read the same binding. When the user taps or tabs into a field, the binding is
written back, so your code can read it, for example to show a hint for the
active field. Writing `None` clears focus; clearing a field that is not the
current one is ignored, so a stale write cannot steal focus from the field the
user is in.

Under the hood, `focused` wraps the view in `Metadata<Focused>`, where
`Focused` (in `waterui::component::focus`) holds a `Binding<bool>` derived
from your binding with `Focused::new(&binding, value)`. Reach for that type only
when you build your own wrapper. The Apple backend maps the boolean to first
responder requests on the wrapped control's single focus anchor, and a request
made before the view is in a window is applied once it arrives, so setting the
binding at construction time is safe.

Because focus is a plain binding, it composes with everything else reactive.
A "Next" button, an `on_submit` action and a validation error that jumps to
the offending field are all one `focus.set(...)`. Do not invent a parallel
focus tracker in view state: the binding is the single source.

## The keyboard and layout

On a device with a software keyboard, the screen has two kinds of unsafe area
on each edge: the *container* (system bars, cutouts, the home indicator) and
the *keyboard*. The layout specification defines both regions, and the default
does the right thing:

- A view is laid out clear of every region it does not ignore. A composer
  pinned to the bottom edge therefore rides the top of the keyboard.
- A scroll surface touching the bottom edge extends under the keyboard and
  insets its content, and when a text field inside it gains focus it scrolls
  the minimum distance that brings the field above the keyboard.
- A keyboard that appears or changes height is followed frame by frame with
  the platform's animation. The window and rendering surface are never
  resized to avoid it.

So a messenger screen needs no keyboard code at all:

```rust
use waterui::prelude::*;

fn composer(draft: Binding<Str>) -> impl View {
    hstack((
        field("Message", &draft).prompt("Message / 消息 / メッセージ / 메시지"),
        button("Send").action(move || draft.set(Str::from(""))),
    ))
    .padding_with(12.0)
}

fn messenger() -> impl View {
    let draft = binding::<Str>(Str::default());
    vstack((scroll(text("Conversation")), composer(draft)))
}
```

If a screen should not move for the keyboard, name the region you ignore.
`SafeAreaRegions::KEYBOARD.on(edges)` ignores only the keyboard band, so the
view still respects the container inset:

```rust
use waterui::prelude::*;

fn full_height_editor() -> impl View {
    // Lay out under the keyboard, but stay above the home indicator.
    text!("editor").ignore_safe_area(SafeAreaRegions::KEYBOARD.on(EdgeSet::BOTTOM))
}
```

`SafeAreaRegions` has exactly three values: `CONTAINER`, `KEYBOARD` and `ALL`.
Passing a bare `EdgeSet` to `ignore_safe_area` ignores every region on those
edges, which is almost never what you want for an input screen. The
`examples/keyboard_panel` example in the framework repository is a complete
messenger layout built only on the default behavior. Layout in general is
covered in [the layout chapter](../03-ui/02-layout.md).

## Telling the system what a field holds

A text field can describe its content to the platform in two independent ways,
both set on `TextField`. Import the types from `waterui::component::text_field`.

`KeyboardType` picks the on-screen keyboard layout: `Text` (the default),
`Email`, `URL`, `Number` and `PhoneNumber`. Platforms without a software
keyboard ignore it.

`ContentType` says what the content *means*, for autofill and password
services: `Username`, `Password`, `NewPassword`, `EmailAddress`,
`PhoneNumber`, `OneTimeCode`, `PersonName`, `PostalAddress`, `PostalCode` and
`CreditCardNumber`. It is not a layout. On Apple platforms it is what lets the
system offer a received SMS code above the keyboard; on Android the autofill
service receives the matching hint. A field with no content type is not
offered to autofill. The two combine:

```rust
use waterui::component::text_field::{ContentType, KeyboardType};
use waterui::prelude::*;

fn verification_code() -> impl View {
    let code = binding::<Str>(Str::default());

    field("Code", &code)
        .keyboard(KeyboardType::Number)
        .content_type(ContentType::OneTimeCode)
        .on_submit(|| {
            waterui::log::debug!("verification code submitted");
        })
}
```

Return handling follows the line limit. A field is single-line by default
(`line_limit` is 1): Return runs `on_submit`, and without one it bubbles to
your key handlers. `line_limit(NonZeroUsize)` caps a multi-line field; Return
there still runs `on_submit` when the field has one. Call
`disable_line_limit()` and Return inserts a line break instead and never
submits.

### Input methods

Composition for Chinese, Japanese and Korean input, with its marked text and
candidate window, is handled by the platform text system or the backend's text
editing layer. The public API has no input-method surface of its own: your
code sees the field's `Binding<Str>` and the `KeyPress` events for keys the
field did not consume. Keep that in mind in two places. Do not treat each
`KeyPress` as a typed character, because composed text arrives through the
binding, not through key handlers. And test text fields with CJK strings, not
only ASCII, since line breaking, selection and caret placement differ.

Beyond these types, the public API specifies no per-backend input-method or
autofill behavior.

## Where to go next

- [Gestures](02-gestures.md) covers the pointer side of input.
- [Accessibility](05-accessibility.md) explains how focus and labels become the
  accessibility tree.
- [Menus](../03-ui/09-menus.md) covers commands and their shortcuts.

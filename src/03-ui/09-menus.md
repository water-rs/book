# Menus, commands, and shortcuts

> **In this chapter, you will:**
>
> - Build a popup `Menu` from commands, dividers, and nested menus.
> - Create a `Command` with state, a disabled or checked condition, a subtitle, a role, and a keyboard shortcut.
> - Install an application menu bar with `App::menu_bar`, including the standard Quit and Close Window items.
> - Attach a context menu to any view, with an optional preview and an interactive accessory.

A menu in WaterUI describes what the user can do, not how a platform draws it. You write the commands once. The Apple backend realizes them as `NSMenu` and `UIMenu`. Hydrolysis draws popup and context menus itself, and hands the application menu bar to the platform's own menu bar where one exists (the main menu on macOS, a Win32 menu on Windows). The same `Command` also works in popup menus, the menu bar, context menus, and a text field's selection menu, so a shortcut or a disabled condition is declared in one place.

## A popup menu

`Menu::new(label, items)` is a view. It draws a button; activating it opens the items. The items are anything implementing `MenuView`: a tuple, array, `Vec`, or `Option` of `Command`, `Button`, `Divider`, or a nested `Menu`.

```rust
use waterui::prelude::*;

fn sort_menu() -> impl View {
    let order: Binding<String> = binding("Name");

    Menu::new(
        "Sort by",
        (
            "Name"
                .action(|State(order): State<Binding<String>>| order.set("Name".to_string()))
                .state(&order),
            "Date"
                .action(|State(order): State<Binding<String>>| order.set("Date".to_string()))
                .state(&order),
            Divider,
            Menu::new(
                "More",
                "Size"
                    .action(|State(order): State<Binding<String>>| order.set("Size".to_string()))
                    .state(&order),
            ),
        ),
    )
}
```

Two things in that sample are worth knowing.

- `"Name".action(..)` comes from `CommandExt`, which is implemented for everything that is `IntoLabel`. The receiver is the label and the result is a `Command`, not a `Button`. `"Name".command()` returns a `CommandBuilder` when you prefer to spell it out, and `Command::builder(label).action(..)` is the same call without the extension trait.
- A command's action is a handler, like a button's. It extracts what it needs from the environment, and `.state(&value)` on the command is what installs `State<T>`. Attach `.state(..)` to each command, not to the menu.

The trigger button is drawn like a `Button`: `Menu::style(ButtonStyle)` and `Menu::size(ControlSize)` change it. See [Buttons and controls](03-controls.md) for those types.

Because `Button` converts into `Command` and `MenuItem`, a button you already have can be menu content as is. The conversion keeps the label and the action and drops the button's style and size. A button that has had `.state(..)` applied is no longer a `Button` (that `.state` is the `ViewExt` method, which wraps the view), so build that row as a `Command` instead. `MenuItem` is the enum behind the items (`Command`, `Divider`, `Menu`, `Quit`, `CloseWindow`); you only name it when you build a `Vec<MenuItem>` yourself or declare `Quit` and `CloseWindow`.

## Command options

A `Command` carries more than a label and an action. Each option is a builder method:

| Method | Effect |
| --- | --- |
| `.disabled(signal)` | Grays the command out while the `bool` signal is `true`. |
| `.selected(signal)` | Draws a check mark while the `bool` signal is `true`. |
| `.subtitle(string)` | Adds a secondary line under the label; takes `impl Into<Str>`. |
| `.destructive()` | Same as `.role(CommandRole::Destructive)`; drawn in the platform's destructive style (red text on iOS, macOS and Android, the theme's error color in drawn menus). |
| `.shortcut(Shortcut)` | Arms a keyboard chord and shows its hint in the row. |
| `.state(&value)` | Makes `State<T>` available to the action. |

`disabled` and `selected` take `impl IntoComputed<bool>`, so pass a `Binding<bool>` or any signal and the menu updates as the value changes. A command's own `disabled` signal is OR-combined with any `.disabled(..)` scope around the view that declares it, as a control's is:

```rust
use waterui::prelude::*;

fn view_menu() -> impl View {
    let show_hidden: Binding<bool> = binding(false);
    let read_only: Binding<bool> = binding(true);

    Menu::new(
        "View",
        (
            "Show hidden files"
                .action(|State(flag): State<Binding<bool>>| flag.toggle())
                .state(&show_hidden)
                .selected(show_hidden.clone()),
            "Rename"
                .action(|| {})
                .shortcut(Shortcut::new(NamedKey::F2))
                .disabled(read_only.clone()),
            Divider,
            "Delete"
                .command()
                .action(|| {})
                .destructive()
                .subtitle("Cannot be undone")
                .shortcut(Shortcut::new(NamedKey::Delete).command()),
        ),
    )
}
```

## Shortcuts

`Shortcut::new(key)` takes a `char` or a `NamedKey`, and the modifier methods `.command()`, `.shift()`, `.option()` and `.control()` add to it. Letters match regardless of case, so Shift is always a separate modifier:

```rust
use waterui::prelude::*;

let save = Shortcut::new('s').command();
let save_as = Shortcut::new('s').command().shift();
let delete = Shortcut::new(NamedKey::Delete).command();

assert!(save.modifiers().command());
assert!(save_as.modifiers().shift());
assert_eq!(delete.key(), &ShortcutKey::Named(NamedKey::Delete));
assert_eq!("F5".parse::<ShortcutKey>(), Ok(ShortcutKey::Named(NamedKey::F5)));
assert!("Shift".parse::<ShortcutKey>().is_err());
```

`ShortcutKey` is `Character(char)` or `Named(NamedKey)`. `NamedKey` is the W3C `KeyboardEvent.key` vocabulary, the same type keyboard events report (see [Keyboard input and focus](../06-advanced/09-keyboard-focus.md)). `ShortcutModifiers` is what `Shortcut::modifiers()` returns; its `command()`, `shift()`, `option()` and `control()` methods report each flag.

The command modifier is the platform's menu accelerator: Command on Apple platforms and Control elsewhere. A `Shortcut` formats itself for the platform's menu hint through `Display`, so `Shortcut::new('p').command().shift()` shows as `⇧⌘P` on Apple platforms and `Ctrl+Shift+P` elsewhere.

Keys that could never fire are refused early. `ShortcutKey::from`, and so `Shortcut::new`, panics on a control character (use the named key it stands for, such as `NamedKey::Tab`) and on `NamedKey::Unidentified`, `NamedKey::Dead`, and the modifier and lock keys. `str::parse::<ShortcutKey>()` accepts a single character or a W3C key name and returns `InvalidShortcutKey` for those same inputs, and for any other string, instead of panicking. `InvalidShortcutKey` lives in `waterui::component::menu`; it is not in the prelude. A backend whose platform menu cannot arm a key panics when it builds the item, so pick keys the platforms you target support.

## The application menu bar

`App::menu_bar` takes the top-level menus. The argument is a `MenuBarView`: a single `Menu`, a tuple, array, or `Vec` of menus, an `Option` of any of those, or a `Computed<Vec<Menu>>` for a bar that changes over time. `MenuBarView` lives in `waterui::component::menu` and comes in with the prelude.

```rust
use waterui::app::App;
use waterui::prelude::*;

fn root() -> impl View {
    text("Editor")
}

pub fn app(env: Environment) -> App {
    let word_wrap: Binding<bool> = binding(true);

    App::new(root, env).menu_bar((
        Menu::new(
            "File",
            (
                "New"
                    .action(|| waterui::log::info!("new document"))
                    .shortcut(Shortcut::new('n').command()),
                Divider,
                MenuItem::CloseWindow,
                MenuItem::Quit,
            ),
        ),
        Menu::new(
            "Format",
            "Word wrap"
                .action(|State(flag): State<Binding<bool>>| flag.toggle())
                .state(&word_wrap)
                .selected(word_wrap.clone()),
        ),
    ))
}
```

Menu-bar command actions run under the application environment, not a view's. Besides a command's own `.state(&value)`, `App::state(&value)` installs state for every menu-bar command (and for `App::on_quit_request` and `App::on_terminate`), and handlers extract it the same way view handlers do.

### Quit and Close Window

`MenuItem::Quit` and `MenuItem::CloseWindow` stand for platform commands, so they carry no action. Each is drawn once, with the platform's own label and chord.

- `MenuItem::Quit` requests a cancellable termination, so `App::on_quit_request` still decides, and `App::on_terminate` runs the shutdown work. On macOS the standard application menu already carries Quit, so the item never appears in the menu bar there; it can appear in menus a window mounts.
- `MenuItem::CloseWindow` closes the key window through the same path as its title-bar close button. A window declared without a close button ignores it. When no menu in the bar declares it, the macOS menu bar puts Close in the standard Window menu; once you declare it, it appears only where you placed it.
- Platforms with no application quit, or windows the application cannot close (iOS, Android, the web, and hosts that own their windows), omit these items and arm no chord for them, so a portable menu can declare them unconditionally.

Do not recreate these with a plain command. The macOS menu bar rejects a plain command bound to ⌘Q, ⌘H, or ⌥⌘H: every backend that builds a macOS menu bar panics on one when it builds the bar, at run time, because those chords belong to the standard Quit, Hide, and Hide Others items. A homemade Quit would also skip `on_quit_request` and `on_terminate`.

⌘W, on the other hand, is not reserved. The Close Window chord is decided once for the whole application from the declared menu bar: ⌘W when no command binds it, ⇧⌘W when a command does (a tabbed app's Close Tab) and none binds ⇧⌘W, and no chord when both are bound. On Windows the menu-bar row shows Alt+F4, and Linux has no menu bar, so there the item shows only in mounted and context menus.

## Context menus

`.context_menu(items)` from `ViewExt` attaches a menu to any view. It opens on a long press (a touch or pen hold) or a secondary click (a pointer). It accepts the same `MenuView` content as `Menu::new`, or a `ContextMenu`:

```rust
use waterui::prelude::*;

fn note() -> impl View {
    let status: Binding<String> = binding("Nothing yet");

    vstack((
        text("Long press or right-click me")
            .padding()
            .context_menu((
                "Copy"
                    .action(|State(status): State<Binding<String>>| status.set("Copied".to_string()))
                    .state(&status),
                "Share"
                    .command()
                    .action(|State(status): State<Binding<String>>| status.set("Shared".to_string()))
                    .state(&status)
                    .subtitle("Sends a link"),
                Divider,
                "Delete"
                    .command()
                    .action(|State(status): State<Binding<String>>| status.set("Deleted".to_string()))
                    .state(&status)
                    .destructive(),
            )),
        text!("{status}"),
    ))
}
```

Pass a `ContextMenu` instead of bare items to add a preview or an accessory:

- `.preview(view)` is shown while the menu is open, in the platform's presentation: the lifted preview of the iOS system context menu, or content inside the popover iOS uses when the menu has an accessory. Without it, the platform presents the source view itself.
- `.accessory(view)` is an interactive view presented alongside the commands. On macOS it is a pointer-interactive row inside the `NSMenu`, so it takes no text entry or other keyboard input and its size is fixed until the menu reopens. On iOS, a menu with an accessory shows the preview, accessory, and commands together in a native popover.

Choosing a menu item closes the menu; acting inside the accessory does not. The accessory's environment carries `DismissContextMenu`, which a handler extracts with `Use<DismissContextMenu>`:

```rust
use waterui::prelude::*;

fn message() -> impl View {
    text("Hello").padding().context_menu(
        ContextMenu::new((
            "Reply".action(|| {}),
            "Delete".command().action(|| {}).destructive(),
        ))
        .preview(text("Hello, in a larger preview").padding())
        .accessory(
            button("Like").action(|Use(dismiss): Use<DismissContextMenu>| dismiss.dismiss()),
        ),
    )
}
```

A text field has its own menu for selected text. `field(..).selection_menu(items)` takes the same `MenuView` content, and its handlers can read the live selection; see [the selection menu section of Buttons and controls](03-controls.md#selection-menu).

## Menus on each platform

Menu rows stay within what each platform's native menu can render: a label, an icon, a check mark, a subtitle, a shortcut hint, and the destructive role. Put anything richer in a context menu's accessory instead of a row. The framework-wide platform tiers are in [Platforms](../01-getting-started/05-platforms.md).

## Where to go next

Menus often sit beside window chrome; [Windows](08-windows.md) covers toolbars and window declarations. For the chords your own views handle, continue to [Keyboard input and focus](../06-advanced/09-keyboard-focus.md), and for dialogs, anchored popovers and snackbars see [Dialogs, popovers, and snackbars](10-overlays.md).

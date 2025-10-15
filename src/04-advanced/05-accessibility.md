# Accessibility

WaterUI assigns sensible accessibility metadata to built-in components. When you build composite
widgets or override defaults, use the helpers in `waterui::accessibility` and the `ViewExt`
shortcuts.

## Labels

`AccessibilityLabel` replaces the phrase announced by screen readers.

```rust,ignore
use waterui::accessibility::AccessibilityLabel;
use waterui::prelude::*;

fn destructive_button() -> impl View {
    button("🗑")
        .a11y_label("Delete current draft")
        .on_tap(|| println!("delete"))
}
```

WaterUI automatically drops icon-only labels into the environment so assistive technologies read the
provided description instead of the literal emoji.

## Roles

`AccessibilityRole` describes the control type. Apply it when you repurpose a component for a new
semantic role.

```rust,ignore
use waterui::accessibility::AccessibilityRole;
use waterui::prelude::*;

fn navigation_chip() -> impl View {
    text!("Inbox")
        .padding(12.0)
        .background(style::Background::color(Color::from_rgb(0.19, 0.23, 0.34)))
        .corner_radius(10.0)
        .a11y_role(AccessibilityRole::Tab)
}
```

Roles help operating systems expose the right keyboard shortcuts, rotor menus, and focus handling.

## Custom error summaries

Complex widgets sometimes need bespoke spoken feedback. Compose views with `AccessibilityLabel` or
attach metadata using `.metadata`.

```rust,ignore
use waterui::accessibility::AccessibilityLabel;
use waterui::component::form::TextField;
use waterui::prelude::*;

fn email_field(error: Binding<Option<String>>) -> impl View {
    vstack((
        TextField::new(binding(String::new()))
            .placeholder("Email address")
            .metadata(AccessibilityLabel::new("Email address input")),
        Dynamic::watch(error.clone(), |error| {
            error.map_or_else(
                || AnyView::new(text!("")),
                |msg| AnyView::new(text!("{}", msg).foreground(Color::from_rgb(0.8, 0.2, 0.2))),
            )
        }),
    ))
}
```

Placeholders often disappear when the field gains focus. Supplying a spoken label keeps the context
available to screen-reader users.

## Platform hooks

Backends query the environment for `AccessibilityLabel` and `AccessibilityRole`. When you need to
override the defaults for an entire subtree, set the metadata at the container level.

```rust,ignore
fn modal() -> impl View {
    vstack((
        text!("Session expired").a11y_role(AccessibilityRole::Header),
        text!("Sign back in to continue."),
        button("Sign in").a11y_role(AccessibilityRole::Button),
    ))
    .metadata(AccessibilityRole::Dialog)
}
```

WaterUI forwards metadata as part of the view configuration, so hooks and renderers can translate it
into platform-specific semantics.

# Accessibility

Every WaterUI control ships with reasonable accessibility metadata, but composite views sometimes
need extra context. The `waterui::accessibility` module exposes helpers for customizing labels,
roles, and states so assistive technologies accurately describe your UI.

## Labels

`AccessibilityLabel` overrides the spoken name of a view:

```rust
use waterui::prelude::*;

pub fn destructive_button() -> impl View {
    button("🗑️").a11y_label("Delete draft")
}
```

The emoji-only button now announces “Delete draft” to VoiceOver/TalkBack users.

## Roles

When building custom widgets, mark their semantic role:

```rust
use waterui::accessibility::AccessibilityRole;

pub fn nav_drawer() -> impl View {
    vstack((/* ... */))
        .a11y_role(AccessibilityRole::Navigation)
}
```

Available roles cover buttons, list items, landmarks, menus, tabs, sliders, and more. Choose the one
that matches the behaviour of your component.

## States

`AccessibilityState` communicates dynamic information (selected, expanded, busy, etc.).

```rust
use waterui::accessibility::AccessibilityState;

let state = AccessibilityState::new()
    .selected(true)
    .expanded(Some(false));

accordion_header(name, state)
```

## Patterns

- **Lives regions** – For toast notifications or progress updates, install an environment hook that
  announces changes via `AccessibilityLabel` or a platform-specific API.
- **Custom controls** – When composing primitive views into a new control, set the label/role on the
  container instead of each child to avoid duplicate announcements.
- **Reduced motion** – Read the platform’s accessibility settings (exposed to backends via the
  environment) and adjust animation hooks accordingly.

Accessibility metadata is cheap: it is just view metadata that backends translate into the native
API. Audit new components by navigating with VoiceOver/TalkBack and apply overrides whenever the
defaults fall short.

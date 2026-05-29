# Accessibility

> **In this chapter, you will:**
> - Learn how WaterUI's built-in accessibility defaults work
> - Override labels, roles, and states for custom widgets
> - Hide decorative elements from screen readers
> - Respect reduced motion preferences
> - Test accessibility with platform tools

Your app looks great and handles errors gracefully. But can *everyone* use it?
A user who relies on VoiceOver, TalkBack, or keyboard navigation deserves the
same experience as someone tapping a touchscreen. The good news: WaterUI
components ship with sensible accessibility defaults. Buttons announce themselves
as buttons, text views expose their content, and interactive controls report
their states. This chapter covers what to do when the defaults are not enough --
when you build custom composite widgets, use icons without text labels, or need
to communicate specific semantic meaning to assistive technologies.

The accessibility types live in `waterui::accessibility` and are attached to
views through `ViewExt` methods.

## Design Philosophy

WaterUI follows two principles:

1. **Defaults first.** Built-in components already carry the right roles,
   labels, and states. You should not need to touch accessibility code for
   standard UIs.
2. **Override when necessary.** Custom widgets, decorative elements, and complex
   layouts sometimes need explicit annotations.

Because WaterUI renders to native platform widgets, accessibility metadata maps
directly to the platform's accessibility APIs (UIAccessibility on Apple,
AccessibilityNodeInfo on Android, ATK/AT-SPI on GTK).

## AccessibilityLabel

An `AccessibilityLabel` overrides the spoken label for a component. Use it when
the visual content does not adequately describe the element's purpose -- the
most common case is an icon-only button.

```rust,ignore
use waterui::prelude::*;

// An icon-only button (label is read by VoiceOver, not painted on screen).
button(trash_icon())
    .action(delete_item)
    .a11y_label("Delete draft")
```

The label should be short, action-oriented, and match what a sighted user would
understand from context. Avoid redundant prefixes like "Button:" -- the
accessibility role already communicates that.

### Creating Labels

`AccessibilityLabel::new` accepts anything that converts to `Str`:

```rust,ignore
use waterui::accessibility::AccessibilityLabel;

let label = AccessibilityLabel::new("Delete draft");
let label = AccessibilityLabel::new(format!("Item {index}"));
```

### Attaching to Views

Use `ViewExt::a11y_label`:

```rust,ignore
use waterui::prelude::*;

logo_image().a11y_label("Company logo")
```

## AccessibilityRole

An `AccessibilityRole` describes the semantic purpose of a component. WaterUI
components set their own roles (a `Button` is `Role::Button`, a `Toggle` is
`Role::Switch`), but custom composites need explicit role assignment.

### Available Roles

The `AccessibilityRole` enum covers a wide range of semantics:

| Category | Roles |
|---|---|
| **Interactive** | `Button`, `Link`, `Checkbox`, `RadioButton`, `Switch`, `Slider` |
| **Content** | `Text`, `Image`, `Header`, `Footer`, `Article` |
| **Structure** | `Navigation`, `Main`, `Search`, `Section`, `Group` |
| **Collections** | `List`, `ListItem`, `Tab`, `TabList`, `TabPanel` |
| **Menus** | `Menu`, `MenuItem`, `MenuBar`, `MenuItemCheckbox`, `MenuItemRadio` |
| **Forms** | `Combobox`, `Option`, `ProgressBar` |

### Attaching Roles

Use `ViewExt::a11y_role`. Here is a custom toggle that would otherwise be
invisible to assistive technology:

```rust,ignore
use waterui::prelude::*;
use waterui::accessibility::AccessibilityRole;

fn custom_toggle(is_on: &Binding<bool>) -> impl View {
    let background = is_on.map(|on| {
        if on { Color::srgb(52, 199, 89) } else { Color::srgb(200, 200, 200) }
    });

    hstack((knob(),))
        .padding()
        .background(background)
        .a11y_role(AccessibilityRole::Switch)
        .a11y_label("Dark mode")
        .state(is_on)
        .on_tap(|State(b): State<Binding<bool>>| b.toggle())
}
```

The role tells VoiceOver/TalkBack to announce this as a switch and provide
the appropriate interaction hints.

## AccessibilityState

`AccessibilityState` communicates nuanced state information to assistive
technologies. You need it when building custom controls whose state goes beyond
what a label and role can express.

```rust,ignore
pub struct AccessibilityState {
    disabled: bool,
    selected: bool,
    checked: Option<bool>,
    expanded: Option<bool>,
    busy: bool,
    hidden: bool,
}
```

| Field | Meaning |
|---|---|
| `disabled` | The control is visible but not interactive |
| `selected` | The control is the current selection in its group |
| `checked` | Checked (`Some(true)`), unchecked (`Some(false)`), or mixed/indeterminate (`None` when the concept applies) |
| `expanded` | Expanded (`Some(true)`) or collapsed (`Some(false)`) for disclosure controls |
| `busy` | The control is loading or processing |
| `hidden` | The control should be invisible to assistive technology |

### When to Use States

Most of the time, built-in components handle state automatically. Use
`AccessibilityState` when you build custom controls:

- A custom accordion needs `expanded`.
- A custom checkbox needs `checked`.
- A skeleton loading placeholder needs `busy`.
- Decorative elements need `hidden`.

## Hiding Decorative Elements

Purely decorative views (background patterns, divider lines, brand marks)
should be hidden from assistive technology so screen readers do not announce
noise. Use `ViewExt::a11y_hidden(true)`, which attaches the
`AccessibilityHidden` metadata:

```rust,ignore
use waterui::prelude::*;

decorative_swirl().a11y_hidden(true)
```

If you also want to drop a subtree's children from the tree (for example, an
icon-and-label composite that you re-described with a single label), use
`ViewExt::a11y_children(AccessibilityChildren::ExcludeDescendants)` instead.

## Custom Control Accessibility

Building a fully accessible custom control requires combining label, role, and
state. Here is a complete example of a custom star rating widget:

```rust,ignore
use waterui::prelude::*;
use waterui::accessibility::AccessibilityRole;
use waterui::reactive::watch;

fn star_rating(rating: &Binding<i32>, max: i32) -> impl View {
    let label = rating.map(move |r| format!("Rating: {r} out of {max}"));

    hstack(
        (0..max).map(|i| {
            let filled = rating.map(move |r| r > i);
            let star_label = format!("{} star", i + 1);

            watch(filled, |is_filled| {
                if is_filled { text("*") } else { text("o") }
            })
            .a11y_label(star_label)
            .a11y_role(AccessibilityRole::Button)
            .state(rating)
            .on_tap(move |State(r): State<Binding<i32>>| r.set(i + 1))
        }).collect::<Vec<_>>()
    )
    .a11y_role(AccessibilityRole::Slider)
    .a11y_label(label)
}
```

The container has `Slider` role and a dynamic label. Each star has `Button`
role with its own label. This gives screen reader users both the overall rating
and individual star controls.

> **Try it yourself:** Build a custom accordion component and use
> `AccessibilityState` with the `expanded` field to announce whether each
> section is open or closed.

## Reduced Motion

Some users are sensitive to animation. WaterUI does not yet ship a built-in
"prefers reduced motion" signal -- the recommended pattern is to define your
own marker type, install it from the platform layer, and gate animation
metadata behind it:

```rust,ignore
use waterui::prelude::*;
use waterui_core::animation::Animation;
use core::time::Duration;

#[derive(Debug, Clone, Copy)]
struct PrefersReducedMotion(bool);

fn animated_entrance(env: &Environment) -> impl View {
    let opacity = Binding::f64(0.0);

    let prefers_reduced = env
        .get::<PrefersReducedMotion>()
        .map_or(false, |p| p.0);

    let target_opacity = if prefers_reduced {
        opacity.clone().computed()
    } else {
        opacity
            .clone()
            .with_animation(Animation::ease_in_out(Duration::from_millis(300)))
            .computed()
    };

    text("Welcome!")
        .opacity(target_opacity)
        .on_appear(move || opacity.set(1.0))
}
```

> **Note:** Respecting reduced motion is a real accessibility requirement
> that affects users with vestibular disorders. Wire your platform's
> reduced-motion API into the environment from your backend integration.

## Accessible Navigation

When using `NavigationView` or `TabView`, WaterUI automatically sets the
correct navigation landmarks. Screen readers announce tab switches and
navigation transitions. You can enhance this by adding descriptive labels to
containers:

```rust,ignore
use waterui::prelude::*;
use waterui::accessibility::AccessibilityRole;

fn sidebar() -> impl View {
    vstack((
        text("Menu").headline(),
        button("Home").action(|| {}),
        button("Settings").action(|| {}),
    ))
    .a11y_role(AccessibilityRole::Navigation)
    .a11y_label("Main navigation")
}
```

## Focus Management

WaterUI's focus system (covered in the Modifiers chapter) works with
accessibility. When a view is focused, the accessibility system announces it.
The `focused()` modifier on `ViewExt` integrates with both the visual focus
ring and the accessibility focus:

```rust,ignore
use waterui::prelude::*;

let focus = Binding::container::<Option<Field>>(None);
let name = Binding::container(Str::from(""));

field("Name", &name)
    .focused(&focus, Field::Name)
```

When `focus` is set to `Some(Field::Name)`, VoiceOver/TalkBack will move focus
to that field.

## Testing Accessibility

WaterUI's preferred automated check is the `waterui-testing` crate, which
drives views through the **Hydrolysis accessibility tree**. Because every
component is expected to expose meaningful accessibility metadata,
`waterui-testing` doubles as both an interaction harness and an
accessibility-correctness check. Treat a missing or wrong tree as a bug to fix
in the component, not a gap to paper over.

For visual smoke checks, render a view with `water preview ... --output preview.png`
and inspect the result. Pair these with platform-native auditors when shipping:

- **iOS**: Accessibility Inspector in Xcode
- **Android**: Accessibility Scanner
- **macOS**: VoiceOver (Cmd+F5)
- **Linux/GTK**: Accerciser (AT-SPI explorer)

> **Tip:** Spend ten minutes navigating your app with VoiceOver or TalkBack
> before shipping. Automated checks cannot replace the experience of actually
> hearing how a screen reader interprets your UI.

## Best Practices

1. **Let defaults work.** Do not add `.a11y_label()` to every view. Built-in
   components already expose their text content.

2. **Label icons and images.** Any visual element without text needs an
   explicit label.

3. **Use semantic roles.** A custom `div`-like container should have
   `Group`, `Navigation`, or `Main` role depending on purpose.

4. **Hide decorative content.** Background images, dividers, and brand marks
   should not be announced.

5. **Test with a screen reader.** Automated checks cannot replace the
   experience of navigating your app with VoiceOver or TalkBack.

6. **Provide dynamic labels.** Use reactive bindings to keep accessibility
   labels in sync with changing content.

## Summary

| API | Purpose |
|---|---|
| `.a11y_label(text)` | Override the spoken label |
| `.a11y_role(role)` | Set the semantic role |
| `AccessibilityLabel::new(text)` | Create a label value |
| `AccessibilityRole::Button` | Interactive control role |
| `AccessibilityRole::Image` | Image role |
| `AccessibilityRole::Text` | Non-interactive text |
| `AccessibilityRole::Navigation` | Navigation landmark |
| `AccessibilityRole::Switch` | Toggle/switch control |
| `AccessibilityRole::Slider` | Range input |
| `AccessibilityState` | Disabled, selected, checked, expanded, busy, hidden |
| `.focused(binding, value)` | Programmatic focus management |

## What's Next

Your app is accessible to users regardless of ability. But what about users
who speak different languages? In the [next chapter](06-i18n.md), you will
learn how WaterUI's internationalization system handles translations, plural
rules, and locale-aware formatting.

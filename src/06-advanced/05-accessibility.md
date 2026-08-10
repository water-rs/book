# Accessibility

> **In this chapter, you will:**
> - Rely on the labels WaterUI forces every control to carry
> - Hide a label visually without removing it from the accessibility tree
> - Override labels, roles, and states for custom widgets
> - Report disabled and hidden states correctly
> - Assert on the accessibility tree in `cargo test`

WaterUI does not treat accessibility as an annotation you add later. Its
control constructors put the accessible name in the signature, so the common
path already produces a labeled control. The work left to you is the part the
type system cannot do: describing custom composites and hiding decoration.

The types live in `waterui::accessibility`; the modifiers are on `ViewExt`.

## Labels are mandatory, visibility is not

`button`, `slider`, `stepper`, `toggle`, and `field` all take an `impl
IntoLabel` as their first argument. Screen readers, voice control, switch
control, and command palettes all read that label — a control without one is an
anonymous widget that assistive technology users cannot reach.

Reach for the ergonomic free functions rather than `Toggle::new(&binding)` and
`TextField::new(&binding)`, which start with an empty label and rely on you
remembering `.label(...)` afterwards.

Hiding the label is a separate, presentational decision. `.hide_label()`
collapses the label's rendered view to zero size while keeping the semantic
text in the accessibility tree:

```rust,ignore
use waterui::prelude::slider::slider;
use waterui::prelude::*;

let progress = Binding::f64(0.5);

// Announced as "Playback position"; nothing is drawn next to the track.
slider("Playback position", &progress).hide_label()
```

`.hide_label()` is shorthand for `.label_style(LabelDisplayMode::Hidden)`. The
other modes — `TitleAndIcon`, `TitleOnly`, `IconOnly`, and the default
`Automatic` — pick between title and icon without ever dropping the semantic
text. Install a `LabelDisplayMode` in the environment to set the default for a
whole subtree.

### Icon-only controls

An icon-only button is a display mode, not a label-less button:

```rust,ignore
use waterui::prelude::*;

// Still announced as "Search".
button(label("Search").icon(search_icon()).icon_only())
    .action(run_search)
```

`Label::icon` takes any view, so an icon-pack crate works here. `SystemIcon`
(via `Label::system_icon`) renders SF Symbols on Apple platforms and is
intentionally unsupported on Android, Linux, and Web — for portable code prefer
`Label::icon` with `waterui-icons-lucide`, `waterui-icons-material-icon`, or
`waterui-icons-fontawesome7`.

### When the visible content is not the spoken text

`Label::new(semantic_text, content)` is the general constructor: it takes
arbitrary visual content plus the text assistive technology should announce.

```rust,ignore
use waterui::prelude::*;

let verified = Label::new(
    "Account, verified",
    hstack((text("Account"), verification_badge())),
);

button(verified).action(open_account)
```

A `Label::new` label owns its own layout, so the semantic-label builders
(`.icon()`, `.system_icon()`, `.leading()`, `.trailing()`, `.spacing()`,
`.font()`) panic on it — compose those inside `content` instead.

## Overriding a label

`ViewExt::a11y_label` replaces the spoken label for any view. Reach for it when
the view is not a control and its visual content does not describe it:

```rust,ignore
use waterui::prelude::*;

logo_image().a11y_label("Acme, home")
```

Keep it short and action-oriented, and leave out prefixes like "Button:" — the
role already communicates that.

The label is reactive. Pass a signal and a label derived from app state stays
current without rebuilding the subtree, the same way accessibility *state* does:

```rust,ignore
use waterui::prelude::*;

fn inbox(unread: &Binding<i32>) -> impl View {
    let label = unread.clone().map(|n| Str::from(format!("{n} unread messages")));
    button("Inbox").action(|| {}).a11y_label(label)
}
```

## Roles

`AccessibilityRole` describes what a view *is*. Built-in controls set their own
role; custom composites need one assigned:

| Category | Roles |
|---|---|
| Interactive | `Button`, `Link`, `Checkbox`, `RadioButton`, `Switch`, `Slider` |
| Content | `Text`, `Image`, `Header`, `Footer`, `Article` |
| Structure | `Navigation`, `Main`, `Search`, `Section`, `Group` |
| Collections | `List`, `ListItem`, `Tab`, `TabList`, `TabPanel` |
| Menus | `Menu`, `MenuItem`, `MenuBar`, `MenuItemCheckbox`, `MenuItemRadio` |
| Forms | `Combobox`, `Option`, `ProgressBar` |

Navigation containers do not attach landmark roles for you. If you build a
sidebar, say so:

```rust,ignore
use waterui::accessibility::AccessibilityRole;
use waterui::prelude::*;

vstack((
    text("Menu").headline(),
    button("Home").action(go_home),
    button("Settings").action(go_settings),
))
.a11y_role(AccessibilityRole::Navigation)
.a11y_label("Main navigation")
```

## States

`AccessibilityState` carries what a label and role cannot express. It is a
const builder, so a state is cheap to construct:

```rust,ignore
use waterui::accessibility::AccessibilityState;

let state = AccessibilityState::new().expanded(Some(true)).busy(false);
```

| Field | Meaning |
|---|---|
| `disabled` | Visible but not interactive |
| `selected` | The current selection within its group |
| `checked` | `Some(true)`, `Some(false)`, or `None` for mixed |
| `expanded` | `Some(true)` / `Some(false)` for disclosure controls |
| `busy` | Loading or processing |
| `hidden` | Not exposed to assistive technology |

Attach a fixed state with `.a11y_state(state)`, or a reactive one with
`.a11y_state_signal(signal)`:

```rust,ignore
use waterui::accessibility::AccessibilityState;
use waterui::prelude::*;

fn disclosure(expanded: &Binding<bool>, content: impl View) -> impl View {
    let state = expanded.map(|open| AccessibilityState::new().expanded(Some(open)));

    content.a11y_state_signal(state)
}
```

### Disabled and hidden come for free

Two `ViewExt` modifiers already write state for you:

- `.disabled(signal)` installs a disabled scope over the subtree. Controls
  render their platform disabled appearance, stop hit-testing, and report the
  disabled state to assistive technology. Nested scopes OR-combine, so an
  enclosing `.disabled(true)` cannot be undone by a child.
- `.visible(signal)` sets `hidden` on the accessibility state as it fades the
  view out, so an invisible view is not announced.

`.disabled(...)` reaches every control, because no control implements disabled
state itself: each one reads the scope in force at its own position. A menu
`Command` carries the state as data instead — a menu is a list of records handed
to the platform's menu API, with no leaf environment to read — and its own flag
is OR-combined with the enclosing scope when the command resolves.

## Hiding decoration

Background patterns, dividers, and brand marks add noise to a screen reader.
`ViewExt::a11y_hidden(true)` drops a view from the tree:

```rust,ignore
use waterui::prelude::*;

decorative_swirl().a11y_hidden(true)
```

When you have re-described a whole composite with one label, drop its children
instead of hiding the container:

```rust,ignore
use waterui::accessibility::AccessibilityChildren;
use waterui::prelude::*;

hstack((star_icon(), text("4.8"), text("(120)")))
    .a11y_label("Rated 4.8 out of 5, 120 reviews")
    .a11y_children(AccessibilityChildren::ExcludeDescendants)
```

## A custom control, end to end

A star rating needs a role and a label on the container and on each star. Note
that the filled/empty glyph comes from a mapped signal fed into `text` — not
from `watch`, which would rebuild the star subtree on every rating change and
discard its state.

```rust,ignore
use waterui::accessibility::{AccessibilityRole, AccessibilityState};
use waterui::prelude::*;

fn star_rating(rating: &Binding<i32>, max: i32) -> impl View {
    hstack(
        (0..max)
            .map(|i| {
                let glyph = rating.map(move |r| if r > i { "★" } else { "☆" }).computed();
                let filled = rating.map(move |r| {
                    AccessibilityState::new().selected(r > i)
                });

                text(glyph)
                    .a11y_label(format!("Rate {} of {max}", i + 1))
                    .a11y_role(AccessibilityRole::Button)
                    .a11y_state_signal(filled)
                    .state(rating)
                    .on_tap(move |State(r): State<Binding<i32>>| r.set(i + 1))
            })
            .collect::<Vec<_>>(),
    )
    .a11y_role(AccessibilityRole::Group)
    .a11y_label("Rating")
}
```

The current value reaches the screen reader through each star's `selected`
state rather than through the container's label. That is deliberate:
`AccessibilityLabel` wraps a plain `Str`, so labels are fixed when the view is
built. Only `AccessibilityState` has a reactive form
(`.a11y_state_signal(...)`). When a value genuinely needs to be *announced* as
it changes, use a real `Slider` — it reports its own value — instead of
relabeling a custom composite.

## Reduced motion

WaterUI does not ship a "prefers reduced motion" signal. Define a marker type,
install it from your backend integration, and pick the animation from it:

```rust,ignore
use core::time::Duration;
use waterui::animation::Animation;
use waterui::prelude::*;

#[derive(Debug, Clone, Copy)]
struct PrefersReducedMotion(bool);

fn entrance(env: &Environment) -> impl View {
    let opacity = Binding::f32(0.0);
    let reduced = env.get::<PrefersReducedMotion>().is_some_and(|p| p.0);

    let animation = if reduced {
        Animation::linear(Duration::ZERO)
    } else {
        Animation::ease_in_out(Duration::from_millis(300))
    };

    text("Welcome")
        .opacity(opacity.with(animation))
        .on_appear(move || opacity.set(1.0))
}
```

A zero-duration animation applies the value immediately, so the same code path
serves both preferences. This matters for users with vestibular disorders —
wire your platform's reduced-motion API into the environment rather than
ignoring the preference.

## Focus

`ViewExt::focused` drives both the visual focus ring and accessibility focus
from one binding:

```rust,ignore
use waterui::prelude::*;

#[derive(Clone, PartialEq, Eq)]
enum Field {
    Name,
    Email,
}

let focus = Binding::container(None::<Field>);
let name = Binding::container(Str::from(""));

field("Name", &name).focused(&focus, Field::Name)
```

Setting `focus` to `Some(Field::Name)` moves VoiceOver or TalkBack focus to
that field.

## Testing the tree

`waterui-testing` drives views through the Hydrolysis accessibility tree, so an
interaction test *is* an accessibility test. Query by role and label, then
assert on the node:

```rust,ignore
use waterui::prelude::*;
use waterui_testing::{Role, SemanticApp};

fn submit_button() -> impl View {
    button("Submit").action(|| {}).disabled(true)
}

#[waterui::test(submit_button)]
fn submit_is_named_and_reports_disabled(app: &mut SemanticApp) {
    let element = app.query().role(Role::BUTTON).label("Submit").single();
    assert!(!element.node().enabled());
}
```

`#[waterui::test(...)]` expands to a plain `#[test]`, so these run under the
normal harness. If a component cannot be reached by role and label, that is a
bug in the component, not a reason to skip the test.

Pair automated checks with the platform auditors before shipping: Accessibility
Inspector on iOS and macOS, Accessibility Scanner on Android, VoiceOver
(Cmd+F5), and Accerciser for AT-SPI on GTK. Ten minutes navigating your own app
with a screen reader turned on finds things no assertion will.

## What's next

Your app is usable regardless of ability. In the [next chapter](06-i18n.md) you
will make it readable regardless of language, with translation catalogs, CLDR
plural rules, and locale-aware formatting.

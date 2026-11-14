# Accessibility

> **In this chapter, you will:**
> - Rely on the labels WaterUI forces every control to carry
> - Hide a label visually without removing it from the accessibility tree
> - Describe custom widgets with `a11y_label`, `a11y_value`, `a11y_role`, and `a11y_state_signal`
> - Choose which rectangle an accessibility element reports with `a11y_geometry`
> - Honor reduced motion and assert on the accessibility tree in a UI test

WaterUI does not treat accessibility as an annotation you add later. Its
control constructors put the accessible name in the signature, so the common
path already produces a labeled control. The work left to you is the part the
type system cannot do: describing custom composites and hiding decoration.

The metadata types live in `waterui::accessibility`; the modifiers are on
`ViewExt` and all start with `a11y_`.

## Labels are mandatory, visibility is not

`button`, `slider`, `stepper`, `toggle`, and `field` all take an `impl
IntoLabel` as their first argument. Screen readers, voice control, switch
control, and command palettes read that label, so a control without one is an
anonymous widget that assistive technology users cannot reach.

Hiding the label is a separate, presentational decision. `.hide_label()`
collapses the label's rendered view to zero size while keeping the semantic
text in the accessibility tree:

```rust
use waterui::prelude::*;

fn playback_position(progress: &Binding<f64>) -> impl View {
    // Announced as "Playback position"; nothing is drawn next to the track.
    slider("Playback position", progress).hide_label()
}
```

`.hide_label()` is shorthand for `.label_style(LabelDisplayMode::Hidden)`.
`LabelDisplayMode` has five variants: `Automatic` (the default),
`TitleAndIcon`, `TitleOnly`, `IconOnly`, and `Hidden`. They choose what is
drawn; no mode drops the semantic text. Install a
`LabelDisplayMode` in the environment with `.install(...)` to set the mode for
a whole subtree.

### Icon-only controls

An icon-only button is a display mode, not a label-less button:

```rust
use waterui::prelude::*;

fn search_button() -> impl View {
    // Still announced as "Search".
    button(label("Search").icon(text("?")).icon_only()).action(|| {})
}
```

`Label::icon` takes any `View + Clone`, so an icon-pack crate works here. The
icon crates and platform support are covered in
[Icons](../03-ui/12-icons.md). `Label::system_icon` renders SF Symbols on Apple
platforms and is explicitly unsupported elsewhere, so portable code uses
`Label::icon` with an icon-set crate such as `waterui-icons-lucide`.

### When the visible content is not the spoken text

`Label::new(semantic_text, content)` is the general constructor. It takes the
text assistive technology announces and a *view builder* (any closure that
returns a view) for what is drawn:

```rust
use waterui::prelude::*;

fn account_button() -> impl View {
    let verified = Label::new(
        "Account, verified",
        || hstack((text("Account"), text("✓"))),
    );

    button(verified).action(|| {})
}
```

A `Label::new` label owns its own layout, so the builders that shape a text
label (`.icon()`, `.system_icon()`, `.leading()`, `.trailing()`, `.spacing()`,
`.font()`) panic when called on it. Compose the icon, spacing, and font inside
the builder instead.

## Describing any view

Four modifiers describe a view for assistive technology. Label and value
accept signals; id and role are constants. None of them rebuilds the subtree.

| Modifier | Argument | Meaning |
|---|---|---|
| `a11y_label` | `impl IntoComputed<Str>` | The name: what the view is |
| `a11y_value` | `impl IntoComputed<Str>` | The content: what it currently says, announced after the label |
| `a11y_id` | `impl Into<Str>` | A constant automation key; never spoken |
| `a11y_role` | `AccessibilityRole` | What kind of element it is |

Use `a11y_label` when the view is not a control and its visual content does not
describe it. Keep it short and leave out prefixes like "Button:"; the role
already communicates that.

```rust
use waterui::prelude::*;

fn logo() -> impl View {
    text("Acme").a11y_label("Acme, home")
}
```

Label and value are signals, so text derived from app state stays current:

```rust
use waterui::prelude::*;

fn inbox(unread: &Binding<i32>) -> impl View {
    let label = unread
        .clone()
        .map(|count| Str::from(format!("{count} unread messages")));

    button("Inbox").action(|| {}).a11y_label(label)
}
```

The value exists so that naming a view does not discard what it says. A chart
can be labeled "Weekly revenue" and carry its summary as the value:
`chart.a11y_label("Weekly revenue").a11y_value("Up 12 percent on last week")`.
`a11y_id` is the odd one out: it is a plain string, not a signal, because a
query key that changes with state defeats its purpose. It feeds
`waterui-testing` selectors and native automation such as `XCUITest` and
`UiAutomator`:

```rust
use waterui::prelude::*;

fn submit() -> impl View {
    button("Submit").action(|| {}).a11y_id("login.submit")
}
```

## Roles

`AccessibilityRole` describes what a view *is*. Built-in controls set their own
role; custom composites need one assigned. The enum is `#[non_exhaustive]`:

| Category | Roles |
|---|---|
| Interactive | `Button`, `Link`, `Checkbox`, `RadioButton`, `Switch`, `Slider` |
| Content | `Text`, `Image`, `Header`, `Footer`, `Article`, `ProgressBar` |
| Landmarks | `Navigation`, `Main`, `Search` |
| Structure | `Section`, `Group`, `Dialog` |
| Collections | `List`, `ListItem`, `Tab`, `TabList`, `TabPanel` |
| Menus | `Menu`, `MenuItem`, `MenuBar`, `MenuItemCheckbox`, `MenuItemRadio` |
| Choice | `Combobox`, `Option` |

Framework containers such as `NavigationView` report their own roles, but a
sidebar you assemble from stacks and buttons does not know it is navigation.
Say so:

```rust
use waterui::accessibility::AccessibilityRole;
use waterui::prelude::*;

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

## States

`AccessibilityState` carries what a label and role cannot express. It is a
const builder, so a state is cheap to construct:

| Builder | Meaning |
|---|---|
| `disabled(bool)` | Visible but not interactive |
| `selected(bool)` | The current selection within its group |
| `checked(Option<bool>)` | `Some(true)` or `Some(false)`; `None` leaves it unset |
| `mixed()` | The indeterminate state of a tri-state checkbox |
| `expanded(Option<bool>)` | Disclosure controls |
| `busy(bool)` | Loading or processing |
| `hidden(bool)` | Not exposed to assistive technology |

Attach a fixed state with `.a11y_state(state)`, or a reactive one with
`.a11y_state_signal(signal)`:

```rust
use waterui::accessibility::AccessibilityState;
use waterui::prelude::*;

fn disclosure(expanded: &Binding<bool>, content: impl View) -> impl View {
    let state = expanded
        .clone()
        .map(|open| AccessibilityState::new().expanded(Some(open)));

    content.a11y_state_signal(state)
}
```

### Modifiers that write state for you

Three `ViewExt` modifiers already write state for you:

- `.disabled(signal)` installs a disabled scope over the subtree. Controls
  render their platform disabled appearance, stop hit-testing, and report the
  disabled state to assistive technology. Nested scopes OR-combine, so an
  enclosing `.disabled(true)` cannot be undone by a child.
- `.visible(signal)` sets `hidden` on the accessibility state while it sets the
  view's opacity to zero and stops hit-testing, so an invisible view is not
  announced.
- `.selected(signal)` puts a control in its selected interaction state, which
  assistive technology announces as selected. It applies to the control it
  modifies, not to controls nested inside it.

## Hiding decoration

Background patterns, dividers, and brand marks add noise to a screen reader.
`a11y_hidden(true)` drops a view from the tree:

```rust
use waterui::prelude::*;

fn decoration() -> impl View {
    text("~ ~ ~").a11y_hidden(true)
}
```

When you have re-described a whole composite with one label, drop its children
instead of hiding the container:

```rust
use waterui::accessibility::AccessibilityChildren;
use waterui::prelude::*;

fn rating_summary() -> impl View {
    hstack((text("★"), text("4.8"), text("(120)")))
        .a11y_label("Rated 4.8 out of 5, 120 reviews")
        .a11y_children(AccessibilityChildren::ExcludeDescendants)
}
```

`AccessibilityChildren::Automatic` is the default and lets the backend decide.

## Geometry

An accessibility element reports a rectangle that screen readers outline and
touch exploration hits. By default it is the response rectangle of the
element's *geometry owner*, positioned in the frame its parent assigned. The
[layout engine chapter](../08-internals/03-layout-engine.md#accessibility-geometry)
states the rule; the practical consequence is that **modifier order decides
which view owns the rectangle**. In `button("OK").padding_with(8.0).a11y_label("x")` the padded view owns
the geometry; in `button("OK").a11y_label("x").padding_with(8.0)` the labeled button
does.

`a11y_geometry` names another source when the element stands for something else
than the view's own answer:

| `AccessibilityGeometry` | Reports |
|---|---|
| `Response` (default) | The view's response rectangle |
| `Allocation` | The whole frame the parent assigned, such as a canvas whose drawing fills its allocation |
| `Region(Rect)` | An explicit rectangle in the owner's allocation-local coordinates |

```rust
use waterui::accessibility::AccessibilityGeometry;
use waterui::prelude::*;

fn plot_surface(drawing: impl View) -> impl View {
    drawing
        .a11y_label("Temperature over time")
        .a11y_geometry(AccessibilityGeometry::Allocation)
}
```

The choice changes accessibility bounds only. Hit testing, focus, and drawing
keep their own geometry.

## A custom control, end to end

A star rating needs a role and a label on the container and on each star. The
current rating is the container's *value*, read after its label, and each star
reports `selected` through a state signal:

```rust
use waterui::accessibility::{AccessibilityRole, AccessibilityState};
use waterui::prelude::*;

fn star_rating(rating: &Binding<i32>, max: i32) -> impl View {
    let value = rating
        .clone()
        .map(move |r| Str::from(format!("{r} of {max}")));

    let stars = (0..max)
        .map(|i| {
            let selected = rating
                .clone()
                .map(move |r| AccessibilityState::new().selected(r > i));
            let opacity = rating
                .clone()
                .map(move |r| if r > i { 1.0_f32 } else { 0.3_f32 });

            text("★")
                .opacity(opacity)
                .a11y_label(Str::from(format!("Rate {} of {max}", i + 1)))
                .a11y_role(AccessibilityRole::Button)
                .a11y_state_signal(selected)
                .on_tap(move |State(rating): State<Binding<i32>>| rating.set(i + 1))
                .state(rating)
        })
        .collect::<Vec<_>>();

    hstack(stars)
        .a11y_role(AccessibilityRole::Group)
        .a11y_label("Rating")
        .a11y_value(value)
}
```

The glyph never changes; only its opacity and its accessibility state do, so
the star views are built once and updated in place. For a plain numeric range,
prefer a real `slider`, which reports its own value without any of this.

## Reduced motion

`ReduceMotion` is an environment flag, `ReduceMotion(pub bool)`, exported from
`waterui`. Transitions on the Apple and Hydrolysis backends resolve it when they
start their clock and map themselves to their declared reduced form. No backend
sets it from the system accessibility preference yet, so install it yourself
on the subtree that should reduce motion, the way a test harness does:
`content.with(ReduceMotion(true))`.

For motion you drive yourself, read the flag and choose the animation:

```rust
use waterui::animation::NoAnimation;
use waterui::env::UseEnv;
use waterui::prelude::*;
use waterui::ReduceMotion;

fn entrance(opacity: &Binding<f32>) -> impl View {
    let opacity = opacity.clone();

    UseEnv::new(move |env: &Environment| {
        let reduced = env.get::<ReduceMotion>().is_some_and(|flag| flag.0);

        if reduced {
            AnyView::new(text("Welcome").opacity(opacity.with(NoAnimation)))
        } else {
            AnyView::new(text("Welcome").opacity(opacity.animated()))
        }
    })
}
```

`NoAnimation` applies the change on the next frame. See
[Animation](01-animation.md) for the animation types.

## Focus

The `focused` binding drives keyboard focus; the chapter on
[keyboard input and focus](09-keyboard-focus.md) covers it. On Hydrolysis a
write to that binding also moves the accessibility tree's focus, while a screen
reader moving its own focus onto a non-text element leaves the text caret where
it was.

## Testing the tree

`waterui-testing` drives views through the Hydrolysis accessibility tree, so an
interaction test is an accessibility test: query by role and label, then assert
on the node. The [testing chapter](../07-tools/03-testing.md) covers the
harness and its dev-dependency. This check pins down the icon-only button from
earlier: nothing draws the word "Search", yet the button is still found by it.

```rust
use waterui::prelude::*;
use waterui_testing::{Role, SemanticApp};

fn search_button() -> impl View {
    button(label("Search").icon(text("?")).icon_only()).action(|| {})
}

#[waterui::test(search_button)]
fn icon_only_button_keeps_its_name(app: &mut SemanticApp) {
    app.query().role(Role::BUTTON).label("Search").assert_exists();
}
# fn main() {}
```

If a component cannot be reached by role and label, that is a bug in the
component, not a reason to skip the test. Before shipping, also navigate your
own app with a screen reader turned on, VoiceOver on Apple platforms and
TalkBack on Android: it finds ordering and verbosity problems no assertion
checks.

## What's next

Your views now describe themselves to assistive technology. In the
[next chapter](06-i18n.md) you
will make it readable regardless of language, with translation catalogs and
locale-aware formatting.

# Conditional rendering

> **In this chapter, you will:**
> - Show and hide views reactively with the `when` function
> - Chain conditions with `.or()` and `.otherwise()` for multi-branch logic
> - Derive boolean conditions from signals using `.map()` and `.equal_to()`
> - Pick between `when` and `match` + `.anyview()` for complex branching

Think about the screens in a typical app: a loading spinner while data fetches, a "Welcome back!" message when the user is logged in, a "Please log in" prompt when they are not. Your UI needs to show different things based on conditions that can change at any moment. WaterUI provides the `when` function for exactly this — unlike Rust's built-in `if`/`else` (which evaluates once at build time), `when` creates reactive branches that automatically swap views as conditions change.

## Basic usage

`when` takes a reactive boolean condition and a builder closure that returns the view to show when the condition is `true`:

```rust,ignore
use waterui::prelude::*;
use waterui::widget::condition::when;

fn maybe_message(show_message: &Binding<bool>) -> impl View {
    when(show_message.clone(), || text("This message is visible!"))
}
```

When `show_message` is `false`, nothing is rendered. The UI updates automatically whenever the binding changes.

## Adding a fallback with `.otherwise()`

Use `.otherwise()` to provide an alternative view when the condition is `false`:

```rust,ignore
use waterui::prelude::*;
use waterui::widget::condition::when;

fn login_state(is_logged_in: &Binding<bool>) -> impl View {
    when(is_logged_in.clone(), || text("Welcome back!"))
        .otherwise(|| text("Please log in"))
}
```

This is the reactive equivalent of an `if`/`else` expression — but it responds to signal changes at runtime.

## Chaining conditions with `.or()`

For multi-branch logic (analogous to `if`/`else if`/`else`), chain `.or()` calls. Each `.or()` adds another conditional branch; the chain must end with `.otherwise()`:

```rust,ignore
use waterui::prelude::*;
use waterui::widget::condition::when;

fn status_text(state: &Binding<i32>) -> impl View {
    when(state.equal_to(0), || text("Loading..."))
        .or(state.equal_to(1), || text("Ready"))
        .or(state.equal_to(2), || text("Error"))
        .otherwise(|| text("Unknown state"))
}
```

The first matching condition wins — subsequent branches are not evaluated.

> **Note:** Think of this as a reactive `match`. The conditions are checked
> in order, and only the first matching branch renders.

## Condition types

`when` accepts any type that implements `IntoComputed<bool>`. In practice you will use a handful of common patterns.

### `Binding<bool>`

The simplest case — a boolean binding directly:

```rust,ignore
use waterui::prelude::*;
use waterui::widget::condition::when;

fn visible(show: &Binding<bool>) -> impl View {
    when(show.clone(), || text("Visible"))
}
```

### Negated binding

`Binding<bool>` implements `Not`, which produces a new signal:

```rust,ignore
use waterui::prelude::*;
use waterui::widget::condition::when;

fn hidden(show: &Binding<bool>) -> impl View {
    // Show only when the binding is `false`.
    when(!show.clone(), || text("Hidden content revealed"))
}
```

### Derived `Computed<bool>`

Any `Computed<bool>` works as a condition. Build one with `SignalExt::map`:

```rust,ignore
use waterui::prelude::*;
use waterui::widget::condition::when;

fn positive_indicator(count: &Binding<i32>) -> impl View {
    let is_positive = count.map(|n| n > 0).computed();
    when(is_positive, || text("Count is positive"))
}
```

### Derived conditions with `SignalExt`

`SignalExt` ships with comparison helpers that produce `Computed<bool>` directly. They are the most readable way to turn a value into a condition:

```rust,ignore
use waterui::prelude::*;
use waterui::widget::condition::when;

fn name_status(name: &Binding<Str>) -> impl View {
    when(name.is_empty(), || text("Please enter your name"))
        .otherwise(|| text!("Hello, {name}!"))
}
```

### `.equal_to()` for value comparison

```rust,ignore
use waterui::prelude::*;
use waterui::widget::condition::when;

fn tab_content(selected_tab: &Binding<i32>) -> impl View {
    when(selected_tab.equal_to(0), || text("Home"))
        .or(selected_tab.equal_to(1), || text("Settings"))
        .otherwise(|| text("Unknown tab"))
}
```

### Static `bool`

Plain `bool` values also work. When all conditions in a chain are static booleans, the framework picks the matching branch at construction time, so the unused branches never cost anything at runtime:

```rust,ignore
use waterui::prelude::*;
use waterui::widget::condition::when;

fn debug_only() -> impl View {
    when(cfg!(debug_assertions), || text("Debug mode"))
        .otherwise(|| text("Release mode"))
}
```

> **Tip:** Use this pattern for feature flags and debug-only UI.

## When to reach for `.anyview()` instead

`when().or().otherwise()` chains are great for two or three branches. For richer matching — especially when each arm constructs a different concrete view type — destructure the value with `match` and erase each arm with `.anyview()`:

```rust,ignore
use waterui::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode { A, B, C }

fn render(mode: Mode) -> AnyView {
    match mode {
        Mode::A => text("Mode A").title().anyview(),
        Mode::B => button("Mode B").action(|| {}).anyview(),
        Mode::C => vstack((text("Header"), text("Body"))).anyview(),
    }
}
```

Use `.anyview()` whenever you need uniform view types across branches and the boolean ladder of `when` is starting to feel like an enum match.

## Rendering mechanics

Understanding how `when` works under the hood helps you write efficient conditional views. Internally, `When` uses the `Dynamic` view to swap content:

1. The combined condition signal re-evaluates.
2. The framework determines which branch index matched.
3. The previous view is removed and the matching branch's builder is called.
4. The new view is inserted into the tree.

Each branch closure runs **every time** the condition switches into that branch, so keep them lightweight. State that should survive a branch toggle must live outside the branch — typically in a `Binding` owned by the parent.

## Patterns and examples

### Show / hide with a toggle

```rust,ignore
use waterui::prelude::*;
use waterui::widget::condition::when;

fn settings_panel() -> impl View {
    let show_advanced = Binding::bool(false);
    let value = Binding::f64(0.5);

    vstack((
        toggle("Show Advanced", &show_advanced),
        when(show_advanced.clone(), {
            let value = value.clone();
            move || {
                vstack((
                    text("Advanced Settings").headline(),
                    slider(&value).range(0.0..=1.0),
                ))
            }
        }),
    ))
}
```

### Loading states

```rust,ignore
use waterui::prelude::*;
use waterui::widget::condition::when;

fn data_view(loading: &Binding<bool>, data: &Binding<Str>) -> impl View {
    let data = data.clone();
    when(loading.clone(), || text("Loading..."))
        .otherwise(move || text!("{data}"))
}
```

### Multi-state status indicator

```rust,ignore
use waterui::prelude::*;
use waterui::widget::condition::when;

fn status_indicator(status: &Binding<i32>) -> impl View {
    when(status.equal_to(0), || text("Idle").color(Grey))
        .or(status.equal_to(1), || text("Running").color(Green))
        .or(status.equal_to(2), || text("Warning").color(Yellow))
        .otherwise(|| text("Error").color(Red))
}
```

## Best practices

1. **Always end chains with `.otherwise()`.** A bare `when()` without
   `.otherwise()` renders nothing when the condition is false. Multi-branch
   chains require `.otherwise()` to close.
2. **Use signal combinators, not `.get()`.** Calling `.get()` inside a `when`
   condition or branch closure breaks reactivity. Prefer `.map()`,
   `.is_empty()`, `.equal_to()`, and friends.
3. **Keep branch closures pure.** Branches return views without side effects.
   They may run multiple times as conditions toggle.
4. **Prefer `when` over Rust `if`/`else` in view bodies.** Rust's `if`
   evaluates once at construction time; `when` updates as conditions change.
5. **Switch to `.anyview()` when branches diverge.** Once you reach four or
   more arms, or each arm produces a different concrete view type, a `match`
   plus `.anyview()` is clearer than a long `when` chain.

## Quick reference

| Pattern                                           | Purpose                                |
|---------------------------------------------------|----------------------------------------|
| `when(cond, \|\| view)`                           | Show view when condition is true       |
| `when(cond, \|\| v).otherwise(\|\| w)`            | If/else                                |
| `when(a, \|\| v).or(b, \|\| w).otherwise(\|\| x)` | If/else-if/else                        |
| `when(!binding, \|\| view)`                       | Show when binding is false             |
| `when(sig.equal_to(val), \|\| view)`              | Compare signal to value                |
| `when(sig.map(\|v\| ...), \|\| view)`             | Derived boolean condition              |
| `match value { Mode::A => a().anyview(), ... }`   | Multi-branch over an enum              |

You now have the tools to build dynamic, condition-driven interfaces. The final piece of the UI puzzle is navigation — how do you move between screens, manage a navigation stack, and organize your app with tabs? That is exactly what the [next chapter](07-navigation.md) covers.

# Conditional rendering

> **In this chapter, you will:**
> - Swap views reactively with `when`, `.or()`, and `.otherwise()`
> - Derive boolean conditions from signals without calling `.get()`
> - Understand that a branch switch destroys the old subtree — and why that is correct
> - Choose between `when`, `.visible()`, a signal-taking API, and a `match` + `.anyview()`

Rust's `if`/`else` runs once, while the view tree is being built. `when` builds a reactive branch instead: the condition is a signal, and the rendered branch follows it.

## Basic usage

`when` takes a reactive boolean and a builder closure for the `true` case:

```rust,ignore
use waterui::prelude::*;
use waterui::widget::condition::when;

fn maybe_message(show_message: &Binding<bool>) -> impl View {
    when(show_message.clone(), || text("This message is visible!"))
}
```

With no `.otherwise()`, a false condition renders nothing.

## Fallbacks and chains

`.otherwise()` supplies the `false` branch:

```rust,ignore
use waterui::prelude::*;
use waterui::widget::condition::when;

fn login_state(is_logged_in: &Binding<bool>) -> impl View {
    when(is_logged_in.clone(), || text("Welcome back!"))
        .otherwise(|| text("Please log in"))
}
```

`.or()` adds further branches, and the chain must be closed with `.otherwise()`:

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

Conditions are checked in order and the first match wins. The chain compiles into a single combined `Computed<Option<usize>>` — the index of the matching branch — so adding branches costs one more zipped signal, not one more subscription per rendered view.

## Building conditions

`when` accepts anything implementing `IntoComputed<bool>`.

```rust,ignore
use waterui::prelude::*;
use waterui::widget::condition::when;

fn examples(show: &Binding<bool>, count: &Binding<i32>, name: &Binding<Str>) -> impl View {
    vstack((
        // A boolean binding directly.
        when(show.clone(), || text("Visible")),
        // Negation: Binding<bool> implements Not and yields a new signal.
        when(!show.clone(), || text("Hidden content revealed")),
        // Any derived Computed<bool>.
        when(count.map(|n| n > 0).computed(), || text("Count is positive")),
        // SignalExt comparison helpers.
        when(name.is_empty(), || text("Please enter your name")),
        when(count.equal_to(42), || text("The answer")),
    ))
}
```

Never call `.get()` to build a condition. `.get()` reads a value once and drops the dependency, so the branch freezes at construction time. `.map()`, `.equal_to()`, `.is_empty()`, and the rest of `SignalExt` keep the dependency intact.

### Static conditions fold away

A plain `bool` is also a signal. When *every* condition in a chain is a static `bool`, the matching branch is selected at construction time and the others are never built:

```rust,ignore
use waterui::prelude::*;
use waterui::widget::condition::when;

fn debug_only() -> impl View {
    when(cfg!(debug_assertions), || text("Debug mode"))
        .otherwise(|| text("Release mode"))
}
```

This is the pattern for feature flags and debug-only UI. Mixing one reactive condition into the chain disables the folding for the whole chain.

## What a branch switch actually does

`When` lowers to `Dynamic::watch` over the combined branch-index signal. When the index changes:

1. The previous subtree is removed.
2. The new branch's builder closure runs.
3. The resulting view is inserted.

**State owned inside a branch is discarded when that branch is replaced.** This is deliberate, not a leak: a new branch is a new component instance, and WaterUI does not infer component identity from call position. Anything that must survive a toggle belongs to a `Binding` owned by the parent and passed in:

```rust,ignore
use waterui::prelude::*;
use waterui::widget::condition::when;

fn settings_panel() -> impl View {
    let show_advanced = Binding::bool(false);
    // Owned by the parent, so the value survives collapsing and re-expanding.
    let quality = Binding::f64(0.5);

    vstack((
        toggle("Show Advanced", &show_advanced),
        when(show_advanced.clone(), {
            let quality = quality.clone();
            move || {
                vstack((
                    text("Advanced Settings").headline(),
                    slider("Quality", &quality).range(0.0..=1.0),
                ))
            }
        }),
    ))
}
```

`Accordion` behaves the same way for the same reason — collapsing it discards the content's state, because collapsing destroys the content.

Branch closures run every time their branch is entered, so keep them free of side effects and cheap to call.

## When *not* to reach for `when`

`when` changes view *structure*. Most reactive UI does not.

| The thing that changes | Use | Not |
|---|---|---|
| A displayed value | `text!("{status}")` | `when` / `watch` around two `text()` calls |
| A parameter of a live view | a signal-taking input, e.g. `.blur(amount.clone())` | rebuilding the view |
| The membership of a collection | `ForEach` / `List` over a reactive collection | `watch` over a `Vec` |
| Whether a subtree is on screen but should keep its state | `.visible(signal)` | `when` |
| Which kind of view is on screen | `when` / `Dynamic::watch` | — |

`watch(binding_of_vec, …)` rebuilds and re-dispatches the entire watched subtree on every change, and can escalate into a full-window structural rebuild. A dynamic set of views is a collection, so render it with `ForEach` or `List` and let membership diff by id — see [the lists chapter](05-lists.md).

### `.visible()` keeps the subtree alive

`.visible(signal)` from `ViewExt` does not swap anything. It drives opacity, hit-testing, and the accessibility hidden state from one signal, so the subtree stays mounted and keeps every piece of state it owns:

```rust,ignore
use waterui::prelude::*;

fn draft_banner(has_draft: &Binding<bool>) -> impl View {
    text("Draft saved").visible(has_draft.clone())
}
```

The trade-off is that the hidden subtree still costs layout and memory. Use `.visible()` for something that toggles often and must not lose state; use `when` for something that is genuinely absent.

## Many branches: `match` plus `.anyview()`

Once each arm produces a different concrete view type, or the ladder grows past three or four rungs, a `match` over an enum reads better than a `when` chain. `.anyview()` erases the arms to a common type:

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

To make that reactive, wrap it in `Dynamic::watch(mode_signal, render)` — the same mechanism `when` uses, written directly. The state-loss rule is identical.

## Quick reference

| Pattern | Purpose |
|---|---|
| `when(cond, \|\| view)` | Show a view while the condition is true |
| `when(cond, \|\| v).otherwise(\|\| w)` | If / else |
| `when(a, \|\| v).or(b, \|\| w).otherwise(\|\| x)` | If / else-if / else |
| `when(!binding, \|\| view)` | Show while the binding is false |
| `when(sig.equal_to(val), \|\| view)` | Compare a signal to a value |
| `view.visible(sig)` | Hide without unmounting |
| `Dynamic::watch(sig, \|v\| …)` | Structural swap driven by any signal |

The last piece of the UI puzzle is moving between screens. The [next chapter](07-navigation.md) covers navigation stacks, tabs, and split views.

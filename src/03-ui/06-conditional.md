# Conditional rendering

> **In this chapter, you will:**
> - Swap views reactively with `when`, `.or()`, and `.otherwise()`
> - Derive boolean conditions from signals without reading them with `.snapshot()`
> - Understand that a branch switch rebuilds the subtree and drops its state, and why that is correct
> - Choose between `when`, `Option<impl View>`, `.visible()`, and a `match` with `.anyview()`

Rust's `if`/`else` runs once, while the view tree is being built. When the condition is a signal, the branch has to follow it, and `when` does that: the condition is reactive and the rendered branch tracks it.

## Basic usage

`when` takes a reactive boolean and a builder closure for the `true` case:

```rust
use waterui::prelude::*;
use waterui::widget::condition::when;

fn maybe_message(show_message: &Binding<bool>) -> impl View {
    when(show_message.clone(), || text("This message is visible!"))
}
```

With no `.otherwise()`, a false condition renders nothing.

## Fallbacks and chains

`.otherwise()` supplies the `false` branch:

```rust
use waterui::prelude::*;
use waterui::widget::condition::when;

fn login_state(is_logged_in: &Binding<bool>) -> impl View {
    when(is_logged_in.clone(), || text("Welcome back!"))
        .otherwise(|| text("Please log in"))
}
```

`.or()` adds further branches, and the chain is closed with `.otherwise()`:

```rust
use waterui::prelude::*;
use waterui::widget::condition::when;

fn status_text(state: &Binding<i32>) -> impl View {
    when(state.equal_to(0), || text("Loading..."))
        .or(state.equal_to(1), || text("Ready"))
        .or(state.equal_to(2), || text("Error"))
        .otherwise(|| text("Unknown state"))
}
```

Conditions are checked in order and the first match wins. The chain compiles into a single `Computed<Option<usize>>`, the index of the matching branch, so each extra branch adds one zipped signal and the host view subscribes once.

## Building conditions

`when` accepts anything that implements `IntoComputed<bool>`: a `Binding<bool>`, a `Computed<bool>`, a derived signal, or a plain `bool`.

```rust
use waterui::prelude::*;
use waterui::widget::condition::when;

fn examples(show: &Binding<bool>, count: &Binding<i32>, name: &Binding<Str>) -> impl View {
    vstack((
        // A boolean binding directly.
        when(show.clone(), || text("Visible")),
        // Negation: `!` on a Binding<bool> yields the inverted binding.
        when(!show.clone(), || text("Hidden content revealed")),
        // A predicate over the binding's value.
        when(count.condition(|n| *n > 0), || text("Count is positive")),
        when(name.condition(|s| s.is_empty()), || text("Please enter your name")),
        // Equality against a fixed value.
        when(count.equal_to(42), || text("The answer")),
    ))
}
```

Do not call `.snapshot()` to build a condition. `.snapshot()` reads the current value once and drops the dependency, so the branch is frozen at construction time. `.condition()`, `.equal_to()`, `.map()`, and the rest of `SignalExt` keep the dependency. `.snapshot()` is for event handlers such as a button action, which run outside the reactive graph.

### Static conditions fold away

A plain `bool` is also a valid condition. When every condition in a chain is a plain `bool`, the matching branch is chosen at construction time and the other builders never run:

```rust
use waterui::prelude::*;
use waterui::widget::condition::when;

fn build_flavor() -> impl View {
    when(cfg!(debug_assertions), || text("Debug build"))
        .otherwise(|| text("Release build"))
}
```

This suits feature flags and debug-only UI. A single reactive condition anywhere in the chain turns the whole chain into the reactive form.

## A plain `bool` and `Option`

`Option<V>` is a view when `V` is: `None` renders nothing and `Some(view)` renders the view. For a value that is known when the view is built, `bool::then` is enough:

```rust
use waterui::prelude::*;

fn unread_badge(unread: usize) -> impl View {
    (unread > 0).then(|| text("New"))
}
```

The decision is made once, in Rust, so it never updates. A signal of `Option<impl View>` is not a view. When the condition changes over time, use `when` or `.visible()`.

## What a branch switch does

The chain lowers to `Dynamic::watch` over the combined branch-index signal. Each time one of the conditions notifies:

1. The builder closure of the branch that now matches runs and produces a new view.
2. That view replaces the previous subtree, and the previous subtree is dropped.

The branch index is not deduplicated. A notification that leaves the same branch selected, such as setting `state` in the chain above from `5` to `6`, both of which fall through to `.otherwise()`, still rebuilds that branch.

**State owned inside a branch is discarded every time the branch is rebuilt.** This is the intended behavior, not a leak: a new branch is a new component instance, and WaterUI never infers component identity from where a call sits in the body. Anything that must survive a toggle is a `Binding` owned by the parent and passed in:

```rust
use waterui::prelude::*;
use waterui::widget::condition::when;

fn settings_panel() -> impl View {
    let show_advanced = Binding::bool(false);
    // Owned by the parent, so the value survives collapsing and re-expanding.
    let quality = Binding::f64(0.5);

    vstack((
        toggle("Show advanced", &show_advanced),
        when(show_advanced.clone(), move || {
            slider("Quality", &quality).range(0.0..=1.0)
        }),
    ))
}
```

A builder closure runs every time the chain rebuilds its branch, so keep it cheap and free of side effects.

When tearing a view down is itself costly or lossy, such as a `ParticleSystem` that restarts its simulation or a GPU view that re-uploads its resources, keep every layer mounted and cross-fade their `.opacity` signals instead of switching branches.

## When not to reach for `when`

`when` changes view structure. Most reactive UI does not need that.

| The thing that changes | Use | Not |
|---|---|---|
| A displayed value | `text!("{status}")` | `when` or `watch` around two `text()` calls |
| A parameter of a live view | a signal-taking input, for example `.opacity(amount.clone())` | rebuilding the view |
| The membership of a collection | `ForEach` or `List` over a reactive collection | `watch` over a `Vec` |
| A subtree that stays on screen and must keep its state | `.visible(signal)` | `when` |
| Which kind of view is on screen | `when` or `Dynamic::watch` | none |

`watch` over a `Vec` rebuilds the entire watched subtree on every change. A dynamic set of views is a collection, so render it with `ForEach` or `List` and let membership diff by id; see [the lists chapter](05-lists.md).

### `.visible()` keeps the subtree alive

`.visible(signal)` from `ViewExt` swaps nothing. One signal drives opacity, hit-testing, and the accessibility hidden state, so the subtree stays mounted and keeps every piece of state it owns:

```rust
use waterui::prelude::*;

fn draft_banner(has_draft: &Binding<bool>) -> impl View {
    text("Draft saved").visible(has_draft.clone())
}
```

The trade-off is that a hidden subtree still takes part in layout and stays in memory. Use `.visible()` for something that toggles often and must not lose state, and `when` for something that is genuinely absent.

## Many branches: `match` and `.anyview()`

When each arm produces a different concrete view type, or the ladder grows past three or four rungs, a `match` over an enum reads better than a `when` chain. `.anyview()` erases the arms to a common `AnyView`:

```rust
use waterui::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    List,
    Detail,
    Empty,
}

fn render(mode: Mode) -> AnyView {
    match mode {
        Mode::List => text("Items").anyview(),
        Mode::Detail => button("Open").action(|| {}).anyview(),
        Mode::Empty => vstack((text("Nothing here"), text("Add an item"))).anyview(),
    }
}
```

To make the match reactive, hand the signal and the function to `Dynamic::watch`, the same mechanism `when` uses:

```rust
use waterui::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    List,
    Empty,
}

fn render(mode: Mode) -> AnyView {
    match mode {
        Mode::List => text("Items").anyview(),
        Mode::Empty => text("Nothing here").anyview(),
    }
}

fn screen(mode: &Binding<Mode>) -> impl View {
    Dynamic::watch(mode.clone(), render)
}
```

The state rule is identical: each change of `mode` rebuilds the subtree.

## Quick reference

| Pattern | Purpose |
|---|---|
| `when(cond, \|\| view)` | Show a view while the condition is true |
| `when(cond, \|\| v).otherwise(\|\| w)` | If / else |
| `when(a, \|\| v).or(b, \|\| w).otherwise(\|\| x)` | If / else-if / else |
| `when(!binding, \|\| view)` | Show while the binding is false |
| `when(sig.equal_to(val), \|\| view)` | Compare a signal to a value |
| `(flag).then(\|\| view)` | A condition known at build time |
| `view.visible(sig)` | Hide without unmounting |
| `Dynamic::watch(sig, f)` | Structural swap driven by any signal |

The last piece of the UI puzzle is moving between screens. The [next chapter](07-navigation.md) covers navigation stacks, tabs, and split views.

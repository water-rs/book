# Your First App

> **In this chapter, you will:**
> - Build a counter application from an empty `src/lib.rs`
> - See how views, stacks, and reactive bindings fit together
> - Wire buttons to state through the `State<T>` extractor
> - Run the same code on macOS, iOS, Android, and Linux

## Create the project

```bash
water create "Counter" --mode playground
cd counter
```

You get four files:

```text
counter/
  Cargo.toml
  Water.toml
  src/lib.rs
  .gitignore
```

The generated `src/lib.rs` is a demo screen. Replace it as you work through the
steps below.

## Step 1: a minimal view

```rust,ignore
use waterui::app::App;
use waterui::prelude::*;

fn main() -> impl View {
    "Hello, WaterUI!"
}

pub fn app(env: Environment) -> App {
    App::new(main, env)
}
```

- `use waterui::prelude::*` brings in `View`, `Environment`, `Binding`,
  `State`, the layout and control constructors, and the macros.
- `fn main() -> impl View` is the root view. `&'static str` implements `View`,
  so a bare literal renders as text.
- `pub fn app(env: Environment) -> App` is the entry point. The backend hands
  you an `Environment` carrying theme tokens, locale, and platform services.

Your crate stops there. The `water` CLI generates a separate FFI companion
crate -- in the managed cache for playgrounds, at `backends/ffi/` for app
projects -- which depends on your crate and exports the C entry points native
backends load. Never write `waterui_ffi::export!()` in `src/lib.rs`.

```bash
water run --platform macos
```

## Step 2: styled text

`text()` builds a `Text` view you can configure by chaining:

```rust,ignore
fn main() -> impl View {
    text("Hello, WaterUI!").title().bold()
}
```

`.title()`, `.headline()`, `.sub_headline()`, `.body()`, `.caption()`, and
`.footnote()` select semantic font presets that resolve against the platform's
type scale. Each one replaces the whole font, so apply the preset **first** and
weight or size adjustments after -- `.bold().title()` discards the bold.

`.size(24.0)`, `.italic(true)`, and `.underline(true)` accept signals as well as
plain values, so any of them can be driven by a `Binding`.

Use `text()` for fixed strings and the `text!` macro whenever the content
depends on reactive state.

## Step 3: layout with stacks

`vstack` arranges children top to bottom, `hstack` left to right, and both take
a **tuple** so each child keeps its own type -- no boxing, no common trait
object.

```rust,ignore
fn main() -> impl View {
    vstack((
        text("Counter App").title().bold(),
        "A simple counting application",
    ))
}
```

Nest them freely: an `hstack` inside a `vstack` is how most real layouts get
built.

## Step 4: reactive state

`Binding<T>` holds mutable state. Views that read it update when it changes --
no refresh call, no diff pass.

```rust,ignore
fn main() -> impl View {
    let counter = Binding::i32(0);

    vstack((
        text("Counter App").title().bold(),
        text!("Count: {counter}"),
    ))
}
```

- `Binding::i32(0)` creates a `Binding<i32>`. There are typed constructors for
  `u32`, `u64`, `usize`, `i32`, `i64`, `isize`, `f32`, `f64`, and `bool`. For
  anything else use `Binding::container(value)`. There is no `Binding::new`.
- `text!("Count: {counter}")` interpolates named placeholders that match a
  binding in scope, or an explicit alias such as
  `text!("Count: {n}", n = counter)`. It subscribes to `counter` and rewrites
  only that string when the value changes.

> **Important:** never call `.get()` on a signal inside a view body. That reads
> the value once and severs the dependency. Derive instead: `text!` for text,
> `.map()` / `.zip()` for computed values, and pass the resulting signal into
> whichever component input needs it.

## Step 5: buttons and actions

```rust,ignore
pub fn main() -> impl View {
    let counter = Binding::i32(0);

    vstack((
        text("Counter App").title().bold(),
        text!("Count: {counter}"),
        hstack((
            button("Decrement")
                .action(|State(c): State<Binding<i32>>| *c.get_mut() -= 1)
                .state(&counter),
            button("Increment")
                .action(|State(c): State<Binding<i32>>| *c.get_mut() += 1)
                .state(&counter),
        )),
    ))
}
```

Three pieces:

1. `button("Increment")` takes an `impl IntoLabel`. A string literal becomes a
   semantic label that feeds both the visible text and the accessibility tree,
   so every button carries a label by construction.
2. `.action(...)` runs on activation. Each `State<T>` parameter pulls a value of
   that type out of the button's environment; repeated parameters of the same
   type are matched in injection order.
3. `.state(&counter)` injects a value. Chain one `.state(...)` per `State<T>`
   parameter.

`*c.get_mut() += 1` mutates through a guard and notifies watchers on drop --
prefer it over `c.set(c.get() + 1)`. Reading with `.get()` inside an action
closure is fine; the rule against `.get()` applies to view bodies.

Beyond four injected states, bundle the values into one `#[derive(Clone)]`
struct and inject that instead of stacking parameters.

### Button styles

```rust,ignore
button("Submit").bordered_prominent().action(|| { /* ... */ });  // primary
button("Cancel").bordered().action(|| { /* ... */ });            // secondary
button("Learn more").link().action(|| { /* ... */ });            // hyperlink
button("Skip").plain().action(|| { /* ... */ });                 // no chrome
button("Subtle").borderless().action(|| { /* ... */ });
```

Style is an attribute, not a separate type: the same `Button` renders as a
platform-appropriate control in each style. `.disabled(signal)` takes a
`bool` signal and switches the button to its platform disabled appearance while
reporting the state to assistive technology.

### Async actions

```rust,ignore
button("Fetch Data")
    .action_async(|| async {
        let data = fetch_from_server().await;
        process(data);
    });
```

## Step 6: spacers

`spacer()` expands to absorb the free space in its stack:

```rust,ignore
pub fn main() -> impl View {
    let counter = Binding::i32(0);

    vstack((
        text("Counter App").title().bold(),
        spacer(),
        text!("Count: {counter}").size(48.0),
        spacer(),
        hstack((
            button("Decrement")
                .bordered()
                .action(|State(c): State<Binding<i32>>| *c.get_mut() -= 1)
                .state(&counter),
            spacer(),
            button("Increment")
                .bordered_prominent()
                .action(|State(c): State<Binding<i32>>| *c.get_mut() += 1)
                .state(&counter),
        )),
    ))
}
```

The two spacers in the `vstack` pin the title to the top and the buttons to the
bottom; the one in the `hstack` pushes the buttons to opposite edges. Use
`spacer_min(length)` when you want a guaranteed minimum gap.

## The complete counter

```rust,ignore
use waterui::app::App;
use waterui::prelude::*;
use waterui::preview;

#[preview]
pub fn main() -> impl View {
    let counter = Binding::i32(0);

    vstack((
        text("Counter App").title().bold(),
        spacer(),
        text!("Count: {counter}").size(48.0),
        spacer(),
        hstack((
            button("Decrement")
                .bordered()
                .action(|State(c): State<Binding<i32>>| *c.get_mut() -= 1)
                .state(&counter),
            spacer(),
            button("Increment")
                .bordered_prominent()
                .action(|State(c): State<Binding<i32>>| *c.get_mut() += 1)
                .state(&counter),
        )),
    ))
    .padding()
}

pub fn app(env: Environment) -> App {
    App::new(main, env)
}
```

`.padding()` inserts platform-appropriate insets around the whole stack.
`#[preview]` makes the function addressable without launching the app:

```bash
water preview main --output counter.png
```

## Running on other platforms

```bash
water run --platform macos
water run --platform ios
water run --platform android
water run --platform linux     # GTK4
```

The buttons render as UIKit controls on iOS, Material components on Android,
and GTK4 widgets on Linux, from the same source with no `#[cfg]` branches.

## Next steps

Try adding a "Reset" button, or a second binding that controls the increment
step. Then continue to
[Project Structure and Water.toml](04-project-structure.md) to see how a
project is organised and configured.

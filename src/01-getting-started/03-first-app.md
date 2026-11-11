# Your first app

> **In this chapter, you will:**
> - Create a project with `water create` and find the one function a backend calls
> - Build a counter from an empty `src/lib.rs`, one concept per step
> - Keep state in a `Binding` and read it in views without ever snapshotting it
> - Wire buttons to state with `.action(...)` and `.state(...)`
> - Run the same code on the host platform, iOS, and Android

A counter is the smallest program that needs all three pieces of a WaterUI app:
a view tree, state that changes, and an event that changes it. Building one
shows how those pieces connect before you meet the full component catalog.

## Create the project

```bash
water create "My App"
cd my-app
```

The folder name is the kebab-case form of the display name. In an interactive
terminal the command asks for a bundle identifier and offers
`dev.waterui.myApp`, derived from the name; pass `--bundle-id` to set it up
front.

There are no modes or package types to choose. WaterUI owns the program entry
and the CLI generates and manages every backend project, so your directory
holds only your own code:

```text
my-app/
  Water.toml
  Cargo.toml
  assets/
  src/lib.rs
```

The generated `src/lib.rs` is a demo screen. Replace it as you work through the
steps below. [Project structure](04-project-structure.md) covers the rest of the
layout and [the CLI chapter](01-cli.md) covers the commands.

## Step 1: a minimal view

```rust
use waterui::app::App;
use waterui::prelude::*;

fn root() -> impl View {
    "Hello, WaterUI!"
}

pub fn app(env: Environment) -> App {
    App::new(root, env)
}
```

- `use waterui::prelude::*` brings in `View`, `Environment`, `Binding`, `State`,
  the layout and control constructors, and the macros.
- `fn root() -> impl View` is the root view. A `&'static str` implements `View`,
  so a bare literal renders as text.
- `pub fn app(env: Environment) -> App` is the entry point every backend calls.
  The backend hands in the `Environment`, the typed value store every view
  reads from, and `App::new(root, env)` opens one window whose content is the
  view `root` builds.

That function is all your crate exports. The `water` CLI generates the native
host and the glue that calls `app`; you never write an FFI export yourself. Run
it on your host platform:

```bash
water run
```

`water run` without `--platform` targets the machine you are on. It stays
attached while the app is alive and streams its logs.

## Step 2: styled text

`text()` builds a `Text` view you can configure by chaining:

```rust
use waterui::prelude::*;

fn root() -> impl View {
    text("Hello, WaterUI!").title().bold()
}
```

`.title()`, `.headline()`, `.sub_headline()`, `.body()`, `.caption()`, and
`.footnote()` select semantic font presets that resolve against the platform's
type scale. Each one replaces the whole font, so apply the preset **first** and
weight or size adjustments after: `.bold().title()` discards the bold.

`.size(24.0)`, `.italic(true)`, and `.underline(true)` accept signals as well as
plain values, so a `Binding` can drive any of them. See
[the Text chapter](../03-ui/01-text.md).

Use `text()` for fixed strings and the `text!` macro whenever the content
depends on reactive state.

## Step 3: layout with stacks

`vstack` arranges children top to bottom, `hstack` side by side from the
leading edge to the trailing edge. Both take a
**tuple**, so each child keeps its own type: no boxing, no common trait object.

```rust
use waterui::prelude::*;

fn root() -> impl View {
    vstack((
        text("Counter").title().bold(),
        "A simple counting application",
    ))
}
```

Nest them freely: an `hstack` inside a `vstack` is how most real layouts get
built. The sizing rules are in [the Layout chapter](../03-ui/02-layout.md).

## Step 4: reactive state

`Binding<T>` holds mutable state. A view that reads it updates when it changes,
with no refresh call and no diff pass.

```rust
use waterui::prelude::*;

fn root() -> impl View {
    let counter = Binding::i32(0);

    vstack((
        text("Counter").title().bold(),
        text!("Count: {counter}"),
    ))
}
```

- `Binding::i32(0)` creates a `Binding<i32>`. Typed constructors exist for
  `i32`, `i64`, `isize`, `u32`, `u64`, `usize`, `f32`, `f64` and `bool`. Any
  other `Clone` type goes through `Binding::container(value)`, or through the
  free function `binding(value)`, which converts with `Into` and so takes its
  type from an annotation: `let name: Binding<String> = binding("Ada");`.
- `text!("Count: {counter}")` interpolates named placeholders that match a
  binding in scope, or an explicit alias such as
  `text!("Count: {n}", n = counter)`. It subscribes to `counter` and rewrites
  only that string when the value changes.

> **Important:** never call `.snapshot()` on a signal inside a view body. It
> reads the value once and severs the dependency, and nothing fails to compile.
> Derive instead: `text!` for text, `.map()` for computed values, and pass the
> resulting signal into whichever component input needs it.

`.snapshot()` is the `Signal` method that reads the current value. It belongs in
event handlers and `.map()` closures, where you want the value at that instant.
The next step uses it. [The Reactive chapter](../02-core/02-reactive.md) covers
signals in depth.

## Step 5: buttons and actions

```rust
use waterui::prelude::*;

fn root() -> impl View {
    let counter = Binding::i32(0);

    vstack((
        text("Counter").title().bold(),
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
2. `.action(...)` runs on activation. Its parameters are extractors resolved from
   the button's environment, and `State<T>` is the extractor for a value of type
   `T`.
3. `.state(&counter)` puts a value into that environment. Chain one `.state(...)`
   per `State<T>` parameter. Repeated parameters of the same type bind
   positionally: the first `.state(...)` call feeds the first parameter of that
   type.

`*c.get_mut() -= 1` mutates through a guard and notifies watchers when the guard
drops. Prefer it to `c.set(c.snapshot() - 1)`. Do not bind the guard to a
variable, even `_`: the guard would live to the end of the scope.

An action may also take no parameters at all (`.action(|| { ... })`), which is
the right form when it needs nothing from the environment. Beyond two or three
injected values, bundle them in one `Clone` struct marked `#[state]` and inject
that once instead; see [the Controls chapter](../03-ui/03-controls.md).

### Button styles

Style is an attribute, not a separate type. The same `Button` renders as the
platform's control in whichever style you pick:

```rust
use waterui::prelude::*;

fn buttons() -> impl View {
    vstack((
        button("Submit").bordered_prominent().action(|| {}), // primary
        button("Cancel").bordered().action(|| {}),           // secondary
        button("Learn more").link().action(|| {}),           // hyperlink
        button("Skip").plain().action(|| {}),                // no chrome
        button("Subtle").borderless().action(|| {}),
    ))
}
```

`.style(ButtonStyle::...)` takes the same choice as a value. To disable a button,
use the `.disabled(...)` modifier from `ViewExt`, which takes a `bool` or a
`bool` signal. It disables every control in the subtree it wraps: they take the
platform's disabled appearance, stop responding to input, and report the state
to assistive technology:

```rust
use waterui::prelude::*;

fn submit() -> impl View {
    let busy = Binding::bool(false);
    button("Submit").action(|| {}).disabled(busy)
}
```

### Async actions

`.action_async(...)` takes a handler that returns a future. The future runs on
the local executor, and the button does not wait for it:

```rust
use waterui::prelude::*;

fn refresh() -> impl View {
    let status = Binding::container(Str::from("idle"));

    vstack((
        text!("Status: {status}"),
        button("Refresh")
            .action_async(|State(status): State<Binding<Str>>| async move {
                status.set(Str::from("done"));
            })
            .state(&status),
    ))
}
```

## Step 6: spacers

`spacer()` expands to absorb the free space along its stack's main axis:

```rust
use waterui::prelude::*;

fn root() -> impl View {
    let counter = Binding::i32(0);

    vstack((
        text("Counter").title().bold(),
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
`spacer_min(length)` for a guaranteed minimum gap.

## The complete counter

```rust
use waterui::app::App;
use waterui::prelude::*;
use waterui::preview;

#[preview]
fn root() -> impl View {
    let counter = Binding::i32(0);

    vstack((
        text("Counter").title().bold(),
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
    App::new(root, env)
}
```

`.padding()` insets the whole stack by the default 14 points on every edge.
`#[preview]` makes the function addressable by name without launching the app:

```bash
water preview root --output counter.png
```

[The Preview chapter](../07-tools/01-preview.md) covers the options.

## Running on other platforms

```bash
water run                      # the host platform
water run --platform ios
water run --platform android
```

The same source runs on each platform with no `#[cfg]` branches. The Apple
backend realizes the buttons as AppKit and UIKit controls; Android is drawn by
the Hydrolysis backend. Which platforms and backends are supported, and how to
pick a backend with `--backend`, is in
[the Platforms chapter](05-platforms.md).

## Next steps

Try adding a "Reset" button, or a second binding that controls the increment
step. Then continue to [Project structure](04-project-structure.md) to see how a
project is organized and configured, or jump to
[the View chapter](../02-core/01-view.md) to learn what `impl View` really means.

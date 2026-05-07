# Your First App

> **In this chapter, you will:**
> - Build a counter application from scratch
> - Learn how views, layout stacks, and reactive state work together
> - Add buttons with actions that update the UI automatically
> - Run the same code on macOS, iOS, Android, and Linux

There is no better way to learn a UI framework than to build something with it.
In this chapter, you will create a counter app -- simple enough to understand in
one sitting, but rich enough to introduce the core WaterUI patterns you will use
in every project: views, layout, reactive state, and user interaction.

## Create the project

Scaffold a playground project:

```bash
water create "Counter" --mode playground
cd counter
```

This generates the following files:

```text
counter/
  Cargo.toml
  Water.toml
  src/lib.rs
  assets/
    raw/
    images/
```

Open `src/lib.rs` in your editor. The template includes a full demo app, but
you will replace it with your own code, building it up step by step.

## Step 1: a minimal view

Replace the contents of `src/lib.rs` with the simplest possible WaterUI app:

```rust
use waterui::app::App;
use waterui::prelude::*;

fn main() -> impl View {
    "Hello, WaterUI!"
}

pub fn app(env: Environment) -> App {
    App::new(main, env)
}
```

Here is what each piece does:

- **`use waterui::prelude::*`** imports all commonly used items: `View`,
  `Environment`, `Binding`, layout functions, control constructors, macros,
  and more.
- **`fn main() -> impl View`** is the root view function. It returns any
  type that implements the `View` trait. A `&'static str` implements `View`,
  so a bare string literal is a valid view that renders as text.
- **`pub fn app(env: Environment) -> App`** is the application entry point.
  The native backends call this function through the generated FFI companion
  crate to obtain the `App` instance. The `Environment` is passed in by the
  backend and carries platform-provided services such as theme information.

You do not need to write `waterui_ffi::export!()` yourself. In playground
mode, the CLI generates an FFI companion crate behind the scenes that calls
your `app(env)` function and exports the C entry points the native backends
expect. In app mode, the same companion lives at `backends/ffi/`.

Run it:

```bash
water run --platform macos
```

You should see a window displaying "Hello, WaterUI!" rendered with native
platform text.

> **Tip:** Try changing the string to something else and re-running
> `water run`. Each invocation rebuilds the project incrementally.

## Step 2: using the text view

String literals work, but the `text()` function and `text!` macro give you
control over styling and reactive interpolation. Use `text()` for static
strings and `text!` whenever the displayed value depends on a reactive
binding.

```rust
fn main() -> impl View {
    text("Hello, WaterUI!").bold().title()
}
```

The `text()` function creates a `Text` view. Method calls chain to configure
it:

- `.bold()` sets the font weight to bold.
- `.title()` selects the platform's title font preset.
- Other options include `.size(24.0)`, `.italic(true)`, `.underline(true)`,
  `.headline()`, `.caption()`, and more.

Now that you can display styled text, arrange multiple views together.

## Step 3: layout with vstack and hstack

A single text view is not much of an app. WaterUI uses **stacks** to arrange
views:

- `vstack((...))` arranges children **vertically** (top to bottom).
- `hstack((...))` arranges children **horizontally** (left to right).

Children are passed as a **tuple**:

```rust
fn main() -> impl View {
    vstack((
        text("Counter App").bold().title(),
        "A simple counting application",
    ))
}
```

`vstack` accepts a tuple of views. Each element can be a different type --
the framework composes them without forcing you to box the children.

> **Tip:** Try nesting an `hstack` inside a `vstack` to see how stacks
> compose. This nesting pattern is how you build complex layouts in WaterUI.

## Step 4: adding reactive state

Now for the interesting part. WaterUI uses **reactive bindings** from the
`nami` crate to manage state. When a binding's value changes, any view that
depends on it updates automatically -- no manual refresh calls, no diffing.

Create a binding with one of the typed `Binding` constructors:

```rust
fn main() -> impl View {
    let counter = Binding::i32(0);

    vstack((
        text("Counter App").bold().title(),
        text!("Count: {counter}"),
    ))
}
```

Key concepts:

- **`Binding::i32(0)`** creates a `Binding<i32>` initialised to `0`. There
  are typed constructors for the common primitive shapes:
  `Binding::i32`, `Binding::u32`, `Binding::f64`, `Binding::bool`. For heap
  types such as `String`, use `Binding::container(String::new())`. There is
  no `Binding::new`.
- **`text!("Count: {counter}")`** is the `text!` macro. It only accepts
  **named** placeholders that match a binding in scope (or an explicit
  alias such as `text!("Count: {n}", n = counter)`). When `counter` changes,
  the text updates automatically.

> **Important:** Do not call `.get()` on signals directly inside a view
> body. Doing so reads the value once and breaks reactivity. Instead, use
> `text!`, `watch()`, `.map()`, or `.zip()` to create derived signals that
> track changes.

The display updates, but there is no way to change the count yet. Add some
buttons.

## Step 5: buttons and actions

A counter needs buttons. The `button()` function creates a `Button` view:

```rust
pub fn main() -> impl View {
    let counter = Binding::i32(0);

    vstack((
        text("Counter App").bold().title(),
        text!("Count: {counter}"),
        hstack((
            button("Decrement")
                .state(&counter)
                .action(|State(c): State<Binding<i32>>| c.set(c.get() - 1)),
            button("Increment")
                .state(&counter)
                .action(|State(c): State<Binding<i32>>| c.set(c.get() + 1)),
        )),
    ))
}
```

Breaking down the button pattern:

1. **`button("Increment")`** creates a button with a text label. The label
   can be any `View`, not just a string.
2. **`.state(&counter)`** injects the `counter` binding into the button's
   environment. Chain multiple `.state(...)` calls to inject multiple values --
   each becomes available to the action closure through a `State<T>`
   parameter.
3. **`.action(|State(c): State<Binding<i32>>| ...)`** runs when the button is
   clicked. Each `State<T>` parameter extracts the matching injected value
   from the environment, in the order it was injected.

Inside the action, `c.get()` reads the current value and `c.set(...)` writes
a new one. The write triggers the reactive system, which updates the
`text!("Count: {counter}")` view.

Run this and you have a working counter. Click the buttons and watch the
count change in real time.

### Button styles

Buttons support several visual styles:

```rust
// Primary action (filled background)
button("Submit").bordered_prominent().action(|| { /* ... */ });

// Secondary action (bordered)
button("Cancel").bordered().action(|| { /* ... */ });

// Link style (hyperlink appearance)
button("Learn more").link().action(|| { /* ... */ });

// Plain (no background or border)
button("Skip").plain().action(|| { /* ... */ });
```

> **Tip:** Try changing `.bordered_prominent()` to `.link()` on one of your
> counter buttons to see how the style affects the appearance on your platform.

### Async actions

For actions that need to perform asynchronous work, use `action_async`:

```rust
button("Fetch Data")
    .action_async(|| async {
        let data = fetch_from_server().await;
        process(data);
    });
```

## Step 6: adding a spacer

Use `spacer()` to push views apart within a stack:

```rust
pub fn main() -> impl View {
    let counter = Binding::i32(0);

    vstack((
        text("Counter App").bold().title(),
        spacer(),
        text!("Count: {counter}").size(48.0),
        spacer(),
        hstack((
            button("Decrement")
                .bordered()
                .state(&counter)
                .action(|State(c): State<Binding<i32>>| c.set(c.get() - 1)),
            spacer(),
            button("Increment")
                .bordered_prominent()
                .state(&counter)
                .action(|State(c): State<Binding<i32>>| c.set(c.get() + 1)),
        )),
    ))
}
```

Spacers are flexible -- they expand to fill all remaining space. In this
layout:

- The two `spacer()` calls in the `vstack` push the title to the top and the
  buttons to the bottom, centering the count in between.
- The `spacer()` in the `hstack` pushes the two buttons to opposite edges.

## The complete counter app

Here is the full `src/lib.rs`:

```rust
use waterui::app::App;
use waterui::prelude::*;

pub fn main() -> impl View {
    let counter = Binding::i32(0);

    vstack((
        text("Counter App").bold().title(),
        spacer(),
        text!("Count: {counter}").size(48.0),
        spacer(),
        hstack((
            button("Decrement")
                .bordered()
                .state(&counter)
                .action(|State(c): State<Binding<i32>>| c.set(c.get() - 1)),
            spacer(),
            button("Increment")
                .bordered_prominent()
                .state(&counter)
                .action(|State(c): State<Binding<i32>>| c.set(c.get() + 1)),
        )),
    ))
    .padding()
}

pub fn app(env: Environment) -> App {
    App::new(main, env)
}
```

Note the `.padding()` call at the end -- this adds platform-appropriate
padding around the entire stack, preventing content from touching the screen
edges.

> **Tip:** Try extending this app on your own. Add a "Reset" button that
> sets the counter back to zero, or make the increment step configurable
> with a second binding.

## Running on different platforms

The same code runs on every supported platform:

```bash
# macOS
water run --platform macos

# iOS Simulator
water run --platform ios

# Android
water run --platform android

# Linux (GTK4)
water run --platform linux
```

Each platform renders the counter using its own native widgets. The buttons
look like iOS buttons on iOS, Material buttons on Android, and GTK4 buttons
on Linux. You did not write a single line of platform-specific code.

## Concepts recap

| Concept | What you learned |
|---------|------------------|
| `View` trait | The fundamental building block. Every UI element implements `View`. |
| `text()` / `text!` | Display text, with optional formatting and reactive interpolation. |
| `vstack()` / `hstack()` | Arrange views vertically or horizontally using tuple children. |
| `Binding::i32()` etc. | Create reactive state. Changes propagate to dependent views automatically. |
| `button()` | Create interactive buttons with `.state()` and `.action()` (extracted via `State<T>`). |
| `spacer()` | Flexible space that pushes views apart within stacks. |
| `App::new()` | Create the application entry point. |

## Next steps

Continue to [Project Structure and Water.toml](04-project-structure.md) to
see how WaterUI projects are organised, what goes in the `Water.toml`
manifest, and how assets and fonts are managed.

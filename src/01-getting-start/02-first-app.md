# Your First WaterUI App

Now that your development environment is set up, let's build your first interactive WaterUI application! We'll create a counter app that demonstrates the core concepts of views, state management, and user interaction.

## What We'll Build

Our counter app will feature:
- A display showing the current count
- Buttons to increment and decrement the counter
- Dynamic styling based on the counter value

By the end of this chapter, you'll understand:
- How to create interactive views
- How to manage reactive state
- How to handle user events
- How to compose views together

## Setting Up the Project

If you completed the setup chapter you already have a CLI-generated workspace. Otherwise scaffold one now:

```bash
water create "Counter App" \
  --bundle-id com.example.counterapp \
  --platform ios,android \
  --dev
cd counter-app
```

We will edit `src/lib.rs` so the shared code can run on any backend the CLI installed.

## Building the Counter Step by Step

Let's build our counter app incrementally, learning WaterUI concepts along the way.

### Step 1: Basic Structure

Start with a simple view structure. Since our initial view doesn't need state, we can use a function:

**Filename**: `src/lib.rs`
```rust
use waterui::prelude::*;

pub fn counter() -> impl View {
    text("Counter App")
}
```

Run this to make sure everything works:
```bash
water run --platform ios
```

You should see a window with "Counter App" displayed.

### Step 2: Adding Layout

Now let's add some layout structure using stacks:

```rust
use waterui::prelude::*;

pub fn counter() -> impl View {
    vstack((
        text("Counter App"),
        text("Count: 0"),
    ))
}
```

> **Note**: `vstack` creates a vertical stack of views. We'll learn about `hstack` (horizontal) and `zstack` (overlay) later.

### Step 3: Adding Reactive State

Now comes the exciting part - let's add reactive state! We'll use the `binding` helper from `waterui::prelude` and the `text!` macro for reactive text:

```rust
use waterui::prelude::*;

pub fn counter() -> impl View {
    let count: Binding<i32> = binding(0);
    
    vstack((
        text("Counter App"),
        text!("Count: {count}"),
        hstack((
            button("- Decrement").action_with(&count, |state: Binding<i32>| {
                state.set(state.get() - 1);
            }),
            button("+ Increment").action_with(&count, |state: Binding<i32>| {
                state.set(state.get() + 1);
            }),
        )),
    ))
}
```

`water run --platform ios` will hot reload changes—save the file and keep the terminal open to see updates instantly.

## Understanding the Code

Let's break down the key concepts introduced:

### Reactive State with `binding`

```rust
use waterui::prelude::*;

pub fn make_counter() -> Binding<i32> {
    binding(0)
}
```

This creates a reactive binding with an initial value of 0. When this value changes, any UI elements that depend on it will automatically update.

### Reactive Text Display

```rust
use waterui::prelude::*;

pub fn reactive_label() -> impl View {
    let count: Binding<i32> = binding(0);
    text!("Count: {count}")
}
```

- The `text!` macro automatically handles reactivity
- The text will update whenever `count` changes

### Event Handling

```rust
use waterui::prelude::*;

pub fn decrement_button() -> impl View {
    let count: Binding<i32> = binding(0);
    button("- Decrement").action_with(&count, |count: Binding<i32>| {
        count.set(count.get() - 1);
    })
}
```

- `.action_with()` attaches an event handler with captured state.
- `Binding<T>` implements `Clone` efficiently (it's a reference-counted handle), so you can pass it around.

### Layout with Stacks

```rust
use waterui::prelude::*;

pub fn stack_examples() -> impl View {
    vstack((
        text("First"),
        hstack((text("Left"), text("Right"))),
    ))
}
```

Stacks are the primary layout tools in WaterUI, allowing you to arrange views vertically or horizontally.
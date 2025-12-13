# Controls Overview

Buttons, toggles, sliders, text fields, and steppers live inside `waterui::component`.
They share the same handler ergonomics and reactive bindings you saw in earlier chapters. This
chapter walks through each control, explaining how to wire it to `Binding` values, style labels, and
compose them into larger workflows.

## Buttons

Buttons turn user intent into actions. WaterUI’s `button` helper mirrors the ergonomics of SwiftUI
while keeping the full power of Rust’s closures. This section explains how to build buttons, capture
state, coordinate with the environment, and structure handlers for complex flows.

## Anatomy of a Button

`button(label)` returns a `Button` view. The `label` can be any view—string literal, `Text`, or a
fully custom composition. Attach behaviour with `.action` or `.action_with`.

```rust
# use waterui::prelude::*;

fn simple_button() -> impl View {
    button("Click Me").action(|| {
        println!("Button was clicked!");
    })
}
```

Behind the scenes, WaterUI converts the closure into a `HandlerFn`. Handlers can access the
`Environment` or receive state via `.action_with`.

## Working with State

Buttons often mutate reactive state. Use `action_with` to borrow a binding without cloning it
manually.

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# use waterui::widget::condition::when;
# use waterui::Binding;

fn counter_button() -> impl View {
    let count: Binding<i32> = binding(0);

    vstack((
        text!("Count: {count}"),
        button("Increment").action_with(&count, |binding: Binding<i32>| {
            binding.set(binding.get() + 1);
        }),
    ))
}
```

`.action_with(&binding, handler)` clones the binding for you (bindings are cheap handles). Inside
the handler you can call any of the binding helpers—`set`, `set_from`, `with_mut`, etc.—to keep the
state reactive.

## Custom Labels and Composition

Because labels are just views, you can craft rich buttons with icons, nested stacks, or dynamic
content.

```rust
# use waterui::prelude::*;
# use waterui::layout::{padding::EdgeInsets, stack::hstack};

fn hero_button() -> impl View {
    button(
        hstack((
            text("🚀"),
            text("Launch")
                .size(18.0)
                .padding_with(EdgeInsets::new(0.0, 0.0, 0.0, 8.0)),
        ))
        .padding()
    )
    .action(|| println!("Initiating launch"))
}
```

You can nest buttons inside stacks, grids, navigation views, or conditionals—WaterUI treats them
like any other view.

## Guarding Actions

WaterUI does not currently ship a built-in `.disabled` modifier. Instead, guard inside the handler or
wrap the button in a conditional.

```rust
# use waterui::prelude::*;
# use waterui::widget::condition::when;
# use waterui::Computed;
fn guarded_submit(can_submit: Computed<bool>) -> impl View {
    when(can_submit.clone(), || {
        button("Submit").action(|| println!("Submitted"))
    })
    .or(|| text("Complete all fields to submit"))
}
```

For idempotent operations, simply return early:

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# use waterui::Binding;
pub fn pay_button() -> impl View {
    let payment_state: Binding<bool> = binding(false);
    button("Pay").action_with(&payment_state, |state: Binding<bool>| {
        if state.get() {
            return;
        }
        state.set(true);
    })
}
```

## Asynchronous Workflows

Handlers run on the UI thread. When you need async work, hand it off to a task:

```rust
# use waterui::prelude::*;
# use waterui::task::spawn;
pub fn refresh_button() -> impl View {
    button("Refresh").action(|| {
        spawn(async move {
            println!("Refreshing data…");
        });
    })
}
```

`spawn` hands the async work to the configured executor so the handler stays lightweight—schedule
work and return immediately.

## Best Practices

- **Keep handlers pure** – Avoid blocking IO or heavy computation directly in the closure.
- **Prefer `action_with`** – It guarantees the binding lives long enough and stays reactive.
- **Think environment-first** – Use extractors when a button needs shared services.
- **Make feedback visible** – Toggle UI state with bindings (loading spinners, success banners) so
  the user sees progress.

Buttons may look small, but they orchestrate the majority of user journeys. Combine them with the
layout and state tools covered elsewhere in this book to build polished, responsive workflows.

## Toggles

Toggles expose boolean bindings with a platform-native appearance.

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# use waterui::widget::condition::when;
# use waterui::Binding;
pub fn settings_toggle() -> impl View {
    let wifi_enabled: Binding<bool> = binding(true);

    vstack((
        toggle("Wi-Fi", &wifi_enabled),
        when(wifi_enabled.map(|on| on), || text("Connected to Home"))
            .or(|| text("Wi-Fi disabled")),
    ))
}
```

- Pass the label as any view (string, `Text`, etc.) along with a `Binding<bool>`.
- Bind directly to a `Binding<bool>`; if you need side effects, react in a separate handler using `when` or `task`.
- Combine with `when` to surface context (“Wi-Fi connected to Home” vs “Wi-Fi off”).

## Sliders

Sliders map a numeric range onto a drag gesture. Provide the inclusive range and the bound value.

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# use waterui::Binding;
pub fn volume_control() -> impl View {
    let volume: Binding<f64> = binding(0.4_f64);

    Slider::new(0.0..=1.0, &volume)
        .label(text("Volume"))
}
```
Tips:
- `.step(value)` snaps to increments.
- `.label(view)` attaches an inline view (e.g., `text("Volume")`).
- For discrete ranges (0-10), wrap the slider alongside a `Text::display(volume.map(...))`.

## Steppers

Steppers are ideal for precise numeric entry without a keyboard.

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# use waterui::Binding;
pub fn quantity_selector() -> impl View {
    let quantity: Binding<i32> = binding(1);

    stepper(&quantity)
        .step(1)
        .range(1..=10)
        .label(text("Quantity"))
}
```

- `.range(min..=max)` clamps the value.
- `.step(size)` controls increments/decrements.
- Because steppers operate on `Binding<i32>`/`Binding<i64>`, convert floats to integers before using.

## Text Fields

`TextField` binds to `Binding<String>` and exposes placeholder text plus secure-entry modes.

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# use waterui::Binding;

pub fn login_fields() -> impl View {
    let username: Binding<String> = binding(String::new());
    let password: Binding<String> = binding(String::new());

    vstack((
        vstack((
            text("Username"),
            TextField::new(&username).prompt(text("you@example.com")),
        )),
        vstack((
            text("Password"),
            TextField::new(&password).prompt(text("••••••")),
        )),
    ))
}
```

WaterUI automatically syncs user edits back into the binding. Combine `.on_submit(handler)` with the
button patterns above to run validation or send credentials. When you need structured forms, these
controls are exactly what the `#[form]` macro wires up behind the scenes.

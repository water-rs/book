# Nami - The Reactive Heart of WaterUI

Reactive state management is the core of any interactive WaterUI application. When your data changes, the UI should automatically update to reflect it. This chapter teaches you how to master WaterUI's reactive system, powered by the **nami** crate.

> All examples assume the following imports:
> ```rust
> use waterui::prelude::*;
> use waterui::reactive::binding;
> ```

## The `Signal` Trait: A Universal Language

Everything in nami's reactive system implements the `Signal` trait. It represents **any value that can be observed for changes**.

```rust
# use core::marker::PhantomData;
pub struct Context<T>(PhantomData<T>);

pub trait Signal: Clone + 'static {
    type Output;
    type Guard;

    // Get the current value of the signal
    fn get(&self) -> Self::Output;

    // Watch for changes (used internally by the UI)
    fn watch(&self, watcher: impl Fn(Context<Self::Output>) + 'static) -> Self::Guard;
}
```

A `Signal` is a reactive value that knows how to:
1. **Provide** its current value (`get()`).
2. **Notify** observers when it changes (`watch()`).

## Types of Signals

### 1. `Binding<T>`: Mutable, Two-Way State

A `Binding<T>` is the most common way to manage **mutable** reactive state. It holds a value that can be changed, and it will notify any part of the UI that depends on it.

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# use waterui::Binding;
// Create mutable reactive state with automatic type conversion
let counter: Binding<i32> = binding(0);
let name: Binding<String> = binding("Alice".to_string());

// Set new values, which triggers UI updates
counter.set(42);
name.set("Bob".to_string());
```

### 2. `Computed<T>`: Derived, Read-Only State

A `Computed<T>` is a signal that is **derived** from one or more other signals. It automatically updates its value when its dependencies change. You create computed signals using the methods from the `SignalExt` trait.

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# use waterui::SignalExt;
# use waterui::Binding;
let first_name: Binding<String> = binding("Alice".to_string());
let last_name: Binding<String> = binding("Smith".to_string());

// Create a computed signal that updates automatically
let full_name = first_name.zip(last_name).map(|(first, last)| {
    format!("{} {}", first, last)
});

// `full_name` will re-compute whenever `first_name` or `last_name` changes.
```

The `binding(value)` helper is re-exported from WaterUI, giving you a concise way to
initialize bindings with automatic `Into` conversions (e.g. `binding("hello")` -> `Binding<String>`).
Once you have a binding, reach for `Binding`'s convenience methods like `.increment()`,
`.toggle()`, or `.push()` to keep your state updates expressive and ergonomic.

#### When Type Inference Needs Help

Sometimes the compiler can't deduce the target type—especially when starting from `None`,
`Default::default()`, or other type-agnostic values. In those cases, add an explicit type
with the turbofish syntax:

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# #[derive(Clone)]
# struct User;
// Starts as None, so we spell out the final type.
let selected_user = binding::<Option<User>>(None);

// Empty collection with an explicit element type.
let log_messages = binding::<Vec<String>>(Vec::new());
```

The rest of the ergonomics (methods like `.set`, `.toggle`, `.push`) remain exactly the same.

### 3. Constants: Signals That Never Change

Even simple, non-changing values can be treated as signals. This allows you to use them seamlessly in a reactive context.

```rust
# use waterui::reactive::constant;
let fixed_name = constant("WaterUI"); // Never changes
let literal_string = "Hello World";   // Also a signal!
```

## The Golden Rule: Avoid `.get()` in UI Code

Calling `.get()` on a signal extracts a **static, one-time snapshot** of its value. When you do this, you break the reactive chain. The UI will be built with that snapshot and will **never update** when the original signal changes.

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# use waterui::Binding;
let name: Binding<String> = binding("Alice");

// ❌ WRONG: Using .get() breaks reactivity
let broken_message = format!("Hello, {}", name.get());
text(broken_message); // This will NEVER update when `name` changes!

// ✅ CORRECT: Pass the signal directly to keep the reactive chain intact
let reactive_message = s!("Hello, {name}");
text(reactive_message); // This updates automatically when `name` changes.
```

**When should you use `.get()`?** Only when you need to pass the value to a non-reactive system, such as:
- Logging or debugging.
- Sending the data over a network.
- Performing a one-off calculation outside the UI.

## Mastering `Binding<T>`: Your State Management Tool

`Binding<T>` is more than just a container. It provides a rich set of convenience methods to handle state updates ergonomically.

### Basic Updates: `.set()`

The simplest way to update a binding is with `.set()`.

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
let counter = binding(0);
counter.set(10); // The counter is now 10
```

### In-Place Updates: `.update()`

For complex types, `.update()` allows you to modify the value in-place without creating a new one. It takes a closure that receives a mutable reference to the value.

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# #[derive(Clone)]
# struct User { name: String, tags: Vec<&'static str> }
let user = binding(User { name: "Alice".to_string(), tags: vec![] });

// Modify the user in-place
user.with_mut(|user: &mut User| {
    user.name = "Alicia".to_string();
    user.tags.push("admin");
});
// The UI updates once, after the closure finishes.
```
This is more efficient than cloning the value, modifying it, and then calling `.set()`.

### Boolean Toggle: `.toggle()`

For boolean bindings, `.toggle()` is a convenient shortcut.

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# use waterui::Binding;
let is_visible: Binding<bool> = binding(false);
is_visible.toggle(); // is_visible is now true
```

### Mutable Access with a Guard: `.get_mut()`

For scoped, complex mutations, `.get_mut()` provides a guard. The binding is marked as changed only when the guard is dropped.

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
let data = binding::<Vec<i32>>(vec![1, 2, 3]);

// Get a mutable guard. The update is sent when `guard` goes out of scope.
let mut guard = data.get_mut();
guard.push(4);
guard.sort();
```

## The `s!` Macro: Reactive String Formatting

The `s!` macro is a powerful tool for creating reactive strings. It automatically captures signals from the local scope and creates a computed string that updates whenever any of the captured signals change.

Without `s!` (manual & verbose):

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# use waterui::SignalExt;
# use waterui::Binding;
let name: Binding<String> = binding("John");
let age: Binding<i32> = binding(30);

let message = name.zip(age).map(|(n, a)| {
    format!("{} is {} years old.", n, a)
});
```

With `s!` (concise & reactive):

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# use waterui::Binding;
let name: Binding<String> = binding("John");
let age: Binding<i32> = binding(30);

let message = s!("{} is {} years old.", name, age);
```

The `s!` macro also supports named arguments for even greater clarity:
```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# use waterui::Binding;
let name: Binding<String> = binding("John");
let age: Binding<i32> = binding(30);
let message = s!("{name} is {age} years old.");
```

## Feeding Signals Into Views

Signals become visible through views. WaterUI ships a few bridges:

- **Formatting macros** (`text!`, `s!`, `format_signal!`) capture bindings automatically.
- **Dynamic views** (`Dynamic::watch` or `waterui::views::watch`) subscribe to any `Signal` and rebuild their child when it changes.
- **Resolver-aware components** consume `impl Resolvable`, which often wrap signals resolved against the `Environment`.

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# use waterui::Signal;
# use waterui::Binding;
# use waterui_core::dynamic::watch;

pub fn temperature(reading: impl Signal<Output = i32>) -> impl View {
    watch(reading, |value| text!("Current: {value}°C"))
}

pub fn profile(name: &str) -> impl View {
    let name: Binding<String> = binding(name.to_string());
    text!("Hello, {name}!")
}
```

Whenever possible, pass signals directly into these helpers instead of calling `.get()`. That keeps the UI diffing fast and narrowly scoped to the widgets that truly care.

## Transforming Signals with `SignalExt`

The `SignalExt` trait provides a rich set of combinators for creating new computed signals.

- **`.map()`**: Transform the value of a signal.
- **`.zip()`**: Combine two signals into one.
- **`.filter()`**: Update only when a condition is met.
- **`.debounce()`**: Wait for a quiet period before propagating an update.
- **`.throttle()`**: Limit updates to a specific time interval.

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# use waterui::Binding;
# use waterui::SignalExt;
let query: Binding<String> = binding(String::new());

// Trim whitespace reactively so searches only fire when meaningful.
let trimmed_query = query.clone().map(|q| q.trim().to_string());

// A derived signal that performs a search when the query is not empty.
let search_results = trimmed_query.map(|q| {
    if q.is_empty() {
        vec![]
    } else {
        // perform_search(&q)
        vec!["Result 1".to_string()]
    }
});
```

## Watching Signals Manually

Occasionally you need to run imperative logic whenever a value changes—logging, analytics, or triggering a network request. Every signal exposes `watch` for that purpose:

```rust
# use waterui::prelude::*;
# use waterui::reactive::binding;
# use waterui::reactive::watcher::Context;
# use waterui::Binding;
let name: Binding<String> = binding("Alice");
let _guard = name.watch(|ctx: Context<String>| {
    println!("Name changed to {}", ctx.value());
    println!("Metadata: {:?}", ctx.metadata());
});
```

Keep the returned guard alive as long as you need the subscription. Dropping it unsubscribes automatically. Inside view code, WaterUI keeps these guards alive for you (e.g., `Dynamic::watch` stores the guard alongside the rendered view via `With` metadata), but it is useful to know how the primitive operates when you build custom integrations.

By mastering these fundamental concepts, you can build complex, efficient, and maintainable reactive UIs with WaterUI.

# Reactive state

> **In this chapter, you will:**
> - Learn how `Binding<T>` gives your views mutable, reactive state
> - Understand how `Computed<T>` and signal combinators derive new values from existing ones
> - Use macros like `s!` and `text!` for reactive string formatting and localization
> - Discover `List<T>` for reactive collections and `#[derive(Project)]` for struct decomposition
> - Master the "golden rule" of reactivity that prevents subtle bugs

Imagine a counter app. The user taps a button and the number on screen updates instantly -- no manual DOM manipulation, no message passing, no diffing algorithm. You change the data; the UI follows.

WaterUI delivers that through `waterui::reactive`, a fine-grained reactivity
system re-exported by the top-level `waterui` crate. It provides signals,
bindings, collections, and combinators so your views update automatically when
data changes. This chapter walks through every reactive primitive you will use
day to day.

## The `Signal` trait

At the foundation of WaterUI reactivity is the `Signal` trait:

```rust,ignore
pub trait Signal: Clone + 'static {
    type Output;
    type Guard;

    fn get(&self) -> Self::Output;
    fn watch(&self, watcher: impl Fn(Context<Self::Output>) + 'static) -> Self::Guard;
}
```

- **`get()`** returns the current value synchronously.
- **`watch()`** registers a callback that fires whenever the value changes. It returns a guard -- dropping the guard unsubscribes the watcher.
- **`Context<T>`** wraps the new value along with optional metadata (e.g., animation hints). Call `ctx.into_value()` to extract the raw value.

Every reactive type in `waterui::reactive` implements `Signal`. This uniform interface is what makes the combinator system work -- any signal can be mapped, zipped, filtered, or composed with any other signal.

## `Binding<T>`: mutable reactive state

`Binding<T>` is the primary mutable state container. It is readable as a `Signal` and writable. Think of it as a reactive variable: read the current value, write a new value, and watchers get notified automatically.

### Creating bindings

```rust,ignore
use waterui::prelude::*;
use waterui::Str;

// Typed constructors for primitives
let count = Binding::i32(0);
let ratio = Binding::f64(3.14);
let flag = Binding::bool(true);

// Container constructor for complex types
let name = Binding::container(String::from("Alice"));
let title = Binding::container(Str::from("Welcome"));

// Default value
let items: Binding<Vec<String>> = Binding::default();
```

Use the typed constructors (`Binding::i32`, `Binding::u32`, `Binding::i64`, `Binding::u64`, `Binding::isize`, `Binding::usize`, `Binding::f32`, `Binding::f64`, `Binding::bool`) for primitives, and `Binding::container(value)` for everything else (`String`, `Str`, `Vec<T>`, `Option<T>`, your own types).

### Reading values

```rust,ignore
use waterui::prelude::*;

let count = Binding::i32(10);
let current = count.get(); // 10
```

### Writing values

```rust,ignore
use waterui::prelude::*;

let count = Binding::i32(0);

// Direct set
count.set(42);

// Set with Into conversion
let name = Binding::container(String::from("Alice"));
name.set_from("Bob"); // &str -> String automatically

// Arithmetic operations on numeric bindings
count.add_assign(5);   // count += 5
count.sub_assign(2);   // count -= 2
count.mul_assign(3);   // count *= 3
count.div_assign(2);   // count /= 2
count.rem_assign(3);   // count %= 3

// Bitwise operations on integer bindings
count.bitand_assign(0xFF);
count.bitor_assign(0x10);
count.bitxor_assign(0x01);
count.shl_assign(2);
count.shr_assign(1);

// Append to a string-like or vec-like binding
let text = Binding::container(String::from("Hello"));
text.append(" World"); // "Hello World"
```

### Mutating In Place

For complex mutations, use `with_mut` or `get_mut`:

```rust,ignore
let items = Binding::container(vec!["a".to_string(), "b".to_string()]);

// with_mut -- preferred, avoids extra clone for Container bindings
items.with_mut(|vec| {
    vec.push("c".into());
    vec.sort();
});

// get_mut -- returns a guard that writes back on drop
*count.get_mut() += 10; // modify and auto-commit

// IMPORTANT: Do NOT bind get_mut() to `let _`
// let _ = count.get_mut(); // This keeps the guard alive until scope end!
// Instead, use the one-liner pattern above.
```

> **Warning:** Be careful with `get_mut()`. The returned guard writes the value back when it is dropped. If you accidentally bind it to a variable, the write-back is delayed until the variable goes out of scope, which can cause surprising behavior.

The `with_mut` method is more efficient for `Container`-backed bindings because it avoids an intermediate clone.

### take()

Extract the value and replace it with the default:

```rust,ignore
let name = Binding::container("hello".to_string());
let taken = name.take(); // taken == "hello", name is now ""
```

Now that you know how to read and write bindings, let's look at the specialized methods available for common types.

### Boolean Bindings

`Binding<bool>` has specialized methods that make working with toggles and flags ergonomic:

```rust,ignore
let dark_mode = Binding::bool(false);

dark_mode.toggle();     // false -> true
let light = dark_mode.reverse(); // Binding<bool> that is always the opposite

// Conditional selection
let theme = dark_mode.bidirectional_select("dark".to_string(), "light".to_string());
// theme.get() == "dark" when dark_mode is true

// Produce Option from bool
let username = dark_mode.then("admin".to_string());
// Some("admin") when true, None when false

// Logical NOT via operator
let enabled = !dark_mode; // same as dark_mode.reverse()
```

### Option Bindings

`Binding<Option<T>>` provides unwrapping helpers so you do not have to manually match on `Some`/`None`:

```rust,ignore
let maybe_name = Binding::container::<Option<String>>(None);

// Unwrap with default
let name = maybe_name.unwrap_or("Anonymous".to_string());
let name = maybe_name.unwrap_or_default();
let name = maybe_name.unwrap_or_else(|| generate_name());

// Check equality through Option
let is_alice = maybe_name.some_equal_to("Alice".to_string());
```

Setting a value on the unwrapped binding wraps it in `Some` automatically.

### Numeric Bindings

For `PartialOrd` types:

```rust,ignore
let volume = Binding::container(0.5f32);

// Only accept values in range (reject out-of-range sets)
let safe = volume.range(0.0..=1.0);

// Clamp values to range (out-of-range values clamped to bounds)
let clamped = volume.clamp(0.0..=1.0);
```

For `Signed` types:

```rust,ignore
let number = Binding::i32(10);

let sign = number.sign();      // Binding<bool>: true if non-negative
let neg = number.negate();     // Binding<i32>: always the negation
let neg2 = -number;            // operator syntax for negate()
```

> **Tip:** Use `.range()` for validation (silently rejects bad values) and `.clamp()` for correction (forces values into bounds). A volume slider, for example, would typically use `.clamp(0.0..=1.0)`.

### Bidirectional Mappings

Sometimes you need a derived binding that can be written to as well as read. `Binding::mapping` creates a two-way derived binding:

```rust,ignore
let celsius = Binding::f64(0.0);

let fahrenheit = Binding::mapping(
    &celsius,
    |c| c * 9.0 / 5.0 + 32.0,        // getter: celsius -> fahrenheit
    |binding, f| binding.set((f - 32.0) * 5.0 / 9.0), // setter: fahrenheit -> celsius
);

fahrenheit.set(212.0);
assert_eq!(celsius.get(), 100.0);
```

Try setting `fahrenheit` to `32.0` and check what `celsius.get()` returns.

### Filtering

Create a binding that rejects invalid values:

```rust,ignore
let age = Binding::i32(25);
let valid_age = age.filter(|&a| a >= 0 && a <= 150);
valid_age.set(-1); // silently ignored
assert_eq!(age.get(), 25); // unchanged
```

### Condition and Equality

```rust,ignore
let score = Binding::i32(85);

// Condition: arbitrary predicate -> Binding<bool>
let is_passing = score.condition(|&s| s >= 60);

// Equal to a specific value -> Binding<bool>
let is_perfect = score.equal_to(100);
```

## Computed\<T\>: Derived Read-Only State

While `Binding` is for state you *own and modify*, `Computed` is for values you *derive from other signals*. It is a type-erased, read-only signal that wraps any `Signal` implementation behind a `Box<dyn ...>`:

```rust,ignore
pub struct Computed<T>(Box<dyn ComputedImpl<Output = T>>);
```

Create computed values from other signals:

```rust,ignore
let count = Binding::i32(5);

// From a binding (zero-cost conversion)
let computed: Computed<i32> = count.computed();

// Constant computed (never changes)
let always_42 = Computed::constant(42);

// Default computed
let zero: Computed<i32> = Computed::default(); // wraps 0
```

`Computed<V: View>` also implements `View` directly -- it watches itself and dynamically re-renders whenever the inner view changes.

> **Note:** `Computed` is useful when you need to store a signal in a struct field or pass it across an API boundary where the concrete signal type would be inconvenient. In most cases, you can work with concrete signal types directly.

## SignalExt Combinators

The `SignalExt` trait is automatically available on all `Signal` types. It provides a rich set of combinators for deriving new signals -- similar to how iterator adapters work in Rust's standard library.

### Transforming: map

The most fundamental combinator. It creates a new signal whose value is derived from another:

```rust,ignore
let count = Binding::i32(5);
let doubled = count.map(|n| n * 2);
assert_eq!(doubled.get(), 10);

count.set(10);
assert_eq!(doubled.get(), 20);
```

### Combining: zip

When you need a value that depends on *two* signals, use `zip`:

```rust,ignore
let width = Binding::container(100.0f32);
let height = Binding::container(50.0f32);

let area = width.zip(&height).map(|(w, h)| w * h);
assert_eq!(area.get(), 5000.0);
```

`zip` creates a signal that emits whenever *either* input changes.

### Type Conversion: map_into

```rust,ignore
let count = Binding::i32(42);
let as_i64 = count.map_into::<i64>();
```

### Side Effects: inspect

```rust,ignore
let value = Binding::i32(0);
let inspected = value.inspect(|v| tracing::debug!("Value changed to {v}"));
```

`inspect` runs a side-effect function on each value but passes the original value through unchanged.

### Deduplication: distinct

```rust,ignore
let noisy = Binding::i32(5);
let quiet = noisy.distinct(); // only emits when value actually changes
```

> **Tip:** Use `distinct()` after expensive `map()` operations to avoid redundant downstream updates when the mapped result has not actually changed.

### Caching: cached

```rust,ignore
let expensive = count.map(|n| heavy_computation(n));
let cached_result = expensive.cached(); // memoizes the last value
```

### Type Erasure: computed

```rust,ignore
let signal = count.map(|n| n * 2);
let erased: Computed<i32> = signal.computed();
```

### Comparison Helpers

These produce boolean signals from numeric or comparable values:

```rust,ignore
let score = Binding::i32(85);

let is_100 = score.equal_to(100);          // Signal<Output = bool>
let is_high = score.condition(|s| *s > 90); // arbitrary predicate
let above_50 = score.gt(50);               // greater than
let below_50 = score.lt(50);               // less than
let at_least_60 = score.ge(60);            // greater or equal
let at_most_90 = score.le(90);             // less or equal
```

### Boolean Combinators

Combine boolean signals with familiar logical operations:

```rust,ignore
let logged_in = Binding::bool(true);
let is_admin = Binding::bool(false);

let not_logged = logged_in.not();
let can_edit = logged_in.and(&is_admin);
let can_view = logged_in.or(&is_admin);

// Conditional values
let badge = is_admin.then_some("Admin");    // Signal<Output = Option<&str>>
let role = is_admin.select("admin", "user"); // Signal<Output = &str>
```

### Numeric Combinators

```rust,ignore
let temp = Binding::i32(-5);

let abs_temp = temp.abs();          // 5
let neg_temp = temp.negate();       // 5
let is_pos = temp.is_positive();    // false
let is_neg = temp.is_negative();    // true
let is_zero = temp.is_zero();       // false
let sign = temp.sign();             // false (negative)
```

### Option Combinators

Work with `Signal<Output = Option<T>>` without unwrapping manually:

```rust,ignore
let maybe = Binding::container(Some(42i32));

let is_some = maybe.is_some();              // true
let is_none = maybe.is_none();              // false
let value = maybe.unwrap_or(0);             // 42
let value = maybe.unwrap_or_default();      // 42
let value = maybe.unwrap_or_else(|| 99);    // 42
let eq = maybe.some_equal_to(42);           // true

let nested = Binding::container(Some(Some(5i32)));
let flat = nested.flatten();                // Some(5)

let mapped = maybe.map_some(|n| n.to_string());  // Some("42")
let chained = maybe.and_then_some(|n| if n > 0 { Some(n) } else { None });
```

### Result Combinators

```rust,ignore
let result = Binding::container::<Result<i32, String>>(Ok(42));

let is_ok = result.is_ok();
let is_err = result.is_err();
let ok_val = result.ok();       // Signal<Output = Option<i32>>
let err_val = result.err();     // Signal<Output = Option<String>>
let safe = result.unwrap_or_result(0);
let mapped = result.map_ok(|n| n * 2);
let mapped_err = result.map_err(|e| format!("Error: {e}"));
```

### String Combinators

```rust,ignore
let text = Binding::container("hello world".to_string());

let empty = text.is_empty();             // false
let len = text.str_len();                // 11
let has_world = text.contains("world");  // true
```

### Timer Combinators

These require the `timer` feature and are essential for handling rapid user input:

```rust,ignore
use std::time::Duration;

let rapid_input = Binding::container(String::new());

// Only emit after 300ms of inactivity
let debounced = rapid_input.debounce(Duration::from_millis(300));

// Emit at most once per 100ms
let throttled = rapid_input.throttle(Duration::from_millis(100));
```

> **Tip:** Use `debounce` for search-as-you-type (wait until the user stops typing). Use `throttle` for scroll or resize handlers (limit update frequency).

## constant(): Static Signals

For values that never change but need to participate in the signal graph:

```rust,ignore
use waterui::reactive::constant;

let tax_rate = constant(0.08);
let price = Binding::f64(100.0);

let total = price.zip(&tax_rate).map(|(p, r)| p * (1.0 + r));
assert_eq!(total.get(), 108.0);
```

A `Constant<T>` implements `Signal` but its `watch()` is a no-op -- watchers are never notified because the value never changes. This makes it zero-overhead in the reactive graph.

## Lazy: Deferred Constants

For expensive constant computations that should only run on first access:

```rust,ignore
use waterui::reactive::constant::Lazy;

let config = Lazy::new(|| {
    // Expensive computation, runs only once
    load_config_from_disk()
});

// First call computes and caches; subsequent calls return cached value
let value = config.get();
```

## The s! Macro: Reactive String Formatting

Building formatted strings from multiple reactive values is a common need. The `s!` macro creates a signal that produces a formatted `String`, automatically capturing reactive variables from scope:

```rust,ignore
let name = Binding::container("Alice".to_string());
let age = Binding::i32(30);

// Named variable capture -- variables are found by name in scope
let greeting = s!("Hello {name}, you are {age} years old");
// greeting is a Signal<Output = String> that updates when name or age change

// Positional arguments
let msg = s!("Value: {}", count);
```

The macro supports up to 4 reactive variables. It automatically `zip`s and `map`s them, producing a signal that re-formats whenever any input changes.

**Rules**:
- Named placeholders like `{name}` are auto-captured from scope.
- Positional placeholders like `{}` require explicit arguments.
- You cannot mix named and positional placeholders in the same call.

> **Note:** `s!` produces a `Signal<Output = String>`. If you need a `Text` view, use `text!` instead.

## The text! Macro: Localized Reactive Text

The `text!` macro creates a localized `Text` view with full i18n support:

```rust,ignore
// Simple text -- looked up in i18n/*.toml files
text!("Hello, World!")

// With reactive placeholders
let name = Binding::container("Alice".to_string());
text!("Hello, {name}")

// Plural support -- {#count} marks the plural source
let count = Binding::i32(3);
text!("I have {#count} apple")
// English: "I have 3 apples" (other)
// English: "I have 1 apple" (one)

// Context disambiguation
text!("Right" @ "direction")  // different from text!("Right" @ "correct")

// Explicit binding
text!("Hello, {name}", name = get_current_user())
```

Translation files are TOML in the `i18n/` directory:

```toml
# i18n/en.toml
"Hello, World!" = "Hello, World!"
"I have {#count} apple" = { one = "I have {count} apple", other = "I have {count} apples" }

# i18n/zh.toml
"Hello, World!" = "你好，世界！"
"I have {#count} apple" = { other = "我有{count}个苹果" }
```

## The #[derive(Project)] Macro

When you have a `Binding<Struct>`, you often need to pass individual fields to different child views. The `Project` derive macro lets you decompose a struct binding into per-field bindings:

```rust,ignore
#[derive(Clone, Project)]
struct Person {
    name: String,
    age: u32,
}

let person = Binding::container(Person {
    name: "Alice".to_string(),
    age: 30,
});

// Decompose into individual field bindings
let projected: PersonProjected = person.project();
// projected.name: Binding<String>
// projected.age: Binding<u32>

// Changes propagate bidirectionally
projected.name.set_from("Bob");
projected.age.set(25);
assert_eq!(person.get().name, "Bob");
assert_eq!(person.get().age, 25);
```

The macro generates a `PersonProjected` struct with `Binding<T>` for each field. Each projected binding uses `Binding::mapping` internally, so changes in either direction are reflected.

Tuples also implement `Project` natively (up to 14 elements):

```rust,ignore
let pair = Binding::container((42i32, "hello".to_string()));
let (num, text) = pair.project();
num.set(100);
assert_eq!(pair.get().0, 100);
```

> **Tip:** `Project` is especially useful when you have a form that edits a struct. Project the struct into per-field bindings and pass each one to its corresponding input control.

## List\<T\>: Reactive Collections

For dynamic lists -- think todo items, chat messages, or search results -- `Binding<Vec<T>>` works but does not tell you *what* changed. `List<T>` is a reactive `Vec` that notifies watchers when its contents change, with fine-grained information about insertions, removals, and reorderings:

```rust,ignore
use waterui::reactive::collection::{Collection, List};

let items = List::new();

// Mutation methods
items.push("first".to_string());
items.push("second".to_string());
items.insert(1, "middle".to_string());
let removed = items.remove(0);  // returns "first"
let last = items.pop();         // returns Some("middle")
items.clear();
items.sort(); // for Ord types

// Reading
let snapshot: Vec<String> = items.snapshot(); // clone current contents
let len = items.len(); // via Collection trait

// Iteration (clones the list to avoid borrow conflicts)
for item in &items {
    // ...
}
```

`List<T>` implements the `Collection` trait, which supports range-based watching:

```rust,ignore
// Watch the entire collection
let all_items_guard = items.watch(.., |ctx| {
    let current = ctx.into_value();
    tracing::debug!("Items: {current:?}");
});

// Watch a specific range
let visible_items_guard = items.watch(1..4, |ctx| {
    tracing::debug!("Items 1..4: {:?}", ctx.into_value());
});
```

`List<T>` is reference-counted internally -- cloning a `List` creates a shared handle. Modifications through any handle notify all watchers.

### Using List with ForEach

To render a reactive list, use `ForEach`:

```rust,ignore
use waterui::views::ForEach;
use waterui::Identifiable;

#[derive(Clone)]
struct TodoItem {
    id: i32,
    title: String,
    completed: Binding<bool>,
}

impl Identifiable for TodoItem {
    type Id = i32;

    fn id(&self) -> Self::Id {
        self.id
    }
}

let todos: List<TodoItem> = List::new();
let list_view = ForEach::new(todos, |item| {
    hstack((
        text(item.title),
        Spacer,
        Toggle::new(&item.completed),
    ))
});
```

Each item must implement `Identifiable` so the framework can track insertions, removals, and reorderings efficiently.

> **Warning:** Do not use `Vec` with `Dynamic::watch` for lists that change frequently. You will lose all diffing benefits and re-render the entire list on every change. Use `List<T>` with `ForEach` instead.

## The BindingMailbox: Cross-Thread Access

Since `Binding<T>` is `!Send` (it uses `Rc` internally), you cannot send it across threads. If you need to update UI state from a background task -- say, after fetching data from a network -- the `BindingMailbox` provides an async interface:

```rust,ignore
let count = Binding::i32(0);
let mailbox = count.mailbox();

// Send from another task
async fn background_work(mailbox: BindingMailbox<i32>) {
    let current = mailbox.get().await;
    mailbox.set(current + 1).await;

    // Or send a mutation job
    mailbox.handle(|binding| {
        binding.add_assign(10);
    });
}
```

The mailbox spawns a local task that processes jobs sequentially on the UI thread.

## Watching Signals Manually

While most reactive updates happen automatically through the view system, you can watch signals manually for side effects like logging, analytics, or synchronizing with external systems:

```rust,ignore
let count = Binding::i32(0);

let guard = count.watch(|ctx| {
    let new_value = ctx.into_value();
    tracing::debug!("Count changed to {new_value}");
});

// IMPORTANT: The guard keeps the watcher alive.
// Dropping the guard unsubscribes the watcher.
// Use .retain(guard) to tie it to a view's lifecycle.
```

To keep a manual watcher alive for the lifetime of a view:

```rust,ignore
fn my_view(count: Binding<i32>) -> impl View {
    let guard = count.watch(|ctx| {
        tracing::debug!("Count: {}", ctx.into_value());
    });

    text!("Hello")
        .retain(guard) // guard lives as long as the view
}
```

## Feeding Signals into Views

There are several ways to connect reactive state to the UI. Let's look at each approach and when to use it.

### Dynamic::watch

Rebuild a view section whenever a signal changes:

```rust,ignore
let count = Binding::i32(0);

Dynamic::watch(count, |n| {
    text!("Count: {n}")
})
```

This is the most general approach -- the closure receives the raw value and returns any `View`.

### The text! and s! macros

For text content, the macros handle reactivity automatically:

```rust,ignore
let name = Binding::container("World".to_string());
text!("Hello, {name}") // updates when name changes
```

### Component-level reactivity

Many WaterUI components accept signals directly:

```rust,ignore
let is_on = Binding::bool(false);
Toggle::new(&is_on) // Toggle reads and writes the binding

let progress = Binding::f64(0.5);
Slider::new(&progress) // Slider binds to the value

let label = Binding::container("Click me".to_string());
Button::new(text!("{label}")).action(|| { /* action */ })
```

## The Golden Rule

> **Never call `.get()` in view body code to feed values into the UI.**

This is the single most important rule for working with WaterUI reactivity. When you call `.get()`, you take a snapshot of the current value. The UI will never update when the signal changes because no watcher was registered:

```rust,ignore
// BAD -- breaks reactivity
fn bad_view(count: Binding<i32>) -> impl View {
    let n = count.get(); // snapshot! never updates
    text!("Count: {n}")  // n is a plain i32, not a signal
}

// GOOD -- reactive
fn good_view(count: Binding<i32>) -> impl View {
    Dynamic::watch(count, |n| text!("Count: {n}"))
}

// GOOD -- text! captures the binding reactively by name
fn also_good(count: Binding<i32>) -> impl View {
    text!("Count: {count}")
}
```

> **Warning:** This is the number one source of "my UI is not updating" bugs. If your view is not reacting to state changes, check whether you are accidentally calling `.get()` in the view body.

Use `.get()` only in:
- Event handlers and callbacks (e.g., `on_tap(|| { let x = count.get(); ... })`)
- Watcher closures
- Async tasks
- Tests

## Combining Multiple Signals

Use `zip` and `map` to derive values from multiple signals without breaking reactivity:

```rust,ignore
let first_name = Binding::container("Alice".to_string());
let last_name = Binding::container("Smith".to_string());

// Combine two signals
let full_name = first_name.zip(&last_name)
    .map(|(f, l)| format!("{f} {l}"));

// Use in a view
Dynamic::watch(full_name, |name| text!("{name}"))
```

For more than two signals, chain `zip`:

```rust,ignore
let a = Binding::i32(1);
let b = Binding::i32(2);
let c = Binding::i32(3);

let sum = a.zip(&b).zip(&c)
    .map(|((a, b), c)| a + b + c);
```

> **Tip:** If you find yourself zipping more than three signals, consider whether they belong in a struct with `#[derive(Project)]`. It often leads to cleaner code.

## Summary Table

| Type | Readable | Writable | Use Case |
|------|----------|----------|----------|
| `Binding<T>` | Yes | Yes | Primary mutable state |
| `Computed<T>` | Yes | No | Type-erased derived value |
| `Constant<T>` | Yes | No | Static value in signal graph |
| `Lazy<F, T>` | Yes | No | Deferred constant computation |
| `Map<S, F, O>` | Yes | No | Transformed signal |
| `Zip<A, B>` | Yes | No | Combined signals |
| `Distinct<S>` | Yes | No | Deduplicated signal |
| `Cached<S>` | Yes | No | Memoized signal |
| `Debounce<S>` | Yes | No | Time-delayed signal |
| `Throttle<S>` | Yes | No | Rate-limited signal |
| `List<T>` | Yes | Yes | Reactive collection |

| Macro | Purpose |
|-------|---------|
| `s!("...")` | Reactive string formatting |
| `text!("...")` | Localized reactive text view |
| `#[derive(Project)]` | Decompose struct bindings into per-field bindings |

With reactive state under your belt, the next chapter introduces the **Environment** -- WaterUI's dependency injection system that lets you share configuration, themes, and services across your entire view tree.

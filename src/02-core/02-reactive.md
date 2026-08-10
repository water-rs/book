# Reactive state

> **In this chapter, you will:**
> - Use `Binding<T>` for mutable state and `Computed<T>` for derived state
> - Learn the golden rule that keeps your UI updating, and the `map`/`zip` combinators that replace `.get()`
> - Format reactive text with `s!` and `text!`
> - Render changing collections with `List<T>`, `ForEach`, and `#[derive(Identifiable)]`

You change the data; the UI follows. WaterUI does that with fine-grained signals: a change to one value updates exactly the labels, colors, and attributes that read it, without rebuilding the surrounding view tree.

The reactivity engine is re-exported as `waterui::reactive`, and its main types (`Binding`, `Computed`, `Signal`, `SignalExt`) are in the prelude.

## The `Signal` trait

```rust,ignore
pub trait Signal: Clone + 'static {
    type Output;
    type Guard;

    fn get(&self) -> Self::Output;
    fn watch(&self, watcher: impl Fn(Context<Self::Output>) + 'static) -> Self::Guard;
}
```

- `get()` returns the current value synchronously.
- `watch()` registers a callback and returns a guard. Dropping the guard unsubscribes.
- `Context<T>` carries the new value plus metadata such as an animation hint; `ctx.into_value()` unwraps it.

Everything reactive implements `Signal`, which is what makes the combinators below universal: any signal can be mapped, zipped, or combined with any other.

## `Binding<T>`: mutable state

`Binding<T>` is a signal you can also write to.

```rust,ignore
use waterui::prelude::*;
use waterui::Str;

// Typed constructors for primitives
let count = Binding::i32(0);
let ratio = Binding::f64(3.14);
let flag = Binding::bool(true);

// container() for everything else
let name = Binding::container(String::from("Alice"));
let title = Binding::container(Str::from("Welcome"));

// Default value
let items: Binding<Vec<String>> = Binding::default();
```

The typed constructors exist for `i32`, `i64`, `isize`, `u32`, `u64`, `usize`, `f32`, `f64`, and `bool`. Everything else -- `String`, `Str`, `Vec<T>`, `Option<T>`, your own types -- goes through `Binding::container(value)`. Note the type parameter belongs to `Binding`, not to `container`: write `Binding::<Option<String>>::container(None)`.

### Writing

```rust,ignore
let count = Binding::i32(0);

count.set(42);

// Preferred for in-place mutation: the guard writes back when it drops
*count.get_mut() += 1;

// Arithmetic and bitwise helpers
count.add_assign(5);
count.mul_assign(3);
count.bitor_assign(0x10);

// Into conversion at the call site
let name = Binding::container(String::from("Alice"));
name.set_from("Bob");

// Extend a string-like or vec-like binding
name.append(" Smith");
```

`*binding.get_mut() += 1` is the house idiom for mutating a binding in a handler. Keep it a one-liner: the guard commits the write and notifies watchers when it drops, so binding it to a variable delays the notification until the end of the scope.

For multi-step edits, `with_mut` is better -- it mutates the container in place and notifies once, without the intermediate clone:

```rust,ignore
let items = Binding::container(vec!["b".to_string(), "a".to_string()]);

items.with_mut(|vec| {
    vec.push("c".into());
    vec.sort();
});
```

`take()` moves the value out and leaves `T::default()` behind.

## The golden rule

> **Never call `.get()` on a signal inside a view body.**

`.get()` returns a plain value -- a snapshot with no subscription attached. The view renders once with that number and never hears about the next one. This is the single most common cause of "my UI is not updating".

```rust,ignore
// BAD: n is a plain i32, the label freezes at its initial value
fn bad(count: Binding<i32>) -> impl View {
    let n = count.get();
    text!("Count: {n}")
}

// GOOD: text! subscribes to the binding it names
fn good(count: Binding<i32>) -> impl View {
    text!("Count: {count}")
}

// GOOD: derive a new signal and hand it to a signal-aware input
fn also_good(count: Binding<i32>) -> impl View {
    let is_high = count.map(|n| n > 10);
    text!("Count: {count}").opacity(is_high.map(|high| if high { 1.0 } else { 0.5 }))
}
```

`.get()` is correct everywhere a view body is *not* running: event handlers, watcher closures, async tasks, and tests.

## `Computed<T>`: derived, read-only

`Binding` is state you own; `Computed<T>` is a type-erased read-only signal, useful when you need to store a signal in a struct field or accept one across an API boundary:

```rust,ignore
let count = Binding::i32(5);

let computed: Computed<i32> = count.computed();
let always_42 = Computed::constant(42);
let zero: Computed<i32> = Computed::default();
```

Most component inputs take `impl IntoComputed<T>` or `impl Signal<Output = T>`, so a `Binding`, a mapped signal, or a plain value all work without an explicit conversion.

## Deriving signals

`SignalExt` is implemented for every signal. Two combinators carry most of the weight.

**`map`** transforms one signal:

```rust,ignore
let count = Binding::i32(5);
let doubled = count.map(|n| n * 2);
assert_eq!(doubled.get(), 10);

count.set(10);
assert_eq!(doubled.get(), 20);
```

**`zip`** combines two, emitting whenever either changes:

```rust,ignore
let width = Binding::container(100.0f32);
let height = Binding::container(50.0f32);

let area = width.zip(&height).map(|(w, h)| w * h);
assert_eq!(area.get(), 5000.0);
```

Chain `zip` for more inputs -- `a.zip(&b).zip(&c).map(|((a, b), c)| ...)`. If you are past three, the values probably belong in a struct with `#[derive(Project)]`, covered below.

The rest of `SignalExt` is a shorthand layer over `map`, grouped by the output type you are working with:

| Group | Methods |
|-------|---------|
| Comparison | `equal_to`, `condition`, `gt`, `lt`, `ge`, `le` |
| Boolean | `not`, `and`, `or`, `then_some`, `select` |
| Numeric | `abs`, `negate`, `sign`, `is_positive`, `is_negative`, `is_zero` |
| `Option` | `is_some`, `is_none`, `unwrap_or`, `unwrap_or_else`, `unwrap_or_default`, `some_equal_to`, `flatten`, `map_some`, `and_then_some` |
| `Result` | `is_ok`, `is_err`, `ok`, `err`, `unwrap_or_result`, `map_ok`, `map_err` |
| String-like | `is_empty`, `str_len`, `contains` |
| Plumbing | `map_into`, `inspect`, `distinct`, `cached`, `computed`, `with` |
| Timing (`timer` feature, on by default) | `debounce`, `throttle` |

Two of those are worth calling out. `distinct()` suppresses emissions when the mapped value did not actually change -- put it after an expensive `map` so downstream work does not re-run. And `debounce(Duration)` waits for a pause in input, which is what a search-as-you-type field wants, while `throttle(Duration)` caps the update rate for scroll and resize handlers.

```rust,ignore
use std::time::Duration;

let query = Binding::container(String::new());
let debounced = query.debounce(Duration::from_millis(300));
```

### Binding-specific helpers

`Binding` adds helpers that stay *writable*, unlike the read-only `SignalExt` versions:

```rust,ignore
let dark_mode = Binding::bool(false);

dark_mode.toggle();
let light = dark_mode.reverse();      // Binding<bool>, always the opposite
let light2 = !dark_mode.clone();      // same thing via the Not operator
let theme = dark_mode.bidirectional_select("dark", "light");

let volume = Binding::container(0.5f32);
let checked = volume.range(0.0..=1.0);  // rejects out-of-range writes
let clamped = volume.clamp(0.0..=1.0);  // clamps out-of-range writes

let age = Binding::i32(25);
let valid = age.filter(|&a| (0..=150).contains(&a));
```

Use `range` for validation (bad writes are dropped) and `clamp` for correction (bad writes are pulled into bounds).

`Binding::mapping` builds a two-way derived binding when a simple helper is not enough:

```rust,ignore
let celsius = Binding::f64(0.0);

let fahrenheit = Binding::mapping(
    &celsius,
    |c| c * 9.0 / 5.0 + 32.0,
    |binding, f| binding.set((f - 32.0) * 5.0 / 9.0),
);

fahrenheit.set(212.0);
assert_eq!(celsius.get(), 100.0);
```

## Constants

`constant(value)` lifts a plain value into the signal graph. Its `watch()` is a no-op, so it costs nothing:

```rust,ignore
use waterui::reactive::constant;

let tax_rate = constant(0.08);
let price = Binding::f64(100.0);
let total = price.zip(&tax_rate).map(|(p, r)| p * (1.0 + r));
```

`Lazy::new(closure)` is the deferred version: the closure runs on first `get()` and the result is cached.

## `s!`: reactive string formatting

`s!` produces a signal of `String`, capturing reactive variables from scope by name:

```rust,ignore
let name = Binding::container("Alice".to_string());
let age = Binding::i32(30);

let greeting = s!("Hello {name}, you are {age} years old");
```

Named placeholders are captured automatically; positional `{}` placeholders need explicit arguments (`s!("Value: {}", count)`); mixing the two forms is a compile error. Either form supports at most four reactive inputs.

## `text!`: localized reactive text

`text!` builds a `Text` view and routes the string through the i18n catalog:

```rust,ignore
// Looked up in i18n/*.toml
text!("Hello, World!");

// Reactive placeholder captured from scope
let name = Binding::container("Alice".to_string());
text!("Hello, {name}");

// Plural: {#count} marks the plural source
let count = Binding::i32(3);
text!("I have {#count} apple");

// Context disambiguation
text!("Right" @ "direction");

// Explicit alias when the local name is not the slot name
text!("Hello, {name}", name = current_user());
```

Placeholder names are translation slot keys, which is why `text!` accepts identifiers and explicit aliases but not arbitrary expressions.

Translations are TOML files under `i18n/`:

```toml
# i18n/en.toml
"Hello, World!" = "Hello, World!"
"I have {#count} apple" = { one = "I have {count} apple", other = "I have {count} apples" }

# i18n/zh.toml
"Hello, World!" = "你好，世界！"
"I have {#count} apple" = { other = "我有{count}个苹果" }
```

Use `text()` for static strings and `text!` for anything reactive or localized.

## `#[derive(Project)]`

A `Binding<Struct>` is awkward to hand to child views one field at a time. `Project` decomposes it into per-field bindings that stay connected in both directions:

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

let projected: PersonProjected = person.project();
// projected.name: Binding<String>
// projected.age:  Binding<u32>

projected.name.set_from("Bob");
assert_eq!(person.get().name, "Bob");
```

The derive generates a `<Name>Projected` struct with one `Binding<T>` per field, each built on `Binding::mapping`. Tuples implement `Project` natively:

```rust,ignore
let pair = Binding::container((42i32, "hello".to_string()));
let (num, text) = pair.project();
num.set(100);
assert_eq!(pair.get().0, 100);
```

This is what makes form editing pleasant: project the model once, pass each field binding to its input control.

## Reactive collections

For a set of rows whose membership changes -- todos, chat messages, search results -- `Binding<Vec<T>>` is the wrong tool. It tells watchers *that* the vector changed but not *how*, so the whole list has to be rebuilt.

`List<T>` is a reactive vector that reports insertions, removals, and reorderings:

```rust,ignore
use waterui::prelude::*;
use waterui::reactive::collection::{Collection, List};

let items: List<String> = List::new();

items.push("first".to_string());
items.insert(1, "middle".to_string());
let removed = items.remove(0);
let last = items.pop();
items.sort();

// Swap the whole contents in one diffed update instead of N pushes
let previous = items.replace(vec!["a".into(), "b".into()]);

let snapshot: Vec<String> = items.snapshot();
let len = items.len();
```

`List<T>` is reference-counted: cloning gives you a second handle onto the same data, and writes through any handle notify every watcher. The `Collection` trait it implements also supports range-scoped watching (`items.watch(1..4, |ctx| ...)`), which is how virtualized backends observe only the visible window.

> **Note:** the prelude also exports a `List` -- the *list component* from `waterui::component::list`. An explicit `use waterui::reactive::collection::List;` shadows the glob import, so the two coexist, but keep the distinction in mind: one is data, the other is a view.

### Rendering with `ForEach`

```rust,ignore
use waterui::prelude::*;
use waterui::reactive::collection::List;
use waterui::views::ForEach;
use waterui::Identifiable;

#[derive(Clone, Identifiable)]
struct TodoItem {
    #[id]
    id: u64,
    title: Str,
    completed: Binding<bool>,
}

fn todo_list(todos: List<TodoItem>) -> impl View {
    ForEach::new(todos, |item| {
        hstack((
            text(item.title),
            Spacer::flexible(),
            toggle("Completed", &item.completed),
        ))
    })
}
```

Items must implement `Identifiable` so the framework can match rows across updates and move, insert, or remove only what changed. `#[derive(Identifiable)]` writes the impl for you: mark exactly one field with `#[id]`, and its type must be `Hash + Ord + Clone` (the generated `id()` clones it). The derive works on named fields, tuple fields, and generic id types; enums, unit structs, and zero or multiple `#[id]` markers are compile errors.

For a fixed, known set of children, do not reach for a collection at all -- pass an array or build a tuple stack:

```rust,ignore
vstack((header(), body(), footer()));
```

## Watching manually

Side effects that are not views -- logging, analytics, syncing to disk -- need an explicit watcher. The subscription lives exactly as long as its guard, and a view body's locals are dropped as soon as it returns, so tie the guard to the view with `.retain()`:

```rust,ignore
fn my_view(count: Binding<i32>) -> impl View {
    let guard = count.watch(|ctx| {
        tracing::debug!("Count: {}", ctx.into_value());
    });

    text!("Count: {count}").retain(guard)
}
```

Forgetting `.retain()` is a classic bug: the watcher unsubscribes immediately and the side effect silently never fires.

## Updating from another thread

`Binding<T>` uses `Rc` internally and is therefore `!Send`. To drive UI state from a background task, take a mailbox:

```rust,ignore
let count = Binding::i32(0);
let mailbox = count.mailbox();

async fn background_work(mailbox: BindingMailbox<i32>) {
    let current = mailbox.get().await;
    mailbox.set(current + 1).await;

    // Or enqueue a mutation without awaiting a reply
    mailbox.handle(|binding| binding.add_assign(10));
}
```

The mailbox owns a local task that applies queued jobs sequentially on the UI thread. `get_as::<T2>()` converts while it reads, which is how a `Binding<Str>` becomes an owned `String` on the other side of an `await`.

## When the view's *shape* changes

Everything above updates values in place. Occasionally the semantic structure itself has to change -- a loading screen becomes a detail screen. That is what `Dynamic::watch` is for, and it is a deliberate exception:

```rust,ignore
use waterui::prelude::*;

watch(phase, |phase| match phase {
    Phase::Loading => loading_screen().anyview(),
    Phase::Ready => detail_screen().anyview(),
})
```

`watch` **replaces the entire child subtree and discards its state**. Reaching for it to update a number, a color, or a list is the most expensive mistake you can make in a WaterUI view:

| You want to update | Use this, not `watch` |
|--------------------|------------------------|
| Text content | `text!("{status}")` |
| A view attribute | the signal-taking modifier, e.g. `photo.blur(amount)` |
| Collection membership | `ForEach::new(rows, row_view)` / `List` |

Next: [The Environment](03-environment.md), which shares themes, locales, and services across the view tree without threading parameters through every function.

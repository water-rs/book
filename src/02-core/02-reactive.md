# Reactive state

> **In this chapter, you will:**
> - Hold mutable state in `Binding<T>` and derived, read-only state in `Computed<T>`, and follow the golden rule that keeps the UI updating: never snapshot in a body
> - Derive signals with `map`, `zip` and the rest of `SignalExt`, and write through two-way helpers such as `toggle`, `clamp` and `Binding::mapping`
> - Format reactive text with `s!` and `text!`, and split a struct binding with `#[derive(Project)]`
> - Render changing collections with `List<T>` and `Lazy::for_each`, and know when `watch` and `Dynamic` are the escape hatch
> - Drive a binding from a background thread through its mailbox

You change the data; the UI follows. WaterUI does that with fine-grained signals: a change to one value updates exactly the labels, colors and attributes that read it, without rebuilding the surrounding view tree.

The engine is the [`nami`](https://crates.io/crates/nami) crate (0.13, on top of `nami-core` 0.5), re-exported as `waterui::reactive`. Its main types are in the prelude: `Binding`, `Computed`, `Signal` and `SignalExt`, plus the `binding` constructor function.

```rust
use waterui::prelude::*;

fn counter() -> impl View {
    let count = Binding::i32(0);

    vstack((
        text!("Count: {count}"),
        button("+1")
            .action(|State(count): State<Binding<i32>>| *count.get_mut() += 1)
            .state(&count),
    ))
}
```

The button's handler writes the binding, and the label, which names `count` inside `text!`, shows the new value on its own. The rest of this chapter is about the pieces of that loop: what a signal is, how to derive one from another, and how to keep a view subscribed.

## The `Signal` trait

Everything reactive implements `Signal`. These are the three methods you will meet:

```rust,ignore
pub trait Signal: Clone + 'static {
    type Output: 'static;
    type Guard: WatcherGuard;

    fn snapshot(&self) -> Self::Output;
    fn with_ref<R>(&self, f: impl FnOnce(&Self::Output) -> R) -> R;
    // identity() is omitted: backends use it, applications rarely do
    fn watch(&self, watcher: impl Fn(Context<Self::Output>) + 'static) -> Self::Guard;
}
```

- `snapshot()` returns the current value. It is a one-off, untracked read: the value is detached from reactivity and no dependency is recorded.
- `with_ref()` lends the current value to a closure. A container `Binding`, a constant, or a `Computed` wrapping one of them lends it without cloning; mapped and zipped signals fall back to a snapshot.
- `watch()` registers a callback and returns a guard. Dropping the guard unsubscribes. `Context<T>` carries the new value plus metadata such as an animation hint; `ctx.into_value()` unwraps it.

Because every signal shares this trait, the combinators below are universal: any signal can be mapped, zipped or combined with any other.

## `Binding<T>`: mutable state

`Binding<T>` is a signal you can also write to. Cloning one clones a handle, not the value, so a clone is cheap and every clone sees every write.

### Creating

```rust
use waterui::prelude::*;

// Typed constructors for primitives
let count = Binding::i32(0);
let ratio = Binding::f64(3.14);
let flag = Binding::bool(true);

// container() for everything else
let name = Binding::container(String::from("Alice"));
let title = Binding::container(Str::from("Welcome"));

// Default value, and an explicit type for an empty optional
let items: Binding<Vec<String>> = Binding::default();
let selection = Binding::<Option<String>>::container(None);

// binding() converts through Into, so a literal becomes a Str
let heading: Binding<Str> = binding("Inbox");
```

The typed constructors exist for `i32`, `i64`, `isize`, `u32`, `u64`, `usize`, `f32`, `f64` and `bool`. Everything else, such as `String`, `Str`, `Vec<T>`, `Option<T>` or your own types, goes through `Binding::container(value)`. The type parameter belongs to `Binding`, not to `container`: write `Binding::<Option<String>>::container(None)`.

`binding(value)` takes `impl Into<T>`, so Rust cannot infer `T` from the argument alone. It works when something else pins the type: a `let` annotation, a turbofish, or a control that takes `&Binding<Str>`.

### Writing

```rust
use waterui::prelude::*;

let count = Binding::i32(0);
count.set(42);

// In-place mutation: the guard writes back when it drops
*count.get_mut() += 1;

// Arithmetic helpers
count.add_assign(5);
count.mul_assign(3);
assert_eq!(count.snapshot(), 144);

// Conversions and extension
let name = Binding::container(String::from("Alice"));
name.set_from("Bob");
name.append(" Smith");
assert_eq!(name.snapshot(), "Bob Smith");
```

`*binding.get_mut() += 1` is the idiom for a one-line edit. Keep it a one-liner: the guard commits the write and notifies watchers when it drops, so binding it to a named variable delays the notification until the end of the scope. A guard that is only read, never written through, commits nothing and notifies no one.

For multi-step edits, `with_mut` mutates the container in place and notifies once:

```rust
use waterui::prelude::*;

let items = Binding::container(vec!["b".to_string(), "a".to_string()]);

items.with_mut(|list| {
    list.push("c".into());
    list.sort();
});

// with_ref lends the value without cloning it
let len = items.with_ref(|list| list.len());
assert_eq!(len, 3);

// take() moves the value out and leaves T::default() behind
let drained = items.take();
assert_eq!(drained.len(), 3);
```

`Binding::custom` wraps your own `CustomBinding` implementation (a `Signal` plus a `set`) when a binding has to write somewhere other than an in-memory container.

## The golden rule

> **Never snapshot in a body.**

`snapshot()` returns a plain value, a copy with no subscription attached. A view body that snapshots a signal renders once with that number and never hears about the next one. This is the most common cause of "my UI is not updating".

```rust
use waterui::prelude::*;

// BAD: n is a plain i32, so the label freezes at its initial value
fn frozen(count: Binding<i32>) -> impl View {
    let n = count.snapshot();
    text!("Count: {n}")
}

// GOOD: text! subscribes to the binding it names
fn live(count: Binding<i32>) -> impl View {
    text!("Count: {count}")
}

// GOOD: derive a new signal and hand it to a signal-aware modifier
fn dimmed(count: Binding<i32>) -> impl View {
    let is_high = count.gt(10);
    text!("Count: {count}").opacity(is_high.select(1.0_f32, 0.5))
}
```

`snapshot()` is correct wherever a view body is *not* running: event handlers, watcher closures, async tasks and tests. A handler runs outside the reactive graph, so reading a value there is exactly what you want.

## `Computed<T>`: derived, read-only

`Binding` is state you own; `Computed<T>` is a type-erased, read-only signal. Reach for it when you need to store a signal in a struct field or accept one across an API boundary:

```rust
use waterui::prelude::*;
use waterui::reactive::signal::IntoComputed;

let count = Binding::i32(5);

let computed: Computed<i32> = count.computed();
let always_42 = Computed::constant(42);
let zero: Computed<i32> = Computed::default();
assert_eq!(computed.snapshot(), 5);
assert_eq!(always_42.snapshot(), 42);
assert_eq!(zero.snapshot(), 0);

// A component input that accepts a plain value, a binding or any signal
struct Meter {
    level: Computed<f64>,
}

fn meter(level: impl IntoComputed<f64>) -> Meter {
    Meter {
        level: level.into_computed(),
    }
}

let fixed = meter(0.5);
let live = meter(Binding::f64(0.2));
assert_eq!(fixed.level.snapshot(), 0.5);
assert_eq!(live.level.snapshot(), 0.2);
```

Most component inputs take `impl IntoComputed<T>` or `impl Signal<Output = T>`, so a `Binding`, a mapped signal or a plain value all work without an explicit conversion. Two-way controls such as `toggle`, `slider` and `field` take `&Binding<T>` instead, because they write back.

## Deriving signals

`SignalExt` is implemented for every signal, and its methods take `&self`, so you never clone the source first. Two combinators carry most of the weight.

**`map`** transforms one signal:

```rust
use waterui::prelude::*;

let count = Binding::i32(5);
let doubled = count.map(|n| n * 2);
assert_eq!(doubled.snapshot(), 10);

count.set(10);
assert_eq!(doubled.snapshot(), 20);
```

**`zip`** combines two signals and emits whenever either changes:

```rust
use waterui::prelude::*;

let width = Binding::f32(100.0);
let height = Binding::f32(50.0);

let area = width.zip(&height).map(|(w, h)| w * h);
assert_eq!(area.snapshot(), 5000.0);

// Chaining left-associates, so the closure destructures nested pairs
let depth = Binding::f32(2.0);
let volume = width
    .zip(&height)
    .zip(&depth)
    .map(|((w, h), d)| w * h * d);
assert_eq!(volume.snapshot(), 10000.0);
```

Past three inputs the values probably belong in a struct with `#[derive(Project)]`, covered below. Call `.computed()` at the end when you need a nameable `Computed<T>` to store.

The rest of `SignalExt` is a shorthand layer over `map`, grouped by the output type you are working with:

| Group | Methods |
|-------|---------|
| Comparison | `equal_to`, `condition`, `gt`, `lt`, `ge`, `le` |
| Boolean | `not`, `and`, `or`, `then_some`, `select` |
| Numeric | `abs`, `negate`, `sign`, `is_positive`, `is_negative`, `is_zero` |
| `Option` | `is_some`, `is_none`, `unwrap_or`, `unwrap_or_else`, `unwrap_or_default`, `some_equal_to`, `flatten`, `map_some`, `and_then_some` |
| `Result` | `is_ok`, `is_err`, `ok`, `err`, `unwrap_or_result`, `unwrap_or_else_result`, `map_ok`, `map_err` |
| String-like | `str_is_empty`, `str_len`, `str_contains` |
| Plumbing | `map_into`, `inspect`, `distinct`, `cached`, `computed`, `with` |
| Timing | `debounce`, `throttle` |

The string methods carry a `str_` prefix so they do not shadow `Vec::is_empty` and `slice::contains` on the many types that can be a constant signal. `with(metadata)` attaches metadata such as an animation to a signal; see [Animation](../06-advanced/01-animation.md).

```rust
use waterui::prelude::*;

let query = Binding::container(String::new());
let is_blank = query.str_is_empty();
assert!(is_blank.snapshot());

query.set_from("rust");
assert!(!is_blank.snapshot());
assert_eq!(query.str_len().snapshot(), 4);

let age = Binding::i32(25);
let can_vote = age.ge(18);
let label = can_vote.select("adult", "minor");
assert_eq!(label.snapshot(), "adult");
```

Two of the plumbing and timing methods deserve a call-out. `distinct()` suppresses emissions when the value did not actually change, so put it after an expensive `map` and downstream work does not re-run. `debounce(Duration)` waits for a pause in input, which is what a search-as-you-type field wants, and `throttle(Duration)` caps the update rate for scroll and resize handlers. Both need the `timer` feature of `nami`, which is on by default.

```rust
use std::time::Duration;
use waterui::prelude::*;

// The field writes `query` on every keystroke; the search reads the debounced copy
fn settled_query(query: &Binding<String>) -> Computed<String> {
    query.debounce(Duration::from_millis(300)).computed()
}
```

### Constants

A plain value is already a constant signal wherever `impl IntoComputed<T>` is expected. `constant(value)` lifts a value into the signal graph explicitly, and its `watch()` is a no-op, so it costs nothing:

```rust
use waterui::prelude::*;
use waterui::reactive::constant;

let tax_rate = constant(0.08);
let price = Binding::f64(100.0);
let total = price.zip(&tax_rate).map(|(p, r)| p * (1.0 + r));
assert_eq!(total.snapshot(), 108.0);
```

nami pre-declares the numeric primitives, `bool`, `char`, `&'static str`, `String`, `Duration`, `Vec<T>`, `BTreeMap`, `BTreeSet`, arrays and `&'static [T]` as constant signals, and `Str` is one too. For your own `Clone` type, `impl_constant!` closes the gap; without it, passing a plain value of a custom enum where `impl IntoComputed<T>` is expected fails to compile:

```rust
use waterui::prelude::*;
use waterui::reactive::impl_constant;
use waterui::reactive::signal::IntoComputed;

#[derive(Clone)]
enum ChartMode {
    Bar,
    Line,
}

impl_constant!(ChartMode);

fn chart_mode(mode: impl IntoComputed<ChartMode>) -> Computed<ChartMode> {
    mode.into_computed()
}

// The plain value needs impl_constant!; the binding works either way
let fixed = chart_mode(ChartMode::Bar);
let live = chart_mode(Binding::container(ChartMode::Line));
assert!(matches!(fixed.snapshot(), ChartMode::Bar));
assert!(matches!(live.snapshot(), ChartMode::Line));
```

`waterui::reactive::constant::Lazy::new(closure)` is the deferred version: the closure runs on the first `snapshot()` and the result is cached. It is a different type from the `Lazy` layout helper in `waterui::component::lazy`.

## Two-way helpers on `Binding`

`Binding` adds helpers whose results stay *writable*, unlike the read-only `SignalExt` versions. Where an inherent method and a `SignalExt` method share a name (`equal_to`, `condition`, `then_some`, `unwrap_or`, `unwrap_or_else`, `unwrap_or_default`, `some_equal_to`, `negate`, `sign`), the `Binding` method wins and returns a `Binding`.

```rust
use waterui::prelude::*;

let dark_mode = Binding::bool(false);

dark_mode.toggle();
let light = dark_mode.reverse();       // Binding<bool>, always the opposite
let also_light = !dark_mode.clone();   // the same, through the Not operator
let theme = dark_mode.bidirectional_select("dark", "light");
assert!(!light.snapshot());
assert!(!also_light.snapshot());
assert_eq!(theme.snapshot(), "dark");

// Writing through the derived binding flows back to the source
theme.set("light");
assert!(!dark_mode.snapshot());

// Some(value) while true, None while false; setting Some or None writes the bool
let signed_in = Binding::bool(false);
let username = signed_in.then(Str::from("alice"));
assert!(username.snapshot().is_none());
```

Validation and correction are two different jobs:

```rust
use waterui::prelude::*;

let volume = Binding::f32(0.5);

// range rejects out-of-range writes
let checked = volume.range(0.0..=1.0);
checked.set(2.0);
assert_eq!(volume.snapshot(), 0.5);

// clamp pulls out-of-range writes into bounds
let clamped = volume.clamp(0.0..=1.0);
clamped.set(2.0);
assert_eq!(volume.snapshot(), 1.0);

// filter accepts any predicate
let age = Binding::i32(25);
let valid_age = age.filter(|&a| (0..=150).contains(&a));
valid_age.set(200);
assert_eq!(age.snapshot(), 25);

// condition and equal_to give a read-only Binding<bool>: writes to it are ignored
let is_adult = age.condition(|&a| a >= 18);
let is_twenty_five = age.equal_to(25);
assert!(is_adult.snapshot());
assert!(is_twenty_five.snapshot());

// unwrap_or turns Binding<Option<T>> into a writable Binding<T>
let nickname = Binding::<Option<Str>>::container(None);
let shown = nickname.unwrap_or(Str::from("anonymous"));
assert_eq!(shown.snapshot().as_str(), "anonymous");
shown.set(Str::from("Kai"));
assert!(nickname.snapshot().is_some());
```

`clamp` only clamps against inclusive bounds (`a..=b`, `a..`, `..=b`); the exclusive end of `a..b` is not enforced, so use `range` for that. `Binding::mapping` builds any other two-way derived binding. It is an associated function, and its setter receives the *source binding* to write back through, not a `&mut` slot:

```rust
use waterui::prelude::*;

let celsius = Binding::f64(0.0);

let fahrenheit = Binding::mapping(
    &celsius,
    |c| c * 9.0 / 5.0 + 32.0,
    |source, f| source.set((f - 32.0) * 5.0 / 9.0),
);

fahrenheit.set(212.0);
assert_eq!(celsius.snapshot(), 100.0);
```

## `s!`: reactive string formatting

`s!` produces a signal of `String`, capturing reactive variables from scope by name. It is not in the prelude; import it from `waterui::reactive`:

```rust
use waterui::prelude::*;
use waterui::reactive::s;

let name = Binding::container("Alice".to_string());
let age = Binding::i32(30);

// Named placeholders are captured from scope
let greeting = s!("Hello {name}, you are {age} years old");
assert_eq!(greeting.snapshot(), "Hello Alice, you are 30 years old");

// Positional placeholders take explicit arguments
let value = s!("Value: {}", age);
assert_eq!(value.snapshot(), "Value: 30");

// Named arguments rename a slot
let hello = s!("Hello {who}", who = &name);
assert_eq!(hello.snapshot(), "Hello Alice");

// Wrap the result to render it
let view = text(greeting.computed());
```

Mixing positional and named placeholders in one string is a compile error, and so is an argument the format string never uses.

## `text!`: localized reactive text

`text!` builds a `Text` view and routes the string through the i18n catalog. Placeholder names are translation slot keys, which is why `text!` accepts identifiers and explicit `name = expr` aliases but not arbitrary expressions:

```rust
use waterui::prelude::*;

fn labels(name: Binding<String>, count: Binding<i32>) -> impl View {
    vstack((
        // Looked up in i18n/*.toml, shown as-is when no entry exists
        text!("Hello, World!"),
        // A reactive placeholder captured from scope
        text!("Hello, {name}"),
        // An explicit alias when the local name is not the slot name
        text!("Hello, {who}", who = name.clone()),
        // Plural: {#count} marks the plural source
        text!("I have {#count} apple"),
        // Context disambiguation
        text!("Right" @ "direction"),
    ))
}
```

Translations are TOML files under `i18n/`:

```toml
# i18n/en.toml
"Hello, World!" = "Hello, World!"
"I have {#count} apple" = { one = "I have {count} apple", other = "I have {count} apples" }

# i18n/zh.toml
"Hello, World!" = "你好，世界！"
"I have {#count} apple" = { other = "我有{count}个苹果" }

# i18n/ja.toml
"Hello, World!" = "こんにちは、世界！"
"I have {#count} apple" = { other = "りんごが{count}個あります" }

# i18n/ko.toml
"Hello, World!" = "안녕하세요, 세계!"
"I have {#count} apple" = { other = "사과가 {count}개 있습니다" }
```

A placeholder can also carry a format spec, as in `{blur:.1}`. `text("...")` with a string literal is also looked up in the catalog, at run time, while `text(String)` and `text(Str)` show the value verbatim; reach for `text!` when the string has placeholders. The catalog format and locale resolution are in [Internationalization](../06-advanced/06-i18n.md).

## `#[derive(Project)]`

A `Binding<Struct>` is awkward to hand to child views one field at a time. `Project` decomposes it into per-field bindings that stay connected in both directions, each built on `Binding::mapping`:

```rust
use waterui::prelude::*;

#[derive(Clone, Project)]
struct Person {
    name: String,
    age: u32,
}

let person = Binding::container(Person {
    name: "Alice".to_string(),
    age: 30,
});

let projected = person.project(); // PersonProjected
projected.name.set_from("Bob");
projected.age.set(31);

assert_eq!(person.snapshot().name, "Bob");
assert_eq!(person.snapshot().age, 31);
```

For a struct with named fields, the derive generates a `<Name>Projected` struct with one public `Binding<T>` per field and the same visibility as your struct. A tuple struct projects to a tuple of bindings and a unit struct to `()`; enums and unions are rejected. Tuples of two to fourteen elements implement `Project` natively:

```rust
use waterui::prelude::*;

let pair = Binding::container((42i32, "hello".to_string()));
let (num, label) = pair.project();

num.set(100);
label.set_from("world");
assert_eq!(pair.snapshot().0, 100);
assert_eq!(pair.snapshot().1, "world");
```

Each field binding watches the whole source, so a write to one field also notifies the readers of the others, which re-read an unchanged value. Chain `.distinct()` on a field binding when its readers must skip those updates. Projecting is what makes form editing pleasant: project the model once and pass each field binding to its input control. `#[form]` derives `Project` for you; see [Forms](../03-ui/04-forms.md).

## Reactive collections

For a set of rows whose membership changes (todos, chat messages, search results), `Binding<Vec<T>>` is the wrong tool. It tells watchers *that* the vector changed but not *how*, so every row has to be rebuilt.

`List<T>` is a reactive vector that reports insertions, removals, moves and updates, aligned by item id. Its items implement `Identifiable`; `#[derive(Identifiable)]` writes the impl when you mark exactly one field with `#[id]`. The id type must be `Ord + Clone`. Import the trait explicitly, because the derive expands to an unqualified `Identifiable`.

```rust
use waterui::Identifiable;
use waterui::prelude::*;
use waterui::reactive::collection::List as ReactiveList;

#[derive(Clone, PartialEq, Identifiable)]
struct Message {
    #[id]
    id: u64,
    text: Str,
}

let message = |id: u64, text: &'static str| Message { id, text: Str::from(text) };

let mut messages: ReactiveList<Message> = ReactiveList::new();
messages.push(message(1, "first"));
messages.insert(0, message(0, "zeroth"));
messages.extend([message(2, "second"), message(3, "third")]);

let removed = messages.remove(0);
let last = messages.pop();
assert_eq!(removed.id, 0);
assert_eq!(last.map(|m| m.id), Some(3));

messages.update(0, |m| m.text = Str::from("edited"));
messages.sort_by_key(|m| std::cmp::Reverse(m.id));

// Swap the whole contents in one diffed update instead of N pushes
let previous = messages.replace(vec![message(9, "fresh"), message(1, "edited")]);
assert_eq!(previous.len(), 2);

let rows: Vec<Message> = messages.snapshot();
assert_eq!(rows.len(), 2);
assert_eq!(messages.len(), 2);
```

`List<T>` is reference-counted: cloning gives a second handle onto the same data, and a write through any handle notifies every watcher. The mutators take `&mut self`, so a handler mutates its own clone. `replace` aligns old and new items by id and needs `PartialEq`, so a surviving id whose item changed is reported as an updated row rather than a remove plus an insert. The `Collection` trait that `List` implements (`waterui::reactive::collection::Collection`) also supports range-scoped watching (`messages.watch(1..4, |ctx, change| ...)` with the trait in scope), which is how virtualized backends observe only the visible window.

> **Note:** the prelude also exports a `List`, the *list component* from `waterui::component::list`. Alias the data type on import (`List as ReactiveList`) so the two never collide: one is data, the other is a view.

### Rendering with `Lazy::for_each`

`ForEach` is a collection of views, not a view: a container has to consume it. `Lazy::for_each(data, generator)` is shorthand for `Lazy::vstack(ForEach::new(data, generator))` and builds a scrolling stack that realizes rows on demand; `Lazy::hstack` is the horizontal form. The list *component* has its own `List::for_each`, covered in [Lists](../03-ui/05-lists.md).

```rust
use waterui::Identifiable;
use waterui::component::lazy::Lazy;
use waterui::prelude::*;
use waterui::reactive::collection::List as ReactiveList;

#[derive(Clone, Identifiable)]
struct TodoItem {
    #[id]
    id: u64,
    title: Str,
    completed: Binding<bool>,
}

fn todo_list(todos: ReactiveList<TodoItem>) -> impl View {
    Lazy::for_each(todos, |item| {
        hstack((
            text(item.title),
            Spacer::flexible(),
            toggle("Completed", &item.completed),
        ))
    })
}
```

The framework matches rows across updates by id and moves, inserts or removes only what changed. Each row keeps its own `Binding<bool>`, so a toggle updates one row without touching the list.

### Derived row sets

A filtered, sorted or joined row set is not a `List`. Wrap any `Signal<Output = Vec<T>>` of `Identifiable + PartialEq` items in `SignalCollection`, which diffs each new snapshot against the previous one by id. This is the answer to "filter a list as the user types":

```rust
use waterui::Identifiable;
use waterui::component::lazy::Lazy;
use waterui::prelude::*;
use waterui::reactive::collection::SignalCollection;

#[derive(Clone, PartialEq, Identifiable)]
struct Mail {
    #[id]
    id: u64,
    subject: Str,
}

fn inbox(messages: Binding<Vec<Mail>>, query: Binding<Str>) -> impl View {
    let visible = SignalCollection::new(messages.zip(&query).map(|(all, q)| {
        all.into_iter()
            .filter(|m| m.subject.contains(q.as_str()))
            .collect::<Vec<_>>()
    }));

    Lazy::for_each(visible, |mail| text(mail.subject))
}
```

### Fixed sets

A fixed, known set needs no reactive collection. An array or `&'static [T]` works directly with `Lazy::for_each` and `ForEach::new`, and its rows are identified by index. To use a plain value as its own id, such as a tag string, wrap it in `SelfId` (`List<SelfId<String>>`); `UseId` and `.use_id(..)` derive the id from a function.

```rust
use waterui::component::lazy::Lazy;
use waterui::prelude::*;

let names = Lazy::for_each(["First", "Second"], |name| text(name));

// A fixed group of different children is just a tuple stack
let screen = vstack((text("Header"), text("Body"), text("Footer")));
```

## Reacting to changes

A side effect that is not a view (logging, analytics, syncing to disk) needs an explicit watcher. The subscription lives exactly as long as its guard, and a view body's locals are dropped when it returns, so tie the guard to the view with `.retain()`:

```rust
use waterui::log::debug;
use waterui::prelude::*;

fn my_view(count: Binding<i32>) -> impl View {
    let guard = count.watch(|ctx| {
        debug!("count is now {}", ctx.into_value());
    });

    text!("Count: {count}").retain(guard)
}
```

Forgetting `.retain()` is a classic bug: the watcher unsubscribes immediately and the side effect silently never fires. Diagnostics go through `tracing` (re-exported as `waterui::log`), never `println!`.

When a view only needs to react to a value, `.on_change(&signal, handler)` is shorter: the new value arrives first, by value, and any state extractors follow it:

```rust
use waterui::prelude::*;

fn mirrored(selection: Binding<i32>, last_seen: Binding<i32>) -> impl View {
    text!("{selection}")
        .on_change(
            &selection,
            |value: i32, State(last_seen): State<Binding<i32>>| {
                last_seen.set(value);
            },
        )
        .state(&last_seen)
}
```

## When the view's shape changes

Everything above updates values in place. Occasionally the semantic structure has to change: a loading screen becomes a detail screen. That is what `watch` is for (`Dynamic::watch` is the same function under an associated name), and it is a deliberate exception:

```rust
use waterui::prelude::*;

#[derive(Clone, PartialEq)]
enum Phase {
    Loading,
    Ready,
}

fn screen(phase: Binding<Phase>) -> impl View {
    watch(phase, |phase| match phase {
        Phase::Loading => text("Loading").anyview(),
        Phase::Ready => text("Ready").anyview(),
    })
}
```

The closure takes the value by move and must return one uniform type, so arms with different view types are erased with `.anyview()` or `AnyView::new(..)`.

`watch` **replaces the entire child subtree and discards its state**: focus, scroll position and anything owned below that point are lost. Reaching for it to update a number, a color or a list is the most expensive mistake you can make in a WaterUI view. Check these replacements first:

| You want to update | Use this, not `watch` |
|--------------------|------------------------|
| Text content | `text!("{status}")` |
| A view attribute | the signal-taking modifier, e.g. `photo.blur(amount)` |
| Collection membership | `Lazy::for_each(rows, row_view)` or `List::for_each` |
| A branch that is on or off | a conditional, see [Conditional views](../03-ui/06-conditional.md) |

The other legitimate form is a handler-driven swap through `Dynamic::new()`, which returns a `DynamicHandler` and the view to place in the tree. The handler is `#[state]`-marked, so `.state(&handler)` injects it and a handler parameter reads it bare:

```rust
use waterui::prelude::*;

fn swappable() -> impl View {
    let (handler, slot) = Dynamic::new();
    handler.set(text("Initial"));

    let swap = button("Swap")
        .action(|target: DynamicHandler| target.set(text("Swapped")))
        .state(&handler);

    vstack((swap, slot))
}
```

Even there, keep the reactive properties reactive: build the replacement with `.blur(blur.clone())`, not `.blur(blur.snapshot())`, so the slider keeps working without another swap.

## Updating from another thread

`Binding<T>` is `!Send` and `!Sync`. To drive UI state from a background thread or task, take a mailbox. It owns a local task that applies queued jobs one after another on the UI thread, and the mailbox handle itself can cross threads:

```rust
use waterui::prelude::*;
use waterui::task::spawn;

fn count_in_background(count: &Binding<i32>) {
    let mailbox = count.mailbox();

    spawn(async move {
        let current = mailbox.snapshot().await;
        mailbox.set(current + 1).await;

        // Enqueue a mutation without awaiting a reply
        mailbox.handle(|binding| binding.add_assign(10));
    })
    .detach();
}

fn blink_from_thread(flag: &Binding<bool>) {
    let mailbox = flag.mailbox();

    std::thread::spawn(move || {
        mailbox.handle(|binding| binding.toggle());
    });
}
```

- `handle(job)` enqueues a `Send` closure that receives `&mut Binding<T>` and returns at once.
- `snapshot()` and `set(value)` are async: they enqueue a job and await its reply. `snapshot_as::<T2>()` converts while it reads, which is how a `Binding<Str>` becomes an owned `String` on the other side of an `await`.
- The receiving task holds its own clone of the binding and runs until the mailbox is dropped, so the state stays alive while a worker holds the mailbox. `handle`, `snapshot` and `set` panic if that task no longer runs.
- Create the mailbox on the UI thread once the app is mounted, from a handler, `.task(..)` or `.on_appear(..)`, because it spawns its receiving loop on that thread's local executor. `mailbox_with_executor` takes an explicit one.

Anything CPU-bound belongs on a worker; deliver its result back through the mailbox rather than touching the binding directly.

Next: [The Environment](03-environment.md), which shares themes, locales and services across the view tree without threading parameters through every function.

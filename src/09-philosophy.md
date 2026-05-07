# Philosophy

> **In this chapter, you will:**
>
> - Understand the "why" behind WaterUI's core design decisions
> - See how native-first rendering, fine-grained reactivity, and Rust's type system work together
> - Explore the water metaphor that shaped the framework's identity
> - Learn the principles that guide WaterUI's evolution

Every framework is a set of bets. Bets about what matters most, about which trade-offs are worth making, and about what kind of developer experience will stand the test of time. WaterUI's bets are deliberate, and understanding them will help you use the framework effectively -- and decide whether it is the right tool for your project.

## Native-First

WaterUI renders to **platform-native widgets** wherever a real platform widget
exists. When you write `text("Hello")`, it becomes a `UILabel` on iOS and a
`TextView` on Android, and on Hydrolysis it becomes a GPU-rendered text node
that participates in the same accessibility tree the test runner asserts
against. Native-first is not a compromise -- it is the core design principle.

### Why Native?

- **Accessibility for free**: Screen readers, voice control, and switch access work
  because the platform already knows how to interact with its own widgets.
- **Platform conventions**: Text selection, context menus, drag-and-drop, and
  keyboard navigation behave exactly as users expect on each platform.
- **System integration**: Dynamic Type (iOS), Material You (Android), and GTK
  themes automatically apply without extra work from the developer.
- **Performance**: Native widgets are GPU-composited by the platform's own
  rendering pipeline, which has been optimized for decades.

### The Trade-off

Native rendering means pixel-perfect consistency across platforms is not the goal.
A WaterUI button will look like an iOS button on iOS and a Material button on
Android. If you need pixel-identical rendering everywhere -- or you are running
somewhere without a native widget set, like a Linux desktop or an embedded
target -- the Hydrolysis backend draws its own pixels on the GPU instead.

> **Note:** This is a philosophical stance, not just a technical one. WaterUI believes that respecting each platform's identity leads to better software than forcing a single look everywhere.

## Reactive, Not Virtual DOM

WaterUI uses **fine-grained reactivity** from the `nami` crate. There is no virtual
DOM, no tree diffing, and no reconciliation pass.

### How It Works

Each piece of mutable state is a `Binding<T>`. Derived values are `Computed<T>`.
When a binding changes, only the computed values that depend on it are recalculated,
and only the specific widget properties bound to those computed values are updated:

```text
Binding<i32> changes from 0 to 1
  |
  v
Computed<String> = "Count: 1"  (only this one recomputes)
  |
  v
UILabel.text = "Count: 1"     (only this property updates)
```

No other widgets are touched. No tree is walked. The update cost is O(number of
affected signals), not O(tree size).

### Why Not Virtual DOM?

Virtual DOM frameworks (React, Flutter) rebuild a lightweight tree on every state
change and diff it against the previous tree to find what changed. This has
advantages (simple mental model, works well for web) but also costs:

- **O(n) diffing**: Even if one property changed, the entire subtree must be
  diffed.
- **Allocation pressure**: Every render cycle creates a new tree of objects.
- **Identity problems**: The framework must heuristically determine which nodes
  "are the same" across renders (keys, indices).

WaterUI avoids all of these by connecting signals directly to widget properties.
The framework knows exactly what changed because the reactive graph tracks
dependencies at a granular level.

### Views Are Consumed, Not Retained

In WaterUI, `View::body(self, env)` takes `self` by value. The view struct is
moved and consumed when its body is evaluated. There is no persistent view tree
in memory -- the Rust view structs exist only during the initial walk. After that,
all state lives in `Binding<T>` values and native widgets.

This means:

- No stale view references.
- No memory leaks from retained view trees.
- No confusion about whether you are looking at "the current view" or "the
  previous render's view."

## Rust All The Way

Application logic, UI composition, state management, and even layout algorithms
are all written in Rust. The native backends are thin adapters that translate Rust
types into platform widgets.

### Why Rust?

- **Memory safety without GC**: No garbage collection pauses, no use-after-free,
  no data races. The borrow checker enforces safety at compile time.
- **Zero-cost abstractions**: Generics, traits, and closures compile to the same
  code you would write by hand. The `View` trait has no virtual dispatch overhead
  for statically known types.
- **Shared codebase**: Business logic, networking, data processing, and UI all
  live in the same language. No context switching between Kotlin and Swift.
- **`no_std` support**: The core and FFI crates work without the standard library,
  enabling embedded and WASM targets.

### The FFI Boundary

The price of "Rust all the way" is the FFI layer. Every interaction between Rust
and the native backend crosses a C ABI boundary (or JNI on Android). WaterUI
minimizes this cost by:

- Using 128-bit type IDs for O(1) view dispatch (no string comparisons).
- Transferring ownership of data across the boundary (no reference counting).
- Catching panics at the boundary to prevent undefined behavior.
- Generating FFI code automatically from macros and `cbindgen`.

## The Water Metaphor

The name "WaterUI" embodies a key idea: **like water, the framework adapts to its
container**.

- Water in a glass takes the shape of a glass. WaterUI on iOS takes the shape of
  iOS -- native widgets, platform conventions, system fonts.
- Water in a bottle takes the shape of a bottle. WaterUI on Android takes the
  shape of Android -- Material Design, system navigation, Compose interop.
- Water itself does not have a shape. The same view tree flows through Apple,
  Android, and Hydrolysis without changing what you wrote.

This philosophy extends to the API design:

- **Layout adapts**: `StretchAxis::MainAxis` means "expand along whatever axis the
  parent uses," not "expand horizontally."
- **Colors adapt**: `Color::foreground()` is not a specific hex value -- it resolves
  to the platform's foreground color, whether that is black, white, or a custom
  theme color.
- **Typography adapts**: `Font::body()` resolves to San Francisco on Apple, Roboto
  on Android, and the system font on Linux.

## Design Decisions

The following sections address questions that often arise: "Why did you choose X instead of Y?"

### Why Declarative?

WaterUI views are declarative: you describe *what* the UI should look like, not
*how* to build it. The `View` trait's `body()` method returns a description, and
the backend decides how to realize it.

This matches how modern UIs are designed. Designers create mockups that describe
the final state, not step-by-step instructions. Declarative code reads like
a design specification:

```rust,ignore
fn profile_card(user: &User) -> impl View {
    hstack((
        avatar(user.photo),
        vstack((
            text(user.name).font(Font::headline()),
            text(user.bio).foreground(Color::muted_foreground()),
        ))
    ))
    .padding(16.0)
    .background(Color::surface())
}
```

### Why Not an ECS?

Entity-Component-System architectures are popular in game engines, where you have
thousands of similar entities updated in tight loops. UI frameworks have different
needs:

- **Heterogeneous trees**: A form has text fields, buttons, sliders, and labels --
  all with different data shapes.
- **Sparse updates**: Usually only one or two widgets change at a time.
- **Deep nesting**: Navigation stacks, tab views, and modal sheets create deep
  hierarchies that ECS struggles with.

WaterUI's trait-based view system handles these patterns naturally.

### Why Signals Over Streams?

Reactive streams (Rx, async streams) represent sequences of events over time.
Signals represent the **current value** of a piece of state. For UI, signals are
a better fit because:

- A text field always has a current value, not a stream of values.
- A label always displays the current text, not a history of texts.
- When a new subscriber connects, it immediately gets the current value.

The `nami` crate provides `Binding<T>` (current value + change notifications) and
`Computed<T>` (derived value that updates automatically). These are synchronous,
glitch-free, and main-thread-safe -- exactly what UI state management needs.

## Comparison with Other Frameworks

WaterUI exists in a landscape of cross-platform frameworks. Here is how it
compares at a high level:

| Aspect           | WaterUI                                 | Flutter           | React Native        | Compose Multiplatform |
|------------------|------------------------------------------|-------------------|---------------------|------------------------|
| Language         | Rust                                     | Dart              | JavaScript          | Kotlin                 |
| Rendering        | Native widgets on Apple/Android; GPU (Hydrolysis) elsewhere | Custom (Skia)     | Native widgets      | Native + Skia          |
| State            | Signals                                  | setState/Riverpod | useState/Redux      | State/Flow             |
| Platforms        | iOS, Android, Linux/desktop via Hydrolysis | iOS, Android, Web, Desktop | iOS, Android | iOS, Android, Desktop  |
| Runtime overhead | No managed runtime                       | ~5MB runtime      | ~5MB + JSC          | ~2MB runtime           |

Each framework makes different trade-offs. WaterUI's niche is: **Rust developers
who want native performance, native look-and-feel, and a single codebase without
a managed runtime.**

## Principles for Contributors

If you are contributing to WaterUI, keep these principles in mind. They are not just guidelines -- they are the invariants that hold the framework together:

1. **Native first**: If a platform has a built-in widget for something, use it.
   Do not reimplement platform functionality unless there is a strong reason.

2. **Type safety over runtime checks**: Use Rust's type system to catch errors
   at compile time. Prefer generics over `dyn Any`.

3. **Composition over configuration**: Instead of adding flags to existing views,
   create new composable building blocks.

4. **No global state**: Pass context through `Environment`, not through static
   variables or singletons.

5. **Fail fast**: If something is wrong, panic with a clear message. Do not
   silently fall back to a default behavior that hides the bug.

6. **Less code is better**: Import a well-tested crate rather than writing your
   own implementation. Every line of code is a line that can break.

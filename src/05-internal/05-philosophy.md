# Philosophy

WaterUI’s design balances three goals:

1.  **Native-First** – We render through native toolkits (SwiftUI, Jetpack Compose) so apps look, feel, and behave like first-class citizens on every platform. We believe users can tell the difference, and platform conventions matter.
2.  **Reactive, Not Virtual DOM** – We use fine-grained reactivity (`nami`). Changes in state propagate directly to the specific properties or views that need updating, without diffing a virtual tree.
3.  **Rust all the way** – You write your UI logic, layout, and state management in Rust. The FFI layer handles the translation to the platform, but you stay in Rust.

## Why Native?

Drawing pixels (like Flutter or some game engines) gives you control, but you lose platform features for free: accessibility, text selection, native context menus, keyboard navigation, and OS-specific gestures. By wrapping native widgets, WaterUI ensures your app improves as the OS improves.

## Why Reactive?

Immediate mode GUIs are great for games, but retaining state and minimizing CPU usage is crucial for mobile apps. A reactive graph allows us to wake up only when necessary, saving battery and keeping the UI smooth.

## The "Water" Metaphor

Just as water takes the shape of its container, WaterUI apps adapt to the platform they run on. A `Toggle` becomes a `UISwitch` on iOS and a `Switch` on Android. A `NavigationStack` becomes a `UINavigationController` or a `NavHost`. You describe the *intent*, and WaterUI handles the *adaptation*.
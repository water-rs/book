#![allow(dead_code)]

/// Reusable snippets that back the code blocks in the book.
///
/// These functions are imported by the introduction and getting-started
/// chapters so the compiler enforces that the prose stays in sync with the
/// pinned `waterui` submodule. If a chapter modifies one of these snippets,
/// update the source here and the chapter together.
pub mod examples {
    use waterui::SignalExt;
    use waterui::State;
    use waterui::component::button;
    use waterui::layout::stack::{hstack, vstack};
    use waterui::prelude::ViewExt;
    use waterui::reactive::binding;
    use waterui::{Binding, View, text};

    /// A simple counter view showcasing bindings, derived signals, and the
    /// `State<T>` extractor pattern used by buttons.
    pub fn counter_view() -> impl View {
        let count: Binding<i32> = binding(0);
        let doubled = count.clone().map(|value| value * 2);

        vstack((
            text!("Count: {count}"),
            text!("Doubled: {doubled}"),
            hstack((
                button("Increment")
                    .action(|State(c): State<Binding<i32>>| c.set(c.get() + 1))
                    .state(&count),
                button("Reset")
                    .action(|State(c): State<Binding<i32>>| c.set(0))
                    .state(&count),
            )),
        ))
    }

    /// Formats a greeting using the `text!` macro and a reactive name binding.
    pub fn greeting(name: &str) -> impl View {
        let name: Binding<String> = binding(name.to_string());
        text!("Hello, {name}!")
    }
}

#[cfg(test)]
mod tests {
    use super::examples;
    use waterui::reactive::binding;

    #[test]
    fn binding_updates_propagate() {
        let counter = binding(0);
        counter.set(5);
        assert_eq!(counter.get(), 5);
    }

    #[test]
    fn greeting_is_signal_backed() {
        let view = examples::greeting("World");
        let env = waterui::env::Environment::new();
        let _ = waterui::View::body(view, &env);
    }
}

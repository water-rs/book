#![allow(dead_code)]

/// Reusable snippets that back the code blocks in the book.
pub mod examples {
    use waterui::SignalExt;
    use waterui::component::button;
    use waterui::layout::stack::{hstack, vstack};
    use waterui::reactive::binding;
    use waterui::{Binding, View};

    /// A simple counter view showcasing bindings, derived signals, and controls.
    pub fn counter_view() -> impl View {
        let count: Binding<i32> = binding(0);
        let doubled = count.clone().map(|value| value * 2);

        let increment_button = {
            let count = count.clone();
            button("Increment").action(move || count.set(count.get() + 1))
        };

        let reset_button = {
            let count = count.clone();
            button("Reset").action(move || count.set(0))
        };

        vstack((
            waterui::text!("Count: {count}"),
            waterui::text!("Doubled: {doubled}"),
            hstack((increment_button, reset_button)),
        ))
    }

    /// Formats a greeting using the `text!` macro and a string binding.
    pub fn greeting(name: &str) -> impl View {
        let name: Binding<String> = binding(name.to_string());
        waterui::text!("Hello, {name}!")
    }
}

#[cfg(test)]
mod tests {
    use super::examples;
    use waterui::Signal;
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

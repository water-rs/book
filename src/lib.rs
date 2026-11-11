#![allow(dead_code)]

/// Reusable snippets that back the code blocks in the book.
///
/// These functions mirror the counter and greeting the getting-started
/// chapters build, so the compiler enforces that the patterns they teach stay
/// in sync with the pinned `waterui` submodule. If a chapter changes one of
/// these patterns, update the source here and the chapter together.
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
        let doubled = count.map(|value| value * 2);

        vstack((
            text!("Count: {count}"),
            text!("Doubled: {doubled}"),
            hstack((
                button("Increment")
                    .action(|State(c): State<Binding<i32>>| *c.get_mut() += 1)
                    .state(&count),
                button("Reset")
                    .action(|State(c): State<Binding<i32>>| c.set(0))
                    .state(&count),
            )),
        ))
    }

    /// Formats a greeting using the `text!` macro and a reactive name binding.
    pub fn greeting(name: &str) -> impl View + use<> {
        let name: Binding<String> = binding(name.to_string());
        text!("Hello, {name}!")
    }
}

#[cfg(test)]
mod tests {
    use super::examples;
    use waterui::{Binding, Signal, View};

    #[test]
    fn binding_updates_propagate() {
        let counter = Binding::i32(0);
        counter.set(5);
        assert_eq!(counter.snapshot(), 5);
    }

    fn greeting_view() -> impl View {
        examples::greeting("World")
    }

    #[waterui::test(greeting_view)]
    fn greeting_renders_reactive_text(app: &mut waterui_testing::SemanticApp) {
        app.query()
            .role(waterui_testing::Role::LABEL)
            .label("Hello, World!")
            .assert_exists();
    }
}

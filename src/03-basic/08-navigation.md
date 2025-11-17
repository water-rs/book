# Navigation

`waterui_navigation` provides lightweight primitives for stacks, bars, and links that map to native
navigation controllers on each backend. This chapter covers the building blocks so you can push and
pop screens declaratively.

## NavigationView

Wrap your content in `NavigationView` (or the `navigation` helper) to display a persistent bar with
title, actions, and colour configuration.

```rust
use waterui::prelude::*;
use waterui_navigation::{Bar, NavigationView};
use waterui::reactive::binding;

pub fn inbox() -> impl View {
    let unread = binding(42);

    NavigationView {
        bar: Bar {
            title: text!("Inbox ({unread})"),
            color: constant(Color::srgb(16, 132, 255)),
            hidden: constant(false),
        },
        content: vstack((
            text("Recent messages"),
            scroll(list_of_threads()),
        ))
        .anyview(),
    }
}
```

Shorter alternative:

```rust
use waterui_navigation::navigation;

pub fn settings() -> impl View {
    navigation("Settings", settings_list())
}
```

## NavigationLink

Links describe push-style transitions. Provide a label view and a closure that builds the
destination.

```rust
use waterui::prelude::*;
use waterui_navigation::NavigationLink;

fn thread_row(thread: Thread) -> impl View {
    NavigationLink::new(
        text!("{thread.subject}"),
        move || navigation(thread.subject.clone(), thread_detail(thread.clone())),
    )
}
```

Backends render platform-specific affordances (chevrons on Apple platforms, row highlighting on GTK
and Web). Because the destination builder is a closure, WaterUI only instantiates the view after the
link is activated.

## Programmatic Navigation

For complex flows use a binding that tracks the active route:

```rust
use waterui::reactive::binding;
use waterui_navigation::{navigation, NavigationPath, NavigationStack};

#[derive(Clone)]
enum Step {
    Welcome,
    Address,
    Summary,
}

pub fn wizard() -> impl View {
    let path = binding(vec![Step::Welcome]);
    let nav_path = path.map(NavigationPath::from);

    NavigationStack::with(nav_path, navigation("Wizard", welcome_screen()))
        .destination(|step| match step {
            Step::Welcome => navigation("Welcome", welcome_screen()),
            Step::Address => navigation("Address", address_screen()),
            Step::Summary => navigation("Summary", summary_screen()),
        })
}
```

Updating the vector (push/pop) automatically syncs with the rendered stack.

## Tabs (Experimental)

The `waterui_navigation::tab` module exposes a minimal tab bar API:

```rust
use waterui_navigation::tab::{Tab, Tabs};

pub fn home_tabs() -> impl View {
    Tabs::new(vec![
        Tab::new("Home", || home_screen()),
        Tab::new("Discover", || discover_screen()),
        Tab::new("Profile", || profile_screen()),
    ])
}
```

The API is still stabilising; expect future versions to add badges, per-tab navigation stacks, and
lazy loading hooks.

## Best Practices

- Keep destination builders side-effect free; perform work in handlers and let bindings drive
  navigation changes.
- Use environment values (`env::use_env`) to provide routers or analytics so every link can report
  navigation events centrally.
- Combine navigation with the animation chapter to customize transitions once backends expose those
  hooks.

With these primitives you can express full navigation stacks declaratively without reaching for
imperative routers.

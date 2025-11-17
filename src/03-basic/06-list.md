# Lists and Collections

Dynamic data deserves a declarative list view. WaterUI ships a `List` component plus helpers such as
`ForEach`, `ListItem`, and `NavigationPath` so you can render changing collections with minimal
boilerplate.

## Building a List from a Collection

`List::for_each` wires nami collections into reusable rows. It accepts any `Collection`
(`reactive::collection::List`, plain `Vec`, arrays, etc.) as long as each item exposes a stable
identifier via `Identifable`. Reach for `waterui::reactive::collection::List` when you need runtime
mutations (push, remove, sort) that notify the UI automatically.

```rust
# use waterui::prelude::*;
# use waterui::component::list::{List, ListItem};
# use waterui::reactive::collection::List as ReactiveList;
# use waterui_core::id::Identifable;
# use waterui::AnyView;

#[derive(Clone)]
struct Thread {
    id: i32,
    subject: String,
}

impl Identifable for Thread {
    type Id = i32;

    fn id(&self) -> Self::Id {
        self.id
    }
}

pub fn inbox() -> impl View {
    let threads = ReactiveList::from(vec![
        Thread { id: 1, subject: "Welcome".into() },
        Thread { id: 2, subject: "WaterUI tips".into() },
    ]);

    List::for_each(threads.clone(), |thread| {
        let subject = thread.subject.clone();
        ListItem {
            content: AnyView::new(text!("{subject}")),
            on_delete: None,
        }
    })
}
```

Whenever the `threads` binding changes (insert, delete, reorder), the list diffs the identifiers and
only updates the affected rows.

> **Tip:** If your data type lacks a natural identifier, wrap it in a struct that implements
> `Identifable` using a generated `Id`.

## Handling Deletes

`ListItem` exposes `.on_delete` so you can react to destructive actions from the UI (swipe-to-delete
on Apple platforms, context menus on desktop backends).

```rust
# use waterui::prelude::*;
# use waterui::AnyView;
# use waterui::component::list::ListItem;
# use waterui::reactive::collection::List as ReactiveList;
# use waterui_core::id::Identifable;
# #[derive(Clone)]
# struct Thread {
#     id: i32,
#     subject: String,
# }
# impl Identifable for Thread {
#     type Id = i32;
#     fn id(&self) -> Self::Id {
#         self.id
#     }
# }

fn row(thread: Thread, threads: ReactiveList<Thread>) -> ListItem {
    let list = threads.clone();
    let subject = thread.subject.clone();
    ListItem {
        content: AnyView::new(text!("{subject}")),
        on_delete: Some(Box::new(move |_, _| {
            if let Some(index) = list.iter().position(|t| t.id == thread.id) {
                list.remove(index);
            }
        })),
    }
}
```

Set `disable_delete()` to opt out globally for a given row.

## Incremental Rendering with `ForEach`

`ForEach` is the engine behind `List::for_each`, but you can also use it directly whenever you need
to render dynamic tuples without the rest of the list machinery. Wrap primitive types with
`id::SelfId` (or implement `Identifable`) so each item has a stable key:

```rust
# use waterui::prelude::*;
# use waterui::reactive::collection::List as ReactiveList;
# use waterui::views::ForEach;
# use waterui_core::id;
# use waterui::layout::stack::VStack;

fn chip_row(tags: ReactiveList<id::SelfId<String>>) -> impl View {
    VStack::for_each(tags, |tag| {
        let label = tag.clone().into_inner();
        text!("#{label}")
    })
}
```

Inside layout containers (`hstack`, `vstack`) you still get diffing and stable identity, which keeps
animations and focus handling consistent.

## Virtualisation and Performance

On native backends, `List` feeds identifiers into platform list views (SwiftUI `List`, GTK4 list
widgets, DOM diffing on Web). That means virtualization and recycling are handled for you. Keep row
construction pure and cheap; expensive work should happen in signals upstream.

## Troubleshooting

- **Rows flicker or reorder unexpectedly** – Ensure `Identifable::id()` stays stable across renders.
- **Deletes trigger twice** – Some backends emit multiple delete actions for the same row to confirm
  removal. Guard inside the handler by verifying the item still exists before mutating state.
- **Nested scroll views** – Wrap lists inside `NavigationView` or `scroll` instead of stacking
  multiple scroll surfaces unless the platform explicitly supports nested scrolling.

Lists round out the standard component set: combine them with buttons, navigation links, and form
controls to build data-driven experiences that stay reactive as your collections grow.

# Lists

WaterUI ships a `List` component for rendering collections with platform-native chrome. The
component wraps a `Views` collection and forwards it through the environment so backends can diff and
virtualize large datasets efficiently.

## Static content

When you only need a handful of rows, build `ListItem` values manually and feed them into
`List::new`.

```rust,ignore
use waterui::component::list::{List, ListItem};
use waterui::prelude::*;

fn shortcuts() -> impl View {
    List::new(tuple!(
        ListItem {
            content: text!("Open recent"),
            on_delete: None,
        },
        ListItem {
            content: text!("Switch workspace"),
            on_delete: None,
        },
        ListItem {
            content: text!("Start recording"),
            on_delete: None,
        },
    ))
}
```

`ListItem` is a view, so you can use any layout primitives inside `content`. Backends receive a
sequence of typed rows and are free to translate them to `UITableView` cells, `RecyclerView`
entries, or virtualized list rows depending on platform conventions.

## Dynamic collections with `ForEach`

For dynamic data, feed `List` with a `ForEach` collection. Anything implementing
`nami::collection::Collection` works as long as each element can be uniquely identified. The
`IdentifableExt::use_id` helper tags plain data with a stable identifier so WaterUI can diff rows
without rebuilding the whole list.

```rust,ignore
use waterui::component::list::{List, ListItem};
use waterui::prelude::*;
use waterui::views::ForEach;
use nami::collection::List as ReactiveList;

#[derive(Clone)]
struct Document {
    id: u64,
    title: Str,
}

fn recent_documents() -> impl View {
    let docs = ReactiveList::from(vec![
        Document { id: 1, title: "Project brief".into() }.use_id(|doc| doc.id),
        Document { id: 2, title: "Experiment notes".into() }.use_id(|doc| doc.id),
        Document { id: 3, title: "Quarterly review".into() }.use_id(|doc| doc.id),
    ]);

    let rows = ForEach::new(docs.clone(), |doc| {
        ListItem {
            content: hstack((
                text!("{}", doc.title),
                spacer(),
                text!("#{:?}", doc.id()),
            )),
            on_delete: None,
        }
    });

    List::new(rows)
}
```

`ReactiveList` implements `Collection`, so WaterUI automatically registers watchers and only notifies
the backend about inserts, moves, or removals. This keeps large lists smooth even under frequent
reactive updates.

## Handling destructive actions

`ListItem::on_delete` lets the backend expose native affordances such as swipe-to-delete. The
closure receives the current `Environment` alongside the row index, making it easy to inject
services with `Use<T>`.

```rust,ignore
use waterui::component::list::{List, ListItem};
use waterui::prelude::*;
use waterui::views::ForEach;
use waterui_core::extract::{Extractor, Use};
use nami::collection::List as ReactiveList;

#[derive(Clone)]
struct Analytics;

impl Analytics {
    fn track(&self, message: &str) {
        println!("{message}");
    }
}

fn reminders() -> impl View {
    let reminders = ReactiveList::from(vec![
        "Walk the dog".to_string(),
        "Call the studio".to_string(),
        "Prepare slides".to_string(),
    ]);

    let rows = ForEach::new(reminders.clone(), move |title| {
        ListItem {
            content: text!("{}", title.clone()),
            on_delete: Some(Box::new({
                let reminders = reminders.clone();
                move |env, index| {
                    reminders.remove(index);
                    let Use(analytics) = Extractor::extract::<Use<Analytics>>(env)
                        .expect("Analytics must be installed");
                    analytics.track(&format!("Deleted reminder: {title}"));
                }
            })),
        }
    });

    List::new(rows).with(Analytics)
}
```

Backends that support swipe actions call the `on_delete` handler when a row is removed. If your UI
handles deletion explicitly (for example via a button), invoke the handler yourself to keep
analytics and other observers in sync.

## Customizing list presentation

Lists participate in WaterUI's hook system. Installing a `Hook<ListConfig>` lets you translate list
metadata into native styling or inject separators.

```rust,ignore
use waterui::component::list::{List, ListConfig, ListItem};
use waterui::prelude::*;
use waterui::view::Hook;

fn rounded_rows(env: &Environment, config: ListConfig) -> impl View {
    vstack(config.contents.into_iter().enumerate().map(|(index, item)| {
        item.content
            .padding(12.0)
            .background(style::Background::color(Color::from_rgb(0.12, 0.12, 0.14)))
            .corner_radius(12.0)
            .padding((0.0, if index == 0 { 0.0 } else { 8.0 }))
            .metadata(style::Shadow::elevated())
    }))
}

fn custom_list() -> impl View {
    List::new(tuple!(ListItem {
        content: text!("Custom row"),
        on_delete: None,
    }))
    .with(Hook::new(rounded_rows))
}
```

Hooks run in the same environment as the list, so you can pull colors, metrics, or platform
services from `Environment` while translating metadata into the native representation your backend
expects.

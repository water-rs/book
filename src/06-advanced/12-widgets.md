# Home-screen widgets

> **In this chapter, you will:**
> - Declare a home-screen widget with `#[waterui::widget]` and read its family and size from a `WidgetContext`
> - Build widget content from the closed `waterui::widgets` constructors, and see why a `View` there is a compile error
> - Return a `Timeline` with dated entries and a reload policy
> - Make a tappable region with `link` and with `#[waterui::widget_intent]`
> - Know which platforms the widget feature supports and when `water` generates the extension

A widget does not run inside your app. The operating system asks a separate
extension process for a snapshot, keeps it, and draws it later, possibly hours
after your code ran. A `View` is a live tree bound to signals and to a native
view, and neither can cross into that process. WaterUI therefore does not try:
a widget function returns plain, resolved values, and the platform renders them
with its own widget toolkit.

Widgets are Apple-only today. The authoring surface lives in the
`waterui::widgets` module, which requires the `widgets` feature:

```toml
[dependencies]
waterui = { version = "0.5", features = ["widgets"] }
```

## Platform support

The generated extension is a WidgetKit extension, so the feature targets Apple
platforms. Reading the `water` packaging code
(`cli/src/apple/widgets.rs`):

- **macOS, iOS and the iOS simulator** are the platforms the extension is built
  for. Apple support is ARM64 only, and the extension build rejects an Intel
  architecture.
- **Building the extension requires a macOS host**, because it uses `swiftc` and
  `codesign`. Packaging on another host fails with an error that says so.
- **The `accessory*` families are rejected on macOS.** If any widget declares
  `accessoryCircular`, `accessoryRectangular` or `accessoryInline`, packaging
  for macOS fails with an error naming each such family and widget kind.
- **The extension uses the app's deployment target**: the framework's floor
  (26.0 for macOS and iOS), or the higher one you declare under
  `[apple.deployment-targets]` in `Water.toml`.
- **No other platform has a widget backend.** Only Apple packaging compiles the
  extension. On Android, Windows, Linux and the web, a `#[widget]` function
  still compiles, but nothing presents it.

## A first widget

Put `#[waterui::widget]` on a free function. It takes either no parameter or
one `WidgetContext` (by value or by reference), and returns anything that
implements `IntoTimeline`. Any `WidgetContent` qualifies and becomes a
one-entry timeline dated now. A single `TimelineEntry` or a full `Timeline`
also qualifies.

```rust
use waterui::widgets::*;

#[waterui::widget(name = "Steps", description = "Today's step count")]
fn steps() -> impl WidgetContent {
    vstack((
        text("Steps").font(Font::style(FontStyle::Headline)),
        text("6,241").font(Font::style(FontStyle::LargeTitle)),
    ))
    .alignment(HorizontalAlignment::Leading)
}
```

Import from `waterui::widgets` and not from the prelude. The widget module has
its own `text`, `Text`, `vstack`, `VStack`, `hstack`, `zstack`, `Color`,
`link` and `Link`, which are different items from the prelude's components of
the same names. Importing both globs makes every shared name ambiguous where you
use it. In a file that also builds app views, import the widget items you need
by name.

The attribute accepts these arguments:

| Argument | Required | Meaning |
| --- | --- | --- |
| `name = "…"` | yes | The title in the widget gallery |
| `description = "…"` | yes | The subtitle in the widget gallery |
| `kind = "…"` | no | The WidgetKit configuration kind; defaults to the function name |
| `families = [systemSmall, …]` | no | The supported families; defaults to `systemSmall`, `systemMedium`, `systemLarge` |

`families` takes bare identifiers that spell a `WidgetFamily` case:
`systemSmall`, `systemMedium`, `systemLarge`, `systemExtraLarge`,
`accessoryCircular`, `accessoryRectangular` and `accessoryInline`. An empty
list, an unknown name or a duplicate argument is a compile error.

## The closed content surface

`WidgetContent` is implemented only by the schema's node types and by plain
strings, so only these can appear in a widget:

| Constructor | Node | Notes |
| --- | --- | --- |
| `text(..)` | `Text` | `.font(..)`, `.color(..)`, `.line_limit(..)`; a `&'static str`, `String` or `waterui::Str` converts to text on its own |
| `Image::system(..)` / `Image::data(..)` | `Image` | An SF Symbol name, or encoded image bytes (PNG) decoded in the extension; `.color(..)` tints a template image |
| `vstack` / `hstack` / `zstack` | `Stack` | `.spacing(..)`; `.alignment(..)` on `VStack` and `HStack`, `.horizontal_alignment(..)` and `.vertical_alignment(..)` on `ZStack` |
| `Shape::circle()`, `rectangle()`, `rounded_rectangle(r)`, `capsule()`, `ellipse()` | `Shape` | `.fill(Color)` |
| `fill(color)` or a `Color` | `Color` | Paints the space it occupies |
| `link(url, content)` | `Link` | Opens a URL |
| `intent(&action, content)` | `Intent` | Runs a declared action |

Colors are either `Color::srgb(r, g, b)`, `Color::srgb_u8(r, g, b)` or one of
the semantic colors `Color::PRIMARY`, `Color::SECONDARY` and `Color::ACCENT`,
which follow the user's appearance. Fonts are `Font::style(FontStyle::..)` for a
system text style or `Font::system(size, FontWeight::..)` for an explicit size.

A stack's children are a tuple of up to twelve `WidgetContent` values, or a
`Vec`, an array or an `Option` of one content type. Each tuple element must
itself be content. An `Option` is a whole children list, not content, so it
cannot sit inside a tuple. When the children differ in type or depend on a
condition, convert each with `into_node()` and collect them into a
`Vec<Node>`.

```rust
use waterui::widgets::*;

#[waterui::widget(name = "Steps by language", description = "Today's step count, translated")]
fn translated_steps(context: &WidgetContext) -> impl WidgetContent {
    let rows = ["今日の歩数", "오늘 걸음", "今日步数"]
        .map(|label| text(label).font(Font::system(13.0, FontWeight::Medium)));
    let mut children = vec![text("6,241").font(Font::style(FontStyle::LargeTitle)).into_node()];
    if context.display_size().width > 150.0 {
        children.push(vstack(rows).spacing(4.0).into_node());
    }
    vstack(children).spacing(8.0)
}
```

### A `View` does not compile

Nothing in `waterui::prelude` implements `WidgetContent`. Passing an app
`Text`, `Button` or `VStack` where a widget node goes is a compile error. The
trait declares its own diagnostic: the message reads "`…` cannot appear in a
widget", and a note lists the admitted node kinds (text, image, stack, shape,
color, link and intent) and points you to the `waterui::widgets`
constructors.

This is deliberate. The failure is at compile time, not a silent drop at
render time. It is also why the widget module defines its own `text` and
stacks instead of reusing the component ones.

## Choosing what to draw per family

`WidgetContext` carries what the extension knows when it asks for a timeline:
`family()` returns the `WidgetFamily` and `display_size()` returns a `Size`
with public `width` and `height` fields (`f64`, in points). `WidgetFamily` is
`#[non_exhaustive]`, so a `match` on it needs a wildcard arm. Branches usually
build different types, so convert each with `into_node()` and return `Node`,
which is itself widget content.

```rust
use waterui::widgets::*;

#[waterui::widget(
    name = "Next event",
    description = "What is coming up",
    families = [systemSmall, systemMedium, accessoryInline]
)]
fn next_event(context: WidgetContext) -> Node {
    match context.family() {
        WidgetFamily::AccessoryInline => text("Design review at 15:00").into_node(),
        _ => hstack((
            Image::system("calendar").color(Color::ACCENT),
            vstack((
                text("Design review").font(Font::style(FontStyle::Headline)),
                text("15:00").color(Color::SECONDARY),
            ))
            .alignment(HorizontalAlignment::Leading),
        ))
        .spacing(8.0)
        .into_node(),
    }
}
```

Because this widget declares `accessoryInline`, packaging an app that contains
it for macOS fails. Keep lock-screen families on widgets that the macOS build
does not need.

## Timelines and reload policy

A widget cannot animate or update itself between runs. Instead you hand the
system a `Timeline`: dated entries, each an immutable snapshot that becomes
current at its date, and a `ReloadPolicy` that says when to ask again.

- `TimelineEntry::new(date, content)` makes one entry. `date` is anything
  that converts into `Date`: a `Date`, a `SystemTime` or an `f64` of Unix
  seconds.
- `Timeline::new(entries)` takes anything that converts into
  `Vec<TimelineEntry>`, and `.policy(..)` sets the reload policy. A timeline
  needs at least one entry: the extension stops on an empty one.
- `Date::now()`, `Date::unix_timestamp(seconds)`, and `Date + Duration` or
  `Date - Duration` build instants.
- `ReloadPolicy::AtEnd` (the default) reloads after the last entry's date
  passes, `ReloadPolicy::Never` leaves the timeline standing, and
  `ReloadPolicy::After { date }` reloads at a given instant.

```rust
use std::time::Duration;

use waterui::widgets::*;

fn forecast(label: &'static str) -> impl WidgetContent {
    vstack((text("Forecast").color(Color::SECONDARY), text(label)))
        .alignment(HorizontalAlignment::Leading)
}

#[waterui::widget(name = "Forecast", description = "Three hours ahead")]
fn forecast_widget() -> Timeline {
    let now = Date::now();
    Timeline::new([
        TimelineEntry::new(now, forecast("Sunny")),
        TimelineEntry::new(now + Duration::from_secs(3600), forecast("Cloudy")),
        TimelineEntry::new(now + Duration::from_secs(7200), forecast("Rain")),
    ])
    .policy(ReloadPolicy::After {
        date: now + Duration::from_secs(10_800),
    })
}
```

The function runs in the extension each time the system asks for a timeline,
a snapshot or a placeholder; the latter two show the first entry. Its result is
serialized to the Swift side. It holds no signals and no
environment, and the extension owns none of your application state. Compute
what you need inside the function and put the resolved values in the
snapshot.

## Taps

A `link(url, content)` region opens a URL, and the extension fails loudly if
the string does not parse as one. To run code instead, declare an action.

`#[waterui::widget_intent]` goes on a concrete (non-generic) struct or enum.
It derives `Serialize` and `Deserialize` through `waterui::widgets::serde`, so
your crate needs no `serde` dependency, and it declares the type as an
`AppIntent` for the generated extension. Then you write `impl WidgetIntent`
yourself. Its `perform(self)` is a synchronous unit action that runs in the
extension's process, with the action's own fields as its only context.

```rust
use waterui::widgets::*;

#[waterui::widget_intent(name = "Mark done", kind = "mark-done")]
struct MarkDone {
    task: u32,
}

impl WidgetIntent for MarkDone {
    fn perform(self) {
        waterui::log::info!(task = self.task, "marked a task done from the widget");
    }
}

#[waterui::widget(name = "Task", description = "Complete a task")]
fn task() -> impl WidgetContent {
    hstack((
        text("Write the report"),
        intent(&MarkDone { task: 7 }, Image::system("checkmark.circle")),
    ))
    .spacing(8.0)
}
```

The arguments are `name` (required, the intent's title), `kind` (defaults to
the type's name; set it when two types would collide) and `description`
(defaults to `name`). The kind is the intent's identity and the generated
Swift type name derives from its SHA-256 digest, so keep it stable. Only a type
declared with `#[widget_intent]` can be passed to `intent(..)`. Implementing
`WidgetIntent` alone does not compile there, and neither does a string.

## What `water` generates

You never touch Xcode. When `water` packages an Apple app it reads the
declaration records the macros leave in your crate's artifact, one per
`#[widget]` and one per `#[widget_intent]`, and:

- does nothing if there are no widget records, so an app without widgets gets no
  extension;
- otherwise generates the `WidgetBundle` entry point and the intent types,
  compiles them with the `waterui-widgets` crate's own SwiftUI sources into an
  `.appex`, and signs it inside the app bundle's `PlugIns` directory;
- links the extension against the same Rust library your app uses (the static
  archive in a packaged build, the shared runtime library in a development
  build), so it can call the `waterui_widget_*` functions the macro exports.

Build with the usual commands from [the CLI chapter](../01-getting-started/01-cli.md)
and [packaging](../07-tools/07-packaging.md). The widget code is plain Rust in
your crate, so there is nothing else to register.

## See also

- [Platforms](../01-getting-started/05-platforms.md) for the support tiers
- [Plugins](07-plugins.md) for other ways to extend the app
- [Embedding](13-embedding.md) for hosting WaterUI inside another app

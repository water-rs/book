# Text and typography

> **In this chapter, you will:**
> - Display text using `text()` for static content and `text!` for reactive, localized strings
> - Style text with semantic fonts, weights, colors, and decorations
> - Build rich text with `StyledStr`, including Markdown and concatenation
> - Add syntax highlighting for source code

Text is the most fundamental building block in any user interface. Whether you are showing a headline, a label beside a toggle, or a paragraph of help text, you reach for the `Text` component first. WaterUI gives you a small two-function API: `text()` for plain content that does not change, and the `text!` macro for reactive strings that interpolate captured bindings and (optionally) consult a translation catalog.

## Static text with `text()`

The simplest way to display text is the `text()` function. It accepts anything that converts into `Text` — a `&'static str` becomes localized through the catalog, while `String` and `Str` are used verbatim:

```rust
use waterui::prelude::*;

fn greeting() -> impl View {
    text("Hello, World!")
}
```

`Text` sizes itself to fit its content and never stretches to fill extra space. When the available width is limited, it wraps to multiple lines automatically.

### Layout behavior

Here is what you need to know about how `Text` participates in layout:

- **Sizing:** fits its content naturally, like a label.
- **In stacks:** takes only the space it needs, leaving room for siblings.
- **Wrapping:** wraps when width is constrained — for example by a parent `Frame`.

```rust
use waterui::prelude::*;

fn row() -> impl View {
    // Push two labels apart in a row.
    hstack((text("Name"), spacer(), text("Value")))
}
```

> **Tip:** Because `Text` never stretches on its own, you can safely place it in any stack without worrying about it gobbling up space from sibling views.

## Reactive text with `text!`

Static strings are fine for fixed labels, but most apps need text that updates in response to state. The `text!` macro captures any named placeholder from the surrounding scope and re-evaluates whenever those bindings change. When a `i18n/<locale>.toml` catalog is present, the same call site also resolves the matching translation:

```rust
use waterui::prelude::*;

fn counter_label(count: &Binding<i32>) -> impl View {
    // Captures `count` from scope; the rendered text updates on every change.
    text!("Count: {count}")
}
```

The macro only accepts named placeholders. Either name a binding directly (`{count}`), or alias an expression with `name = expr`:

```rust
use waterui::prelude::*;

fn welcome(get_name: impl Fn() -> String) -> impl View {
    text!("Hello, {name}", name = get_name())
}
```

> **Warning:** `text!` does **not** accept positional `{}` placeholders.
> Writing `text!("Count: {}", count)` will not compile. Use a named
> placeholder (`{count}`) and capture the binding from scope, or pass an
> explicit alias (`name = expr`).

### Why not `.get()` and `format!`?

Calling `.get()` on a binding inside a view body reads the current value once and never re-runs. The text would freeze at construction time. Always express formatting through `text!` so the framework tracks the dependency:

```rust
use waterui::prelude::*;

fn show(value: &Binding<f64>) -> impl View {
    // Reactive: updates whenever `value` changes.
    text!("Value: {value:.2}")
}
```

## Displaying arbitrary values

Any signal whose output type implements `Display` can be rendered with `Text::display`:

```rust
use waterui::prelude::*;

fn show_price(price: &Binding<f64>) -> impl View {
    Text::display(price.clone())
}
```

`Text::display` maps the signal through `to_string()` internally, so the text updates whenever the signal does.

## Locale-aware formatting

For specialised formatting — locale-specific dates or numbers — use `Text::format`:

```rust
use waterui::prelude::*;
use waterui::text::locale::Formatter;

fn formatted<T: 'static + Clone>(value: &Binding<T>, fmt: impl Formatter<T> + 'static) -> impl View {
    Text::format(value.clone(), fmt)
}
```

Implement the `Formatter<T>` trait for any type whose presentation depends on the active locale.

## Translation files for `text!`

If your app supports multiple languages, place TOML files under `i18n/` in the crate root:

```toml
# i18n/en.toml
"Count: {count}" = "Count: {count}"

# i18n/zh.toml
"Count: {count}" = "计数：{count}"
```

The macro picks the right translation based on the active `Locale` in the environment. Missing translation files are not an error — `text!` falls back to the format string itself.

> **Note:** Plural forms use `{#count}` syntax in the format string and a TOML
> table with `one`/`other` keys. See `waterui/macros/src/locale.rs` for the
> full grammar.

## Font system

WaterUI provides a semantic font system with six built-in presets. Each preset resolves to a platform-appropriate size and weight through the environment, so a "Larger Text" accessibility setting cascades into your screen automatically:

| Preset        | Default Size | Default Weight |
|---------------|-------------|----------------|
| `Body`        | 16pt        | Normal         |
| `Title`       | 24pt        | SemiBold       |
| `Headline`    | 32pt        | Bold           |
| `Subheadline` | 20pt        | SemiBold       |
| `Caption`     | 12pt        | Normal         |
| `Footnote`    | 10pt        | Light          |

Use the convenience methods on `Text`. They work on values produced by both `text()` and `text!`:

```rust
use waterui::prelude::*;

fn typography() -> impl View {
    vstack((
        text("Page Title").title(),
        text("Main heading").headline(),
        text("Section header").sub_headline(),
        text("Body content").body(),
        text("Small note").caption(),
        text("Legal text").footnote(),
    ))
}
```

### Custom font configuration

For fine-grained control, build a `Font` value and pass it to `.font()`:

```rust
use waterui::prelude::*;
use waterui::text::font::{Font, FontWeight};

fn custom() -> impl View {
    text("Custom").font(
        Font::default()
            .size(18.0)
            .weight(FontWeight::Medium)
            .family("monospace"),
    )
}
```

### Font weights

The `FontWeight` enum provides nine standard weights:

```rust
pub enum FontWeight {
    Thin,       // 100
    UltraLight, // 200
    Light,      // 300
    Normal,     // 400 (default)
    Medium,     // 500
    SemiBold,   // 600
    Bold,       // 700
    UltraBold,  // 800
    Black,      // 900
}
```

### Size, weight, italic shortcuts

You do not always need to construct a `Font`. `Text` provides direct shortcuts. `.size()` and `.weight()` accept signals, so they can react:

```rust
use waterui::prelude::*;

fn highlight(emphasised: &Binding<bool>) -> impl View {
    vstack((
        text("Large bold text").size(28.0).bold(),
        // Italic toggles reactively from the binding.
        text("May be italic").italic(emphasised.clone()),
    ))
}
```

## Color

Colors are zero-sized marker types you can pass into `.color()` or `.foreground()`. The built-in palette includes `Red`, `Blue`, `Green`, `Orange`, `Purple`, `Cyan`, `Yellow`, `Pink`, and `Grey`:

```rust
use waterui::prelude::*;

fn status() -> impl View {
    vstack((
        text("Error message").color(Red),
        text("Success").color(Green),
        text("Highlighted").background_color(Yellow),
    ))
}
```

> **Note:** `.color()` on `Text` sets an explicit foreground color for that
> specific text view. The more general `.foreground()` modifier from `ViewExt`
> sets the inherited foreground for an entire view subtree, so children
> respect the cascade.

## Text decorations

### Underline

```rust
use waterui::prelude::*;

fn link_label(highlighted: &Binding<bool>) -> impl View {
    text("Click here").underline(highlighted.clone())
}
```

`.underline()` accepts any `IntoSignal<bool>`, so the decoration can toggle reactively.

### Strikethrough

Strikethrough lives on `StyledStr`, not on `Text`:

```rust
use waterui::prelude::*;
use waterui::text::styled::StyledStr;

fn deprecated() -> impl View {
    text(StyledStr::plain("Deprecated").strikethrough(true))
}
```

## Concatenating text

Sometimes you need mixed styles within a single line. `Text` implements `Add` and `AddAssign`, so styled fragments compose with `+`:

```rust
use waterui::prelude::*;

fn name_row() -> impl View {
    text("Name: ").bold() + text("Alice")
}
```

The resulting `Text` preserves the styling of each fragment.

## Rich text with `StyledStr`

For full control over rich text, build a `StyledStr` directly. Each chunk carries its own `Style`, which includes font, foreground color, background color, italic, underline, and strikethrough:

```rust
use waterui::prelude::*;
use waterui::text::styled::{Style, StyledStr};

fn intro() -> impl View {
    let mut styled = StyledStr::empty();
    styled.push("Bold intro: ", Style::default().bold());
    styled.push("normal continuation", Style::default());
    text(styled)
}
```

### Markdown shorthand

`StyledStr::from_markdown` parses a small subset of Markdown — headings, bold, italic, strikethrough, inline code, and paragraphs — into a styled string in one step:

```rust
use waterui::prelude::*;
use waterui::text::styled::StyledStr;

fn release_notes() -> impl View {
    text(StyledStr::from_markdown("**Bold** and *italic* with `code`"))
}
```

## Syntax highlighting

If your app displays source code, WaterUI ships a `syntect`-backed highlighter. `highlight_text` is synchronous: it consumes a borrowed source string and a mutable highlighter, and returns a fully styled `StyledStr`:

```rust
use waterui::prelude::*;
use waterui::text::highlight::{DefaultHighlighter, Language, highlight_text};

fn code_view(source: &str) -> impl View {
    let mut highlighter = DefaultHighlighter::default();
    text(highlight_text(Language::Rust, source, &mut highlighter))
}
```

The `Language` enum covers Rust, Swift, Python, TypeScript, and many others. Each chunk in the resulting `StyledStr` carries the appropriate syntax color.

## Quick reference

| Method / Function       | Purpose                                       |
|------------------------|-----------------------------------------------|
| `text("...")`          | Static text, localized for `&'static str`     |
| `text!("Count: {n}")`  | Reactive, localized text capturing `n`         |
| `Text::display(sig)`   | Render any `Signal<Output: Display>`          |
| `Text::format(v, fmt)` | Locale-aware formatted text                    |
| `.title()`             | Apply the `Title` font preset                  |
| `.headline()`          | Apply the `Headline` font preset               |
| `.sub_headline()`      | Apply the `Subheadline` font preset            |
| `.body()`              | Apply the `Body` font preset                   |
| `.caption()`           | Apply the `Caption` font preset                |
| `.footnote()`          | Apply the `Footnote` font preset               |
| `.size(f64)`           | Set a custom font size (accepts signals)       |
| `.bold()`              | Set font weight to `Bold`                      |
| `.weight(w)`           | Set a specific font weight (accepts signals)   |
| `.italic(sig)`         | Toggle italic styling reactively               |
| `.color(c)`            | Set the text foreground color                  |
| `.background_color(c)` | Set the text background color                  |
| `.underline(sig)`      | Toggle underline reactively                    |
| `.font(f)`             | Apply a fully custom `Font`                    |

Now that you can display and style text, it is time to learn how to arrange views on screen. In the [next chapter](02-layout.md), you will explore stacks, frames, grids, and the rest of the layout system.

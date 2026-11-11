# Icons and SVG

> **In this chapter, you will:**
> - Pick a portable icon set and render its icons as ordinary views
> - Size, tint and label icons, and let them follow the surrounding foreground color
> - Know why `SystemIcon` and SF Symbols are Apple-only and have no substitute elsewhere
> - Draw your own artwork with `Svg::new`, `Svg::from_path` and `Svg::from_stroke_path`

An icon in WaterUI is a view like any other. There is no `Icon` component with a name lookup; instead, an icon-set crate exports one function per icon, and each function returns a view you can put in a stack, a button label or a toolbar. This chapter covers the three portable sets, the single Apple-only exception, and the `Svg` view that the sets are built on.

## Portable icon sets

Icon sets ship as separate crates, so an app pays only for the one it depends on. **Pick one set per app** and match it to the design language you are targeting.

| Crate | Set | Default rendering |
|---|---|---|
| `waterui-icons-lucide` | Lucide (stroke icons) | `Svg`, feature `svg` (default) |
| `waterui-icons-material-icon` | Material Design icons from Pictogrammers | `Svg`, feature `svg` (default) |
| `waterui-icons-fontawesome7` | Font Awesome 7 Free | Webfont glyph, `IconGlyph`; `svg` is optional |

Add the crate you want to your app's `Cargo.toml`, then import it under a short alias. Icon names are `snake_case` versions of the upstream kebab-case names: `arrow-left` becomes `arrow_left()`.

```rust,ignore
use waterui::prelude::*;
use waterui_icons_lucide as lucide;
use waterui_icons_material_icon as mdi;

fn toolbar() -> impl View {
    hstack((
        lucide::house(),
        lucide::settings(),
        mdi::account_circle(),
        mdi::check_circle(),
    ))
    .spacing(16.0)
}
```

The icon-set crates are separate dependencies, not facade re-exports, so fences that import them are marked `ignore` in this book; the names above are taken from the crates' own documentation and the `examples/icons` example in the framework repository.

Every Lucide and Material icon is an `Svg` with a 24 by 24 point drawing box. Besides the function, each icon also has a `*_PATH` constant holding its raw path data (`lucide::HOUSE_PATH`, `mdi::HOME_PATH`), which you can feed to `Svg::from_path` or, for Lucide's outline paths, `Svg::from_stroke_path` yourself. Names follow each upstream set, so they differ between sets: Lucide calls the house icon `house`, Material calls it `home`.

### Size and color

Because an icon is an `Svg`, two methods matter. `.size(width, height)` comes from `ViewExt` and gives the icon a layout box. The drawing scales down to fit a smaller box, but its drawing box is also its largest size: a 24-point icon in a 32-point box stays 24 points and sits centered in the box. `.tint(color)` is an `Svg` method that takes anything convertible to a `Color`.

```rust,ignore
use waterui::prelude::*;
use waterui_icons_lucide as lucide;
use waterui_icons_material_icon as mdi;

fn status_row() -> impl View {
    hstack((
        mdi::check_circle().tint(Color::srgb_hex("#10B981")),
        lucide::star().tint(Color::srgb_hex("#F59E0B")),
        mdi::heart().size(16.0, 16.0),
    ))
    .spacing(16.0)
}
```

An icon with no `.tint(...)` does not pick a color of its own. It reads the foreground color of the surrounding environment, so the same icon is dark on a light surface and light on a dark one, and a `.foreground(...)` on an ancestor recolors every untinted icon below it. Reach for a theme token before a literal hue; see [Styling](11-styling.md) for the tokens.

You can use a token with `.foreground(...)` on the icon, or convert it to a `Color` for `.tint(...)`. The following block compiles against the facade alone, using the generic `Svg` view instead of a set crate:

```rust
use waterui::prelude::*;
use waterui::prelude::theme_color::Accent;
use waterui::svg::Svg;

/// Material "home" path data, 24 by 24.
const HOME: &str = "M10 20v-6h4v6h5v-8h3L12 3 2 12h3v8z";

fn accent_home() -> impl View {
    Svg::from_path(HOME, 24.0, 24.0).foreground(Accent).size(20.0, 20.0)
}

fn red_home() -> impl View {
    Svg::from_path(HOME, 24.0, 24.0).tint(Color::red()).size(20.0, 20.0)
}
```

Do not confuse `.size(w, h)` with `Svg::viewbox(w, h)`. `viewbox` declares the coordinate system the path data was drawn in, which is also the drawing's natural and largest size; shrinking it reframes the artwork instead of resizing it. Use `.size` for layout.

### Font Awesome

Font Awesome defaults to webfont rendering. Its icons are `IconGlyph` constants grouped by style, and you size them with `with_size`:

```rust,ignore
use waterui::prelude::*;
use waterui_icons_fontawesome7 as fa;

fn links() -> impl View {
    hstack((
        fa::solid::HOUSE,
        fa::regular::HEART.with_size(32.0),
        fa::brands::GITHUB,
    ))
    .spacing(16.0)
}
```

The three styles are `solid`, `regular` and `brands`. A webfont glyph is styled text, so the font file must be bundled with the app. Enable the crate's `svg` feature if you would rather have `fa::solid::house()` as an `Svg`, with `*_PATH` and `*_VIEWBOX` constants for the raw data. Those icons keep Font Awesome's own drawing box, which is hundreds of units wide (`HOUSE_VIEWBOX` is a `(width, height)` pair), so give each one a `.size(...)`. Font Awesome Free icons are licensed CC BY 4.0, which requires attribution.

`IconGlyph` is also public on its own, so you can describe a glyph from any icon font yourself. `IconGlyph::new` takes a codepoint and a font family name, and `with_size` and `color` adjust it:

```rust
use waterui::icon::IconGlyph;
use waterui::prelude::*;

const HOME: IconGlyph = IconGlyph::new('\u{e88a}', "MaterialIcons-Regular");

fn big_red_home() -> impl View {
    HOME.with_size(32.0).color(Color::red())
}
```

## Accessibility labels

A bare icon carries no text, so a screen reader has nothing to announce. Give every icon that conveys meaning a label with `.a11y_label(...)`:

```rust
use waterui::prelude::*;
use waterui::svg::Svg;

const TRASH: &str = "M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z";

fn delete_icon() -> impl View {
    Svg::from_path(TRASH, 24.0, 24.0)
        .size(24.0, 24.0)
        .a11y_label("Delete")
}
```

The label is reactive, so it can be a signal derived from state. Decorative icons that sit next to a text label should be hidden from assistive technology with `.a11y_hidden(true)` instead, so the control is not announced twice. When you hand `Svg::new` a document that has a root `<title>` element, that title is offered as the drawing's label, and an explicit `.a11y_label(...)` wins over it. The [Accessibility chapter](../06-advanced/05-accessibility.md) covers labels in full.

## SystemIcon and SF Symbols: Apple only

`SystemIcon` names a glyph from the platform's own icon catalog. Only Apple platforms have one: SF Symbols. `SystemIcon` is therefore intentionally asymmetric.

- On the Apple backend it renders the named SF Symbol (`"house"`, `"chevron.right"`).
- Everywhere else it is **unsupported**. Android has no OS-supplied icon catalog, and WaterUI does not substitute a bundled font and call it the system's. A silent substitute would hide the asymmetry from code that should know about it. Hydrolysis, the self-drawn backend that draws Android and Linux, panics when it reaches a `SystemIcon`, with a message naming the icon and pointing you to a packaged icon crate. That includes an Apple target built with the Hydrolysis backend.

The framework states this as a rule: a capability is unsupported only when it depends on an asset that the platform alone can supply and that platform lacks. SF Symbols on Android is that case. A missing control is never such a case, which is why the portable sets above exist.

```rust
use waterui::icon::{SystemIcon, system_icon};
use waterui::prelude::*;

fn apple_only() -> impl View {
    hstack((
        system_icon::home(),
        system_icon::settings(),
        SystemIcon::new("wifi"),
    ))
    .spacing(16.0)
}
```

`system_icon` offers a dozen constructors (`home`, `settings`, `search`, `person`, `plus`, `trash`, `chevron_right`, `chevron_left`, `close`, `checkmark`, `star`, `heart`), each mapped to an SF Symbol name; `SystemIcon::new` and `SystemIcon::from_static` take any other name. Two crates wrap larger name lists: `waterui-icons-sf-symbol` and `waterui-icons-native`. Neither changes the platform rule.

If one codebase must show an SF Symbol on Apple and something else elsewhere, you write that choice yourself. Gate the dependency to Apple targets in `Cargo.toml` and gate every use site with the same condition. Selecting between sibling functions keeps the per-platform choice in one place. The condition below assumes your Apple builds use the Apple backend.

```toml
[target.'cfg(target_vendor = "apple")'.dependencies]
waterui-icons-sf-symbol = "0.5"
```

```rust,ignore
use waterui::prelude::*;

#[cfg(target_vendor = "apple")]
fn settings_icon() -> impl View {
    waterui_icons_sf_symbol::gear()
}

#[cfg(not(target_vendor = "apple"))]
fn settings_icon() -> impl View {
    waterui_icons_lucide::settings()
}
```

For an app that looks the same on every platform, skip `SystemIcon` entirely and use one portable set.

## Drawing with Svg

The Lucide and Material sets, and Font Awesome with its `svg` feature, are thin wrappers over `Svg`, which is available as `waterui::svg::Svg`. That module exists only with the facade's `gpu` feature, which is on by default; an app built with `default-features = false` and no `gpu` has no `waterui::svg`. `Svg` parses with `usvg` and records the drawing through the graphics layer, so it renders the same on every backend. Everything it draws is geometry, and it never asks the operating system for a font: an SVG `<text>` element draws nothing, so convert text to paths before using artwork that contains it. (`waterui-svg`'s non-default `text` feature, with `Svg::fonts` taking a font database you build, lays `<text>` out into outlines; the facade does not turn it on.)

There are three constructors.

| Constructor | Input | Use it for |
|---|---|---|
| `Svg::new(content)` | A full SVG document | Illustrations and multi-color artwork |
| `Svg::from_path(d, width, height)` | A path `d` attribute and its box | Filled icons |
| `Svg::from_stroke_path(d, width, height)` | A path `d` attribute and its box | Outline icons |

Bare path data belongs in `from_path` or `from_stroke_path`, which wrap it in a document of the given box. `Svg::new` hands its content to the parser as it is, and content that does not parse as SVG panics when the view renders.

`from_stroke_path` draws with a stroke width of 2, round caps and round joins, and no fill, which is the Lucide convention. `tint` and the inherited foreground recolor a path-data icon. A full document from `Svg::new` takes neither on its own: give it its drawing box with `.viewbox(width, height)`, matching the document's own `viewBox`, and the color then replaces every `currentColor` in it. Paint written as a literal color stays as written:

```rust
use waterui::prelude::*;
use waterui::svg::Svg;

const MENU: &str = "M3 12h18M3 6h18M3 18h18";

const BADGE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
  <circle cx="12" cy="12" r="10" fill="currentColor"/>
  <path d="M8 12l3 3 5-6" fill="none" stroke="#ffffff" stroke-width="2"/>
</svg>"##;

fn artwork() -> impl View {
    hstack((
        Svg::from_stroke_path(MENU, 24.0, 24.0),
        Svg::new(BADGE)
            .viewbox(24.0, 24.0)
            .tint(Color::srgb_hex("#3B82F6")),
    ))
    .spacing(16.0)
}
```

For anything longer than a path, keep the markup in its own file and include it with `include_str!` instead of pasting it into the source, as in `Svg::new(include_str!("logo.svg"))`.

An `Svg` built from path data, or a document given a `.viewbox`, follows the foreground color unless you tint it, exactly as the set icons do. To show a bitmap rather than vector art, use the media components in [Assets](../04-rich/05-assets.md).

## See also

- [Styling](11-styling.md) for theme color tokens and foreground inheritance
- [Controls](03-controls.md) for putting icons inside buttons
- [Accessibility](../06-advanced/05-accessibility.md) for labels and hiding decorative views
- [Platforms](../01-getting-started/05-platforms.md) for the tier each platform sits in

Next, browse the rich components, starting with [assets and fonts](../04-rich/05-assets.md).

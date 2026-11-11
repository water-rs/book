# Colors, themes, and styles

> **In this chapter, you will:**
> - Style a view from theme tokens first, and know when a concrete color is the right call
> - Build colors in sRGB, Display P3 and OKLCH, transform them, and drive them from signals
> - Install a `Theme` for dark mode, brand colors and type scale, and seed platform resources from `Water.toml`
> - Paint backgrounds with colors, gradients, `Material` and `Glass`, and shape them with fills and clips
> - Use floating surfaces, cards and shadows, and see where Material 3 plugs in

A WaterUI screen should look right before you style anything. A `text!("Hello")` with no modifiers is dark on light in light mode, light on dark in dark mode, and uses the platform's own palette. Producing those defaults is the framework's job: each backend reads **theme tokens** such as `Foreground` and `Surface` and fills them in. Your job is to say which *role* a color plays, not which hue it has.

That is the one rule of this chapter: reach for a token first, and for a concrete color only when the design calls for that exact hue. A screen made of `Foreground`, `MutedForeground`, `Surface`, `Border` and `Accent` follows light and dark mode, the platform palette and a Material 3 theme with no conditional code. If the defaults look wrong on a platform, that is a backend bug to report, not something to hide by hard-coding a color in a view.

## Theme tokens

Tokens live in `waterui::theme::color`, which the prelude re-exports as `theme_color`:

```rust
use waterui::prelude::*;

fn sync_status() -> impl View {
    vstack((
        text!("Sync"),
        text!("Last run: never").caption().muted(),
    ))
}

fn banner() -> impl View {
    text!("Heads up")
        .foreground(theme_color::AccentForeground)
        .padding()
        .background(theme_color::Accent)
}
```

`.foreground(..)` sets the inherited foreground for a whole subtree. `.muted()` is shorthand for `.foreground(theme_color::MutedForeground)`, and `.foreground(theme_color::Foreground)` restores the primary color below a muted subtree. Each token is a zero-sized struct that is also a `View`, so `theme_color::Surface.size(80.0, 40.0)` paints a themed rectangle, and anything that takes `impl Into<Color>` accepts a token directly.

The prelude also exports a `Background` enum and a `Border` struct for other jobs. The tokens with the same names are always written through the `theme_color::` path.

| Token | Role |
|---|---|
| `Background` | Window or page background |
| `Surface` | Elevated surface: cards, sheets |
| `SurfaceVariant` | Alternate surface |
| `Border` | Borders and dividers |
| `Foreground` | Primary text and icons |
| `MutedForeground` | Secondary and dimmed text |
| `TertiaryForeground` | Fainter than muted, such as a month grid's days outside the shown month |
| `DisabledForeground` | Text and glyphs of a control that cannot be used |
| `Accent` | Interactive emphasis |
| `AccentContainer` | Container associated with the accent |
| `AccentForeground` | Foreground drawn on accent backgrounds |
| `Tertiary` | Contrasting accent for complementary emphasis |
| `TertiaryContainer` | Container associated with the tertiary accent |
| `SelectionContainer` | Fill behind a selected item |
| `SelectionForeground` | Foreground drawn on the selection container |
| `UnemphasizedSelection` | Fill behind a selected item while its control is not focused |
| `Error` | Destructive and error emphasis: badges, destructive buttons, validation |
| `ErrorForeground` | Foreground drawn on the error color |
| `Scrim` | Dimming backdrop a modal layer paints behind a dialog |

Tokens come in pairs. Draw `AccentForeground` on `Accent`, `ErrorForeground` on `Error` and `SelectionForeground` on `SelectionContainer`; a theme picks each pair to be legible together.

A token that no backend or theme installed is a contract violation: a debug build panics naming the token, and a release build logs the error and resolves opaque black. In a running app every slot is installed by the time a view resolves a token. You meet that panic in tests that build an `Environment` by hand, or by resolving a token inside `app` itself on Hydrolysis, which installs its tokens only after `app` returns. [The resolvers chapter](../06-advanced/08-resolvers.md) covers the non-panicking probes.

### Tinting controls

`.tint(color)` sets the color the controls in a subtree draw their emphasis with (buttons, switches, sliders, progress indicators) instead of `Accent`. The nearest `.tint` wins, and the color can follow a signal through `signal_color` (below).

```rust
use waterui::prelude::*;

fn confirm_row() -> impl View {
    vstack((
        button("Delete account").action(|| {}).tint(theme_color::Error),
        button("Save").action(|| {}),
    ))
}
```

## Concrete colors

When the design needs a specific hue, build a `Color`. The constructors differ in the space you author in:

```rust
use waterui::prelude::*;

fn palette() -> Vec<Color> {
    vec![
        Color::srgb(59, 130, 246),          // 0-255 integers
        Color::srgb_f32(0.23, 0.51, 0.96),  // 0.0-1.0 floats
        Color::srgb_hex("#3B82F6"),         // panics on malformed input
        Color::srgb_u32(0x3B82F6),          // packed 0xRRGGBB
        Color::p3(0.20, 0.50, 1.0),         // Display P3, wider than sRGB
        Color::oklch(0.65, 0.19, 255.0),    // lightness, chroma, hue in degrees
        Color::transparent(),
        Color::blue(),                      // a named palette color
    ]
}
```

OKLCH is the space to author UI colors in: lightness, chroma and hue move independently, so contrast stays predictable when you change one of them. `Color::srgb_hex` panics on a malformed string; use `Color::try_srgb_hex` for text you did not write:

```rust
use waterui::color::HexColorError;
use waterui::prelude::*;

fn parse_user_color(input: &str) -> Result<Color, HexColorError> {
    Color::try_srgb_hex(input)
}
```

For a value you want in a `const`, use `Srgb`. It takes `0.0`-`1.0` floats in `Srgb::new`, and is itself a view and a valid argument to `.foreground(..)` and `.background(..)`:

```rust
use waterui::color::Srgb;
use waterui::prelude::*;

const BRAND: Srgb = Srgb::from_hex("#3B82F6");
const PAPER: Srgb = Srgb::new(0.98, 0.97, 0.95);

fn brand_label() -> impl View {
    text!("WaterUI").foreground(BRAND).padding().background(PAPER)
}
```

The named palette colors are unit structs and `Color` constructors: `Red`, `Pink`, `Purple`, `DeepPurple`, `Indigo`, `Blue`, `LightBlue`, `Cyan`, `Teal`, `Green`, `LightGreen`, `Lime`, `Yellow`, `Amber`, `Orange`, `DeepOrange`, `Brown`, `Grey` and `BlueGrey`, so `Color::deep_orange()` and `DeepOrange` mean the same thing.

### Transforming colors

Every transform returns a new `Color`. `lighten`, `darken`, `saturate`, `desaturate` and `hue_rotate` work in OKLCH, so `lighten` and `darken` change perceived lightness rather than RGB channels:

```rust
use waterui::prelude::*;

fn swatches() -> impl View {
    let brand = Color::srgb_hex("#3B82F6");
    hstack((
        brand.clone().size(48.0, 48.0),
        brand.clone().lighten(0.15).size(48.0, 48.0),
        brand.clone().darken(0.15).size(48.0, 48.0),
        brand.clone().desaturate(0.5).size(48.0, 48.0),
        brand.clone().hue_rotate(40.0).size(48.0, 48.0),
        brand.clone().mix(Color::red(), 0.5).size(48.0, 48.0),
        brand.with_opacity(0.4).size(48.0, 48.0),
    ))
}
```

`lighten` and `darken` take a lightness delta in `0.0`-`1.0`, `saturate` and `desaturate` scale chroma, `hue_rotate` takes degrees, and `mix(other, factor)` interpolates linearly in the working space (linear Display P3) rather than in OKLCH. A bare `Color` is greedy and fills whatever space it is offered, which is why each swatch above has a `.size(..)`.

Keep two kinds of opacity apart. `Color::with_opacity(..)` bakes alpha into the color value, and accepts a signal. `.opacity(..)` on a view fades the whole rendered subtree, content and all.

### Colors that follow a signal

`.fill(..)`, `.background(..)` and `.foreground(..)` take a `Color`, not a signal. Wrap the signal with `signal_color`, which produces a `Color` that updates in place:

```rust
use waterui::prelude::*;
use waterui::shape::{Circle, ShapeExt};

fn status_dot() -> impl View {
    let online = Binding::bool(false);
    let fill = signal_color(online.select(Color::green(), Color::grey()));
    vstack((
        Circle.fill(fill).size(16.0, 16.0),
        toggle("Online", &online),
    ))
}
```

Reading the signal once with `.snapshot()` would freeze the color at that moment, which is the mistake `signal_color` exists to prevent. [The reactive chapter](../02-core/02-reactive.md) explains why.

Tokens work the same way, because a token is just another color that resolves against the environment:

```rust
use waterui::prelude::*;

fn inbox_row(selected: &Binding<bool>) -> impl View {
    let fill = signal_color(selected.select(
        Color::from(theme_color::SelectionContainer),
        Color::transparent(),
    ));
    text!("Inbox").padding().background(fill)
}
```

### HDR

`color.with_headroom(h)` scales a color's channels by `1 + h`, pushing it past SDR white. Views render in extended range by default: on Apple a view follows its display's EDR capability, so the headroom shows wherever the screen can show it. `.color_space(ColorSpace::Sdr)` clamps a subtree to standard range, which suits avatars and screenshots, and `.color_space(ColorSpace::Hdr)` restores extended range below an `Sdr` ancestor. The modifier takes a plain enum rather than a signal:

```rust
use waterui::metadata::secure::ColorSpace;
use waterui::prelude::*;

fn glow() -> impl View {
    hstack((
        Color::srgb_hex("#FF4D00").with_headroom(0.5).size(80.0, 80.0),
        Color::srgb_hex("#FF4D00")
            .with_headroom(0.5)
            .size(80.0, 80.0)
            .color_space(ColorSpace::Sdr),
    ))
}
```

On Hydrolysis the modifier has no effect: its engine decides headroom per surface.

## Installing a theme

`Theme` is a builder and a plugin. Install it in your `app` function. Only the fields you set take effect, and every slot you leave alone keeps the backend's value. The Apple backend installs its tokens before your `app` function runs; Hydrolysis layers its own underneath your environment after `app` returns, so either way your fields win.

`color_scheme` takes a signal, which is how a runtime dark-mode switch works without a rebuild:

```rust
use waterui::app::App;
use waterui::prelude::*;

fn root(dark: Binding<bool>) -> impl View {
    vstack((
        text!("Appearance").title(),
        toggle("Dark mode", &dark),
    ))
}

pub fn app(mut env: Environment) -> App {
    let dark = Binding::bool(false);
    env.install(Theme::new().color_scheme(dark.select(ColorScheme::Dark, ColorScheme::Light)));
    App::new(move || root(dark.clone()), env)
}
```

Leave `color_scheme` out to follow the system appearance. `ColorScheme` has two variants, `Light` and `Dark`. On Hydrolysis the framework's default tokens and the style package both follow the scheme you install. The Apple backend's tokens follow the system appearance and do not read it, so there the switch reaches only code that reads the scheme itself.

`ColorSettings` overrides individual tokens, and `FontSettings` overrides the type scale. Every setter takes a signal, so a color can follow the same signal as the scheme:

```rust
use waterui::app::App;
use waterui::color::Srgb;
use waterui::prelude::*;
use waterui::text::font::{FontWeight, ResolvedFont};

fn root(dark: Binding<bool>) -> impl View {
    vstack((
        text!("Branded").title(),
        toggle("Dark mode", &dark),
    ))
}

pub fn app(mut env: Environment) -> App {
    let dark = Binding::bool(false);
    let accent = dark.select(
        Srgb::from_hex("#7AA2FF").resolve(),
        Srgb::from_hex("#0057FF").resolve(),
    );
    env.install(
        Theme::new()
            .color_scheme(dark.select(ColorScheme::Dark, ColorScheme::Light))
            .colors(ColorSettings::new().accent(accent))
            .fonts(FontSettings::new().title(ResolvedFont::new(30.0, FontWeight::Bold))),
    );
    App::new(move || root(dark.clone()), env)
}
```

Derive such a color from a signal you own, not from `theme::current_color_scheme(&env)` inside `app`: on Hydrolysis no scheme is installed yet at that point, and a debug build panics. Inside a view body the scheme is always there.

`ColorSettings` has one setter per token in the table above (`background`, `surface`, `muted_foreground`, `scrim` and so on). The colors it takes are `WorkingColor`, the linear Display P3 working space: `Srgb::from_hex(..).resolve()` converts a hex color, and a plain `Srgb` value is accepted where a constant is enough. `FontSettings` has slots for `body`, `title`, `headline`, `subheadline`, `caption` and `footnote`; [the text chapter](01-text.md) covers how they are used.

`env.install(theme)` on the app environment applies everywhere. Calling `.install(theme)` on a view applies it to that subtree only, as [the environment chapter](../02-core/03-environment.md) describes for plugins in general.

On Hydrolysis the application's own environment has the last word: framework defaults come first, then the style package's tokens, then whatever your `app` installed. A `Theme` you install therefore wins over the style, and the style fills every slot you left alone.

### Seeding platform resources from Water.toml

Some colors are needed before any Rust runs: the launch screen, the Android window theme, the Apple accent. The `[theme]` table of `Water.toml` carries those, as `#RRGGBB` strings:

```toml
[theme]
background = "#0B0B0F"
surface = "#15151C"
surface_variant = "#1E1E28"
border = "#2A2A36"
foreground = "#F5F5F7"
muted_foreground = "#A0A0AE"
accent = "#4A84F6"
accent_foreground = "#FFFFFF"
```

The eight slots are `background`, `surface`, `surface_variant`, `border`, `foreground`, `muted_foreground`, `accent` and `accent_foreground`. A malformed color fails when the manifest is read. The CLI reads them when it stages the generated project: `accent` becomes the Apple `AccentColor` asset, `background`, `accent` and `foreground` are bound into the generated Android theme, and `background` is the default for the launch screen's color. [The project chapter](../01-getting-started/04-project-structure.md) covers the file as a whole. The `Theme` plugin is how you change tokens inside the running app; keep the two in agreement.

## Backgrounds

`.background(..)` accepts three kinds of value, and the call looks the same for all of them:

- any `View`: a color, a token, a gradient, a shape fill, an image;
- a `Material`, the platform's frosted pane;
- a `Glass`, the Liquid Glass surface.

```rust
use waterui::prelude::*;
use waterui::shape::RoundedRectangle;

fn surfaces() -> impl View {
    vstack((
        text!("Solid").padding().background(theme_color::Surface),
        text!("Frosted")
            .padding()
            .background(Material::Regular)
            .clip(RoundedRectangle::new(0.2)),
        text!("Glass").padding().background(Glass::regular()),
    ))
}
```

Gradients are views, so they work anywhere a view does. They take `WorkingColor` stops, and their start and end points are in unit space, where `(0, 0)` is the top-left corner:

```rust
use waterui::color::Srgb;
use waterui::prelude::*;

fn hero() -> impl View {
    let rose = Srgb::from_hex("#FF4D80").resolve();
    let sky = Srgb::from_hex("#4D9FFF").resolve();
    text!("Hello").padding().background(Gradient::linear(
        vec![(0.0, rose), (1.0, sky)],
        [0.0, 0.0],
        [1.0, 1.0],
    ))
}
```

[The gradients chapter](../05-graphics/06-gradients.md) covers radial, mesh and animated gradients.

### Material

`Material` has five thicknesses: `UltraThin`, `Thin`, `Regular` (the default), `Thick` and `UltraThick`. `Regular`, `Thick` and `UltraThick` treat the content of your own window behind the view, clipped to the view's shape. `UltraThin` and `Thin` blur what lies *behind the window*, which is the compositor's work. Apple realizes them as a view's background too: macOS blends them behind the window, and iOS over the app's own content. On Hydrolysis a behind-window level works only as a window's background (`Window::background`, see [the windows chapter](08-windows.md)); as a view's background it panics naming the level. Apple and Hydrolysis both realize `Material`; a backend that does not, such as an experimental one, draws the content without the background.

### Glass

Liquid Glass is a different surface with its own parameters, so it is its own type, `Glass`, rather than a sixth thickness. It has two looks (`Glass::regular()` and `Glass::clear()`, the latter for surfaces over media) and three options:

```rust
use waterui::prelude::*;
use waterui::shape::RoundedRectangle;

fn glass_card() -> impl View {
    text!("Save")
        .padding()
        .background(
            Glass::clear()
                .interactive(true)
                .tint(theme_color::Accent)
                .shape(RoundedRectangle::new(0.2)),
        )
}
```

Glass is a capsule unless you give it another `.shape(..)`. The outline belongs to the glass itself, so do not `.clip(..)` a glass background: a mask over glass destroys its refraction. Apple backends project it onto the platform's glass effect; other backends approximate it or draw the content without it. When several glass surfaces sit close together, `.material_group()` on their common parent lets a style that supports it share one backdrop capture and merge them where they touch.

## Shapes

A shape describes an outline in unit space; it is not a view on its own. Use it two ways: `.fill(color)` from `ShapeExt` turns it into a view that fills the space it is given, and `.clip(shape)` cuts any view to it.

```rust
use waterui::prelude::*;
use waterui::shape::{Capsule, Circle, FixedRoundedRectangle, RoundedRectangle, ShapeExt};

fn shapes() -> impl View {
    hstack((
        Circle.fill(theme_color::Accent).size(48.0, 48.0),
        Capsule.fill(theme_color::SurfaceVariant).size(96.0, 32.0),
        RoundedRectangle::new(0.25).fill(theme_color::Surface).size(48.0, 48.0),
        FixedRoundedRectangle::new(12.0).fill(theme_color::Surface).size(120.0, 48.0),
        Color::blue().size(48.0, 48.0).clip(Circle),
    ))
}
```

The available shapes are `Rectangle`, `RoundedRectangle`, `FixedRoundedRectangle`, `UnevenRoundedRectangle`, `FixedUnevenRoundedRectangle`, `Circle`, `Ellipse`, `Capsule` and `Path`.

**`RoundedRectangle::new(r)` takes a fraction of the shorter side, not points.** `0.1` is a subtle round, `0.5` is fully rounded, and larger values saturate at `0.5`. `RoundedRectangle::new(12.0)` therefore compiles and produces a capsule, which is the most common shape mistake. Two types say what you meant:

- `FixedRoundedRectangle::new(12.0)` takes a radius in logical points that stays constant however the shape resizes, the way a design spec states a 12dp card corner. A radius larger than half the shorter side degenerates to the capsule.
- `Capsule` is "fully rounded at any size".

`UnevenRoundedRectangle::new(top_leading, top_trailing, bottom_leading, bottom_trailing)` takes four normalized radii in that order, which is reading order and not clockwise. `FixedUnevenRoundedRectangle` takes the same four in points.

A `Path` is built in unit space, where every coordinate is a fraction of the bounds and the result stretches to fit. Each method consumes `self`, so chain or reassign:

```rust
use waterui::prelude::*;
use waterui::shape::{Path, ShapeExt};

fn triangle() -> impl View {
    Path::new()
        .move_to(0.5, 0.0)
        .line_to(1.0, 1.0)
        .line_to(0.0, 1.0)
        .close()
        .fill(theme_color::Accent)
        .size(64.0, 64.0)
}
```

`quad_to`, `cubic_to` and `arc` add curves. Because a path is unit-space, a corner drawn with one stretches with the aspect ratio. The built-in shapes also report what kind of shape they are, so a backend can resolve a corner radius against the shorter side and keep the corners circular. The built-in shapes can also morph into one another with `morph_to`; [the animation chapter](../06-advanced/01-animation.md) covers it.

## Elevation: shadows, floating surfaces and cards

### Shadows

`.shadow(..)` takes a `Shadow`: a color, an offset, a blur radius and the **silhouette** of the view that casts it. Backends blur that silhouette themselves, so it must be the caster's own shape (the one you fill or clip with), or the shadow will not follow the outline:

```rust
use waterui::prelude::*;
use waterui::shape::{FixedRoundedRectangle, ShapeExt};

fn tile() -> impl View {
    let shape = FixedRoundedRectangle::new(12.0);
    text!("Tile")
        .padding()
        .background(shape.fill(theme_color::Surface))
        .border_with(Border::new(theme_color::Border, 1.0).corner_radius(12.0))
        .shadow(Shadow::new(
            Color::new(theme_color::Foreground).with_opacity(0.15),
            Vector::new(0.0, 4.0),
            12.0,
            shape,
        ))
}
```

`.border(color, width)` draws square corners, so a rounded tile uses `.border_with(..)` with the same radius in points. The shadow color is built from a token, so it follows both appearances.

### Floating surfaces

`.floating()` promotes any view to a themed, elevated surface: a container fill, a clip radius, ambient and key shadows, and a minimum touch target. The shadows resolve against the interaction state of the control inside, so a style can raise a floating action button while it is hovered. It changes the container and keeps the wrapped control's semantic identity, so a floating button is still a button. `.floating_with(style)` supplies explicit `FloatingStyle` tokens:

```rust
use waterui::prelude::*;
use waterui::style::FloatingStyle;

fn actions() -> impl View {
    let round = FloatingStyle {
        clip_radius: 0.5,
        ..FloatingStyle::default()
    };
    hstack((
        button("Compose").action(|| {}).floating(),
        button("Share").action(|| {}).floating_with(round),
    ))
}
```

A bare `.floating()` reads the `FloatingStyle` in the environment, and a style package or your own `env.install(style)` can provide one. Without one it falls back to `FloatingStyle::default()`, which is itself written in tokens (`Surface` for the container, `Accent` for the content, a 44-point minimum target), so the surface is still correct.

### Cards

`Card` groups related content under an optional title and subtitle. Its chrome comes from a `CardTheme` in the environment, with three `CardStyle`s (`Elevated`, `Filled` and `Outlined`) and `Automatic`, which picks the theme's default:

```rust
use waterui::prelude::*;

fn plan_card() -> impl View {
    card(text!("Unlimited projects and 50 GB of storage"))
        .title("Pro")
        .subtitle("Billed monthly")
        .style(CardStyle::Outlined)
}
```

The Material 3 style package installs a `CardTheme`; neither the Apple backend nor Hydrolysis installs one itself. In an environment with none, a `Card` lays out its title, subtitle and content as a plain stack with no fill, border or shadow, so a card you expect to see chrome on needs a style that provides one.

## Style is an attribute

WaterUI does not have a `CheckboxToggle` next to `SwitchToggle`, or a `RadioPicker` next to `MenuPicker`. A component's semantics are fixed, and its presentation is chosen by an attribute, a theme token, an environment plugin, or the platform default of the backend:

```rust
use waterui::prelude::*;

fn preferences(notify: &Binding<bool>) -> impl View {
    vstack((
        toggle("Notifications", notify),
        Toggle::new("Remember me", notify).checkbox(),
        button("Save").style(ButtonStyle::BorderedProminent).action(|| {}),
    ))
}
```

`Toggle` covers the switch and the checkbox, `Picker` covers menu, radio and segmented, `Button` carries its emphasis in `ButtonStyle`, and `Card` in `CardStyle`. Each backend draws the result in its platform's own idiom: the Apple backend with AppKit and UIKit controls, and Hydrolysis through its style package. If you find yourself wanting a new *type* to get a different look, ask for a new style value instead.

## Material 3 on Hydrolysis

Hydrolysis draws every pixel itself and is independent of any look. How a button or a switch appears comes from a **style package**, and the Material Design 3 package is `hydrolysis-m3`. Every Hydrolysis host that `water` generates (desktop, web and Android) runs with `hydrolysis_m3::Material3::defaults()`, the baseline light and dark roles that follow your color scheme. A Hydrolysis `water preview` uses it too; `--theme material3` is the default and currently the only value (see [the preview chapter](../07-tools/01-preview.md)).

Because views read portable tokens, nothing in your tree names the package. `Accent`, `Surface` and `Foreground` map onto the active Material scheme, and swapping the style re-themes the whole app. Prefer the portable tokens in any view that should also run on the Apple backend.

The style is the third argument of `hydrolysis::run(app, env, style)`, which the generated host calls. `Material3` also builds a scheme from a Material You seed color:

```rust,ignore
use hydrolysis_m3::{Argb, Material3, MaterialColorMode, MaterialColorSource};

// A fixed scheme from a seed, light or dark.
let light = Material3::with_seed(Argb(0xFF6750A4));
let dark = Material3::with_seed_mode(Argb(0xFF6750A4), MaterialColorMode::Dark);

// Paired light/dark schemes from one source, picking one mode.
let source = MaterialColorSource::new(Argb(0xFF6750A4));
let schemes = source.schemes();
let picked = Material3::with_color_schemes(&schemes, MaterialColorMode::Light);
```

The Material-specific roles (`Primary`, `OnPrimary`, `PrimaryContainer`, `SurfaceContainerHighest`, `OnSurfaceVariant` and the rest) live in `hydrolysis_m3::color` and resolve from the installed scheme. Use them only where the design is specifically Material; a view that uses `theme_color` tokens stays portable.

## Where to go next

- [Icons](12-icons.md) covers the icon-set crates; pick one set per app and color its icons with the tokens from this chapter.
- [Overlays](10-overlays.md) covers the sheets and dialogs that sit above your content.
- [Gradients](../05-graphics/06-gradients.md) and [filters](../05-graphics/04-filters.md) take color work beyond flat fills.
- [Accessibility](../06-advanced/05-accessibility.md) covers contrast and the accessibility tree that styled views still need.

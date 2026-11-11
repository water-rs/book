# Library authoring

> **In this chapter, you will:**
>
> - Place a component crate in the right repository and consume the framework between releases
> - Define hookable and leaf views with `configurable!` and `raw_view!`
> - Apply the `Type::new` / free-function constructor split, and accept `IntoText`, `IntoLabel`, `IntoComputed<T>` and `IntoSignalF32`
> - Erase generic selection state with `Mapping<T>` and hide object-safety behind a private `XxxImpl` shim
> - Convey metadata to tooling through the `waterui_meta_` linker-section channel, and test with `waterui-testing`

A WaterUI component crate is an ordinary Rust library that follows a handful of
conventions. Following them is what makes your components compose with the rest
of the ecosystem instead of sitting beside it.

## Where a component crate lives

The framework repository (`water-rs/waterui`) holds the core, the layout, text and
reactive foundations, the shared backend contracts, the Apple and Hydrolysis
backends, the engine Hydrolysis draws through, the components whose realization
is a native primitive on each platform, and the `water` CLI. Everything else is
its own repository under the `water-rs` organization:

- **Self-drawn components** (barcode, chart, particle, SVG, math, Mermaid and
  the other split components) each own their implementation,
  examples, assets, tests, CI and releases.
- **Style packages** such as `hydrolysis-m3` implement the public widget and
  theme contracts, never renderer internals.

A new self-drawn component starts as its own repository, never as a crate added
to the framework tree. It depends on the *published* framework crates and on the
public `waterui-graphics` APIs, never on another repository's checkout layout:

```toml
[package]
name = "waterui-barcode"
edition = "2024"

[features]
default = []
# Rasterizing into a standalone image needs a GPU device, so the capability
# that costs the dependency sits behind a feature.
gpu = ["waterui-graphics/gpu"]

[dependencies]
nami = "0.13"
waterui-core = "0.5.1"
waterui-graphics = { version = "0.5.1", default-features = false }
```

Two habits worth copying. **Depend on the specific crates you use**, not the
`waterui` facade: the facade is for applications, and depending on it from a
component pulls the whole dependency closure (and creates a cycle if the facade
re-exports you). And **keep features granular** with `default-features = false`,
so a consumer that disables `gpu` or targets an embedded device drops your GPU
code with it.

### Consuming an unreleased framework change

Between releases, a split repository consumes the framework's `dev` branch
through `[patch.crates-io]`, by `branch`, never by `rev` or `path`. Every
`waterui-*` crate in the graph has to resolve to the same commit, because a
registry `waterui-core` next to a git one gives you two `View` traits in one
graph, so patch each crate your graph reaches:

```toml
[patch.crates-io]
waterui-core = { git = "https://github.com/water-rs/waterui", branch = "dev" }
waterui-graphics = { git = "https://github.com/water-rs/waterui", branch = "dev" }
waterui-layout = { git = "https://github.com/water-rs/waterui", branch = "dev" }
waterui-testing = { git = "https://github.com/water-rs/waterui", branch = "dev" }
```

`Cargo.lock` records the exact commit; move it with `cargo update -p <crate>`,
never by editing the manifest. The framework consumes you the same way in the
other direction, so neither side waits on the other's release.

### The facade only re-exports

The `waterui` facade is a re-export crate, like `crossbeam`. It re-exports
independently maintained crates (`waterui::barcode`, `waterui::chart`,
`waterui::particle`, ...) by direct `pub use`, never through wrapper types or a
second implementation, so `waterui_barcode::Barcode` and `waterui::barcode::Barcode`
are the same type. Heavyweight components are optional Cargo features of the
facade, and disabling one removes its dependency graph. Dependencies flow
facade, then components, then foundations: a component never depends back on
`waterui`.

## The `configurable!` macro

`configurable!` defines a view that carries a config struct and can be
intercepted by downstream consumers. It lives in `waterui-core`, so a component
crate imports it from there:

```rust,ignore
use waterui_core::{configurable, layout::StretchAxis};

// Default stretch axis (None): the view is sized by its content.
#[derive(Debug)]
pub struct BadgeConfig {
    pub count: u32,
}
configurable!(Badge, BadgeConfig);

// A fixed stretch axis.
#[derive(Debug)]
pub struct GaugeConfig {
    pub value: f64,
}
configurable!(Gauge, GaugeConfig, StretchAxis::Horizontal);

// A stretch axis computed from the config.
#[derive(Debug)]
pub struct ProgressConfig {
    pub circular: bool,
}
configurable!(Progress, ProgressConfig, |config| if config.circular {
    StretchAxis::None
} else {
    StretchAxis::Horizontal
});
```

It generates the wrapper struct, a `NativeView` impl on the config declaring the
stretch axis, `ConfigurableView` on the wrapper, `ViewConfiguration` on the
config, `From<Config>` for the wrapper, and a `View` impl that checks the
environment for a `Hook` before falling through to the backend.

That hook is how a consumer replaces your view globally without forking your
crate. Inside the hook, `config.render()` (from `ViewConfiguration`) rebuilds
the ordinary view: the hook runs in a copy of the environment with itself
removed, so this wraps the default rendering instead of recursing:

```rust
use waterui::component::slider::SliderConfig;
use waterui::prelude::*;
use waterui::view::ViewConfiguration;

pub fn padded_sliders(env: &mut Environment) {
    env.insert_hook(|_env: &Environment, config: SliderConfig| {
        config.render().padding()
    });
}
```

The stretch axis is the third argument, given either statically or as a closure
over the config. An optional `resolve` clause runs before the config reaches
the backend, which is where you fold environment state into the payload it
receives:

```rust,ignore
#[derive(Debug, Clone, Copy, Default)]
pub struct Density(pub f32);

#[derive(Debug)]
pub struct MeterConfig {
    pub value: f64,
    pub density: Density,
}

configurable!(Meter, MeterConfig, StretchAxis::Horizontal, resolve |config, env| {
    MeterConfig { density: env.get::<Density>().copied().unwrap_or_default(), ..config }
});
```

When a hook is installed, it receives the unresolved config and `resolve` does
not run, so `resolve` shapes only the payload the backend realizes. Use it when
the backend needs a value it cannot look up itself. Do not use it to snapshot a
signal: that would freeze a reactive input at build time.

## The `raw_view!` macro

For leaf views with nothing to hook, `raw_view!` implements `NativeView` and
`View` without the `ConfigurableView` and `Hook` machinery. The facade
re-exports it:

```rust
use waterui::prelude::*;
use waterui::raw_view;

pub struct Swatch;
raw_view!(Swatch);

pub struct Rule;
raw_view!(Rule, StretchAxis::Horizontal);

pub fn palette() -> impl View {
    vstack((Swatch, Rule))
}
```

The stretch axis follows the layout contract in [the layout engine
chapter](03-layout-engine.md); backends realize a leaf at the size it reports.

## The constructor split

WaterUI exposes construction two ways, and libraries should match:

- **`Type::new(...)` is the general constructor.** It takes the most general
  shape the component renders and shows up by name in type signatures.
- **Free functions like `button(...)` are the ergonomic entry points.** They
  accept narrower semantic inputs so a string literal lands in the i18n-aware
  text pipeline with correct accessibility defaults.

```rust
use waterui::prelude::*;

pub fn actions() -> impl View {
    // Ergonomic: the literal becomes semantic text with a default a11y label.
    let save = button("Save").action(|| {});

    // General: arbitrary visual content, with its spoken text stated separately.
    let verified = Button::new(Label::new("Verified account", || {
        hstack((text("Account"), text("✓")))
    }));

    vstack((save, verified))
}
```

`Button::new` takes a `Label`, not an open `impl View`, because an unlabeled
control is an inaccessible control. `Label::new(semantic_text, builder)` supplies
arbitrary visual content while keeping the spoken text intact. The content is a
builder (any closure returning a view), because a control may realize its label
more than once, for example when a list row scrolls back into view.

Never add a parallel `Type::custom(...)`. If `Type::new` is not general enough,
widen `Type::new`.

## Flexible input types

### `IntoText` and `IntoLabel`

Use `IntoText` for semantic text and `IntoLabel` for control labels. Both route
`&'static str`, `String`, `Str`, `StyledStr`, `Text` and reactive `Binding<T>` /
`Computed<T>` (where `T: IntoText`) through the i18n-aware pipeline, so
localization and accessibility come along automatically. A `&'static str` is a
localization key; an owned `String` or `Str` is verbatim:

```rust
use waterui::prelude::*;
use waterui::text::IntoText;

pub fn caption(content: impl IntoText) -> Text {
    Text::new(content).font(font::Caption)
}

pub fn captions() -> impl View {
    vstack((caption("Saved"), caption(String::from("Saved"))))
}
```

Reach for a bare `impl View` only when the slot is arbitrary visual composition
rather than a textual label.

### `IntoComputed<T>` and `IntoSignalF32`

For non-textual reactive inputs, accept a signal so callers can pass a constant
or a live source without wrapping anything:

```rust
use waterui::prelude::*;
use waterui::reactive::signal::IntoComputed;

pub fn advanced(content: impl View, shown: impl IntoComputed<bool>) -> impl View {
    content.visible(shown)
}

pub fn settings(show_advanced: Binding<bool>) -> impl View {
    vstack((
        advanced(text("Always shown"), true),
        advanced(text("Proxy settings"), show_advanced),
    ))
}
```

This is not a convenience: an API that takes a plain `bool` where the
underlying state is dynamic forces the caller into a subtree rebuild to change
one value. New public surfaces take signals whenever the value can change.

`f32` inputs are the one exception to the generic bound. Numeric literals
default to `f64`, so `impl IntoComputed<f32>` would make callers write
`0.5_f32`. `IntoSignalF32` coerces any numeric constant or signal to an `f32`
signal at the boundary, and `.opacity(0.5)` just works. The trait lives in
`waterui_core`, and the facade does not re-export it, so a component crate
imports it from there:

```rust,ignore
use waterui_core::{Binding, Computed, IntoSignalF32, SignalExt};

pub fn spacing(value: impl IntoSignalF32 + 'static) -> Computed<f32> {
    value.into_signal_f32().computed()
}

pub fn callers(gap: &Binding<f64>) {
    let _ = spacing(8);           // integer literal
    let _ = spacing(8.0);         // float literal
    let _ = spacing(gap.clone()); // Binding<f64>
}
```

It returns a signal, not an `f32`, which is what makes `.spacing(binding)`
re-lay-out instead of freezing the first value. Do not invent sister traits for
other types; the generic `IntoComputed<T>` covers them.

## Erased configuration with `Mapping<T>`

A backend configuration stores erased selection state, `Binding<Id>` and
`Computed<Vec<..>>` of tagged items, because a backend cannot be generic over
your application's selection type. That erasure is deliberate at the
configuration layer. The *public* constructor stays generic over the app's type
and erases through `Mapping<T>`, which assigns stable `Id`s and maps back:

```rust
use waterui::id::{Id, Mapping};
use waterui::prelude::*;

pub fn erase(selection: &Binding<String>) -> (Mapping<String>, Binding<Id>) {
    let mapping: Mapping<String> = Mapping::new();
    // A `Binding<Id>` that reads and writes the caller's `Binding<String>`.
    let erased = mapping.binding(selection);
    (mapping, erased)
}
```

`Picker::new<T: Ord + Clone + 'static>` does exactly this and stores the result
in `PickerConfig`; `NavigationSplitView::new` and `Tabs::new` follow the same
shape. Writing an `Id` the mapping never issued is a contract violation: a debug
build panics, a release build drops the write. If your authoring type leaks `Id`
into app code, add a generic constructor that erases through `Mapping` instead.

## Object safety stays private

Never degrade a public trait to make it object-safe. Expose the friendliest
signature (`-> impl Future`, `-> impl View`, generic methods), and when you need
dynamic dispatch, add a private object-safe shim trait with a blanket impl and
store `Box<dyn XxxImpl>` behind a public wrapper. `ViewRenderer` in `waterui-core`
is the reference:

```rust,ignore
// Public: implementors write a plain `async fn`.
pub trait CustomViewRenderer: 'static {
    fn render_to_rgba(
        &self,
        view: AnyView,
        request: RenderRequest,
    ) -> impl Future<Output = Result<RenderResult, RenderError>>;
}

// Private shim: the future is pinned and boxed here, behind the erasure boundary.
trait CustomViewRendererImpl: 'static {
    fn render_to_rgba<'a>(
        &'a self,
        view: AnyView,
        request: RenderRequest,
    ) -> Pin<Box<dyn 'a + Future<Output = Result<RenderResult, RenderError>>>>;
}

impl<T: CustomViewRenderer> CustomViewRendererImpl for T { /* Box::pin(..) */ }

// Public wrapper: stores the erased renderer.
pub struct ViewRenderer(Box<dyn CustomViewRendererImpl>);
```

The public trait never mentions boxing, and the wrapper's `new` takes any
`T: CustomViewRenderer`.

## Environment for context passing

`Environment` is a type-indexed store. `store::<K, V>` gives you a keyed slot
when the value type alone is not a unique key, and `query::<K, V>` reads it
back. `store` is a consuming builder:

```rust
use waterui::prelude::*;

pub struct DensitySlot;

pub fn density_of(env: &Environment) -> f32 {
    env.query::<DensitySlot, f32>().copied().unwrap_or(1.0)
}

pub fn configured() -> Environment {
    Environment::new().store::<DensitySlot, f32>(2.0)
}
```

Library views should read what they need from the environment rather than making
callers thread parameters through every function; see [the Environment
chapter](../02-core/03-environment.md).

### The `Plugin` trait

Bundle a library's setup into one installable value. `install` takes `self` by
value; the default implementation just inserts the plugin into the environment,
so a plugin that is only a bag of settings needs no method body:

```rust
use waterui::prelude::*;
use waterui::Plugin;

pub struct Density {
    pub scale: f32,
}

impl Plugin for Density {}

pub fn scale_of(env: &Environment) -> f32 {
    env.get::<Density>().map_or(1.0, |density| density.scale)
}

pub fn app_environment() -> Environment {
    let mut env = Environment::new();
    env.install(Density { scale: 2.0 });
    env
}
```

A plugin that replaces a component installs a hook with `env.insert_hook(..)` in
`install`. See [the plugins chapter](../06-advanced/07-plugins.md).

## Composition patterns

Prefer a function that composes existing modifiers over a new view type:

```rust
use waterui::prelude::*;

pub fn panel(content: impl View) -> impl View {
    content.padding()
}
```

Create a dedicated struct only when the component needs one:

- It owns reactive state exposed as `Binding<T>` inputs.
- It is a native or self-drawn leaf a backend realizes.
- It has enough configuration to justify `configurable!`.
- It must intercept or scope environment values for its subtree.

A component's body keeps the shape of its semantics: never wrap otherwise static
content in a `Dynamic` just because the body branches.

### State belongs to the caller, not the body

Views are consumed by `body()`. State that must survive a rebuild lives in a
`Binding` owned above the view and passed in as a parameter:

```rust
use waterui::prelude::*;

pub fn counter(count: &Binding<i32>) -> impl View {
    vstack((
        text!("Count: {count}"),
        button("+1")
            .action(|State(count): State<Binding<i32>>| *count.get_mut() += 1)
            .state(count),
    ))
}
```

Handlers receive state through typed `State<T>` extractor parameters paired with
`.state(...)` calls. Beyond two or three pieces of state, bundle them into one
`Clone` struct marked `#[state]`, which a handler then takes as a bare
parameter. `text!` reads the binding reactively; calling `.snapshot()` in a
body would read once and never update.

There is no renderer-provided local state slot to reach for, and none may be
introduced: component identity is never inferred from call order. If your
component seems to need one, the state is owned at the wrong level.

## Theming

Backends read theme tokens (`Foreground`, `Background`, `Surface`,
`SurfaceVariant`, `Border`, `Accent`, `AccentForeground`, `AccentContainer`,
`MutedForeground`, `TertiaryForeground`, `DisabledForeground`, `Tertiary`,
`TertiaryContainer`, `SelectionContainer`, `SelectionForeground`,
`UnemphasizedSelection`, `Error`, `ErrorForeground`) instead of hard-coding
platform colors. Do the same: a component with no extra modifiers should produce
platform-correct output, and one that must reach into a backend to get defaults
right exposes a backend bug, fixed in the backend. See [the styling
chapter](../03-ui/11-styling.md).

## Conveying metadata to tooling

When a proc macro of yours must tell the `water` CLI (or a test, or a packager)
something about the compiled crate, it does so through one channel: a record in
a dedicated linker section of the compiled artifact. Do not invent a per-feature
side channel such as a generated file, an environment variable or source
scraping, and do not run external processes in a macro: rust-analyzer and every
`cargo check` re-expand macros, so expansion stays a pure read.

The record format is the whole contract, and it lives in `waterui-meta`, a
zero-dependency `no_std` crate holding the section name, the record builders and
the section walker. The macro emits a `static` whose item name starts with
`waterui_meta_`, parks it in the `.wmeta` section (`__DATA,__wmeta` in an Apple
debug build, `__DWARF,__wmeta,regular,debug` in an Apple release build), and
makes its bytes a self-describing record: the item's name, NUL, the payload,
NUL. A record is identified by its own name field rather than the symbol table,
because a linked MSVC image keeps no COFF symbol table.

The `dir_static!` macro applies the right section and retention attributes for
the invoking crate's target and profile, and `dir_entry` builds the record bytes
in const context:

```rust,ignore
use waterui_meta::{dir_entry, dir_static};

const NAME: &[u8] = b"waterui_meta_mycrate_widget";
const PAYLOAD: &[u8] = br#"{"kind":"widget"}"#;
const LEN: usize = NAME.len() + PAYLOAD.len() + 2;

dir_static! {
    #[allow(non_upper_case_globals)]
    static waterui_meta_mycrate_widget: [u8; LEN] = dir_entry::<LEN>(NAME, PAYLOAD);
}
```

Reading is the CLI's side: it walks the section with `dir_records`, which yields
each record's `name` and `payload`, from the project library's artifact of the
build it just ran. Keep these rules in mind when you emit:

- The record is `#[used]` on every target, so a release-consumed record is not
  dropped by a staticlib's internal linkage.
- It is never `#[no_mangle]` or `#[export_name]`, which would put it in a
  cdylib's export table. Exports are for entry points a host calls.
- A debug-only consumer keeps `#[cfg(debug_assertions)]` so shipped binaries
  carry none of its records; a record whose consumer also runs on release builds
  omits the gate.

## Testing

### Accessibility-first component tests

`waterui-testing` mounts a view headlessly and queries the accessibility tree it
produces. Every UI component must produce a meaningful accessibility tree, and one
that cannot be queried this way has an accessibility bug, not an untestable
design. `#[waterui::test(view_fn)]` expands to a plain `#[test]`, so these run
under the normal harness, and nextest runs each in its own process:

```rust
use waterui::prelude::*;
use waterui_testing::{Role, SemanticApp};

fn login_view() -> impl View {
    vstack((text("Welcome back"), button("Login").action(|| {})))
}

#[waterui::test(login_view)]
fn login_smoke(app: &mut SemanticApp) {
    app.query().role(Role::LABEL).label("Welcome back").assert_exists();
    app.query().role(Role::BUTTON).label("Login").assert_exists();
}
# fn main() {}
```

A test that needs pixels, bounds or frame timing carries a style package through
`theme =` and mounts the rendered runtime with `offscreen`; the animation clock
is virtual, so advance it by pumping frames (`OffscreenApp::pump_for`) and wait
on conditions (`wait_for_existence`), never with `std::thread::sleep`. Because
`waterui-testing` depends on `hydrolysis`, your crate adds it as a
dev-dependency, ideally in a separate test package so the component's own test
run does not build and mount a whole framework, as the barcode repository does
with its `tests/mounted` package. A test of a style package's own metrics,
rather than of your component, belongs in that style package's repository. See
[the testing chapter](../07-tools/03-testing.md).

### GPU content and scenes

A self-drawn component draws through `waterui-graphics`, and its output is
tested without a window. Scene content renders through
`OffscreenRenderer::render`; a canvas `Drawing`, which is what a component drawn
on a `Canvas` (the barcode, for one) records for its standalone output, renders
through `OffscreenRenderer::render_drawing(&drawing, size)`; owned `GpuContent` renders through
`GpuRuntime::render_content` on a runtime from the async `GpuRuntime::new()`.
Both produce an `OffscreenImage` you can write out with `save_png` and read with
your own eyes. These types need `waterui-graphics`'s `gpu` feature:

```rust,ignore
use waterui_graphics::{OffscreenRenderer, OffscreenSize, cherenkov_gpu::Gpu};

let size = OffscreenSize::try_from_pixels(768, 768).expect("valid output size");
let gpu = OffscreenRenderer::<Gpu>::new().expect("a working GPU renderer");
let image = gpu.render(&mut my_scene_content, size, 1.0).expect("frame renders");
image.save_png("qr.png").expect("png written");
```

Judge rendered output by visual equivalence with a tolerance, not by byte or
pixel identity, and do not substitute heuristics such as changed-pixel counts
for looking at the image. Diagnostics inside your crate go through `tracing`,
never `println!`.

## Public API shape

Re-export the constructors, the view types and the style enums at the crate
root. A config stays in its public module rather than the root: it must be
public, because hooks and backends receive it, but its fields stay private
behind getters. A public struct is either fixed-shape plain data with every
field public, or keeps its fields private behind `Type::new` and getters.
`#[non_exhaustive]` on a struct with public fields is banned, because it exposes
the representation while still forcing a constructor.

```rust,ignore
pub mod button;
pub use button::{Button, ButtonStyle, button};
// `button::ButtonConfig` is public in its module, with private fields
// behind `label()`, `action()`, `style()` and `size()`.
```

When a type must be public for macro expansion but has no business in the docs,
mark it `#[doc(hidden)]`. Style is an attribute, never a parallel type: a
checkbox is a `Toggle` with a style, not a `CheckboxToggle`.

## What's next

The [next chapter](../09-philosophy.md) steps back from the code to the design
principles these conventions come from.

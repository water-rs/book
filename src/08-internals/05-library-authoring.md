# Library authoring

> **In this chapter, you will:**
>
> - Use `configurable!` and `raw_view!` to define hookable and leaf views
> - Apply the `Type::new` / free-function constructor split WaterUI uses everywhere
> - Accept `IntoText`, `IntoLabel`, `IntoSignal<T>`, and `IntoComputed<T>` in your APIs
> - Pass context through the `Environment` and the `Plugin` trait
> - Test a component through its accessibility tree

A WaterUI component crate is an ordinary Rust library that follows a handful of
conventions. Following them is what makes your components compose with the rest
of the ecosystem instead of sitting beside it.

## Where a component crate lives

In this repository, component crates sit under a domain folder in `components/`:
`foundation` (layout, text, controls, form, navigation, shape, icon), `visual`,
`multimedia`, `data`, `codes`, `assets`, `effects`, `devtools`, and `platform`.
Your own crate follows the same shape as any of them:

```toml
[package]
name = "myco-waterui-widgets"
edition = "2024"
rust-version = "1.95"

[dependencies]
waterui-core.workspace = true
waterui-layout.workspace = true
waterui-text.workspace = true
nami.workspace = true

[features]
default = []

[lints]
workspace = true
```

Two habits worth copying. **Depend on the specific component crates you use**,
not the `waterui` facade — that is what keeps unused features out of a consuming
app's artifact. And **keep features granular**, so a consumer disabling `gpu` or
building for an embedded target drops your GPU code with it.

## The `configurable!` macro

`configurable!` defines a view that carries a config struct and can be
intercepted by downstream consumers:

```rust,ignore
configurable!(Button, ButtonConfig);
configurable!(Slider, SliderConfig, StretchAxis::Horizontal);
configurable!(Progress, ProgressConfig, |config| match config.style {
    ProgressStyle::Linear => StretchAxis::Horizontal,
    ProgressStyle::Circular => StretchAxis::None,
});
```

It generates the wrapper struct, a `NativeView` impl on the config declaring the
stretch axis, `ConfigurableView` on the wrapper, `ViewConfiguration` on the
config, and a `View` impl that checks the environment for a hook before falling
through to native rendering.

That hook is how a consumer replaces your view globally without forking your
crate:

```rust,ignore
let mut env = Environment::new();
env.insert_hook(|env: &Environment, config: ButtonConfig| {
    custom_button(config.label, config.action)
});
```

### Stretch axis and environment resolution

The third argument declares the stretch axis, either statically or from the
config:

```rust,ignore
configurable!(MyView, MyConfig);                          // StretchAxis::None
configurable!(MyView, MyConfig, StretchAxis::Horizontal); // always horizontal
configurable!(MyView, MyConfig, |config| {
    if config.is_expanded { StretchAxis::Both } else { StretchAxis::None }
});
```

An optional `resolve` clause runs before the config reaches `Native`, which is
where you fold environment state into the payload the backend receives:

```rust,ignore
configurable!(MyView, MyConfig, StretchAxis::Horizontal, resolve |config, env| {
    MyConfig { density: env.get::<Density>().copied().unwrap_or_default(), ..config }
});
```

Use `resolve` when the backend needs a value it cannot look up itself. Do not
use it to snapshot a signal — that would freeze a reactive input at build time.

## The `raw_view!` macro

For leaf views with nothing to hook:

```rust,ignore
raw_view!(Divider, StretchAxis::CrossAxis);
raw_view!(Spacer, StretchAxis::MainAxis);
raw_view!(Image);  // StretchAxis::None
```

This implements `NativeView` and `View` without the `ConfigurableView`/`Hook`
machinery.

## The constructor split

WaterUI exposes construction two ways, and libraries should match:

- **`Type::new(...)` is the general constructor.** It takes the most general
  shape the component can render.
- **Free functions like `button(...)` are the ergonomic entry points.** They
  accept narrower semantic inputs so a string literal lands in the i18n-aware
  text pipeline with correct accessibility defaults.

```rust,ignore
// Ergonomic: the literal becomes semantic text with a default a11y label.
let save = button("Save").action(|| { /* ... */ });

// General: arbitrary visual content, with its spoken text stated separately.
let verified = Button::new(Label::new(
    "Verified account",
    hstack((text("Account"), verification_badge)),
));
```

Note what `Button::new` takes: a `Label`, not an open `impl View`. A control's
label is never optional, because an unlabelled control is an inaccessible
control. `Label::new(semantic_text, content)` is how you supply arbitrary visual
content while keeping the spoken text intact; `Label`'s semantic-only builders
(`icon`, `system_icon`, `leading`, `trailing`, `spacing`, `font`) panic on
custom-content labels rather than silently dropping the decoration.

Do not add a parallel `Type::custom(...)`. If `Type::new` is not general enough,
widen `Type::new`.

## Flexible input types

### `IntoText` and `IntoLabel`

Use `IntoText` for semantic text and `IntoLabel` for control labels. Both route
literals, `String`, `Str`, `StyledStr`, and reactive `Computed<T>` through the
i18n-aware pipeline, so localization and accessibility come along automatically:

```rust,ignore
use waterui_text::{IntoText, Text, font::Caption};

pub fn caption(content: impl IntoText) -> Text {
    Text::new(content).font(Caption)
}

caption("Saved");
caption(String::from("Saved"));
caption(text!("Saved at {now}"));
```

Reach for a bare `impl View` only when the slot really is arbitrary visual
composition rather than a textual label.

### `IntoSignal<T>` and `IntoComputed<T>`

For non-textual reactive inputs, accept a signal so callers can pass a constant
or a live source without wrapping anything:

```rust,ignore
pub fn opacity(value: impl IntoComputed<f32>) -> Opacity {
    Opacity { value: value.into_computed() }
}

opacity(0.5);            // constant
opacity(my_binding);     // Binding<f32>
opacity(computed_value); // Computed<f32>
```

This is not a convenience: an API that takes a plain `f32` where the underlying
state is dynamic forces the caller into a subtree rebuild to change one number.
New public surfaces take signals whenever the value can change.

### `IntoSignalF32`

`IntoSignalF32` is the numeric-literal-friendly variant. It converts any signal
whose output is a Rust numeric type into a signal of `f32`:

```rust,ignore
use waterui_core::IntoSignalF32;

pub fn spacing(value: impl IntoSignalF32 + 'static) -> Computed<f32> {
    value.into_signal_f32().computed()
}

spacing(8);          // i32 literal
spacing(8.0);        // f32 literal
spacing(my_binding); // Binding<f64>
```

It returns a *signal*, not an `f32` — that is what makes `.spacing(binding)`
re-lay-out instead of freezing the first value.

## Environment for context passing

`Environment` is a type-indexed store. `Store<K, V>` gives you a keyed slot when
the value type alone is not a unique key:

```rust,ignore
use waterui_core::{Environment, env::use_env};

pub struct AccentSlot;

let env = Environment::new().store::<AccentSlot, Color>(Color::blue());

// Read it back inside a view.
pub fn themed_button() -> impl View {
    use_env(|env: Environment| {
        let color = env.query::<AccentSlot, Color>().cloned().unwrap_or(Color::blue());
        button("Tap me").foreground(color)
    })
}
```

`store` is a consuming builder on `Environment`, and `use_env`'s closure takes
values *extracted* from the environment — `Environment` itself implements
`Extractor`, so an owned `Environment` parameter works, and so does a tuple of
extractable types:

```rust,ignore
let view = use_env(|(nav, db): (Navigator<Route>, Database)| {
    button("Load").action(move || { /* ... */ })
});
```

Library views should extract what they need rather than making callers thread
parameters through every function.

### The `Plugin` trait

Bundle a library's setup into one installable value:

```rust,ignore
use waterui_core::{Environment, plugin::Plugin};

pub struct MyLibraryPlugin {
    pub theme: MyTheme,
}

impl Plugin for MyLibraryPlugin {
    fn install(self, env: &mut Environment) {
        env.insert(self.theme);
        env.insert_hook(|env: &Environment, config: ButtonConfig| {
            custom_button(config)
        });
    }
}

let mut env = Environment::new();
env.install(MyLibraryPlugin { theme: MyTheme::default() });
```

`install` takes `self` by value. The default implementation just inserts the
plugin into the environment, so a plugin that is only a bag of settings needs no
method body at all.

## Composition patterns

Prefer a function that composes existing modifiers over a new view type:

```rust,ignore
// Prefer this.
pub fn panel(content: impl View) -> impl View {
    content.padding_with(EdgeInsets::all(16.0)).floating()
}
```

`.floating()` promotes a view to a themed elevated surface — container color,
clip radius, and both shadows resolved from `FloatingStyle` in the environment.
It panics if those tokens are absent, which is the fast-fail you want: a missing
theme is a setup bug, not a reason to render an unstyled box.

Create a dedicated struct only when the component needs one:

- It owns reactive state exposed as `Binding<T>` inputs.
- It participates in FFI as a native view.
- It has enough configuration to justify `configurable!`.
- It must intercept or scope environment values for its subtree.

### State belongs to the caller, not the body

Views are consumed by `body()`. State that must survive a rebuild lives in a
`Binding` owned above the view, passed in as a parameter:

```rust,ignore
pub fn counter(count: &Binding<i32>) -> impl View {
    vstack((
        text!("Count: {count}"),
        button("+1")
            .action(|State(count): State<Binding<i32>>| *count.get_mut() += 1)
            .state(count),
    ))
}
```

Two things this example is showing. Handlers receive state through typed
`State<T>` extractor parameters paired with `.state(...)` calls, one per
injected value — bundle them into a single `#[derive(Clone)]` struct once you
reach four. And `text!` reads the binding reactively; calling `.get()` inside a
body would read once and never update.

There is no renderer-provided local state slot to reach for. Component identity
is not inferred from call order, so if your component seems to need one, the
state is being owned at the wrong level.

## Theming

Backends resolve `Foreground`, `Background`, `Surface`, `SurfaceVariant`,
`Border`, `Accent`, `AccentContainer`, `AccentForeground`, `MutedForeground`,
`Tertiary`, and `TertiaryContainer` from the environment. Read those tokens
instead of naming concrete colors, and your component adapts to light/dark mode
and to whatever theme the host app installed.

An unresolvable token panics with the slot name rather than rendering
transparent, so a missing token surfaces at first render instead of as an
invisible widget.

## Testing

### Accessibility-first component tests

`waterui-testing` renders a view headlessly and queries the accessibility tree
it produces. `#[waterui::test(view_fn)]` expands to a plain `#[test]`, so these
run under the normal harness:

```rust,ignore
use waterui::ViewExt as _;
use waterui::accessibility::AccessibilityRole;
use waterui_testing::{Role, SemanticApp};

fn glyph_view() -> impl waterui::View {
    IconGlyph::new('\u{2605}', "Helvetica")
        .with_size(24.0)
        .a11y_role(AccessibilityRole::Image)
        .a11y_label("Glyph icon")
}

#[waterui::test(glyph_view)]
fn glyph_exposes_accessibility_image(app: &mut SemanticApp) {
    app.query().role(Role::IMAGE).label("Glyph icon").assert_exists();
}
```

This is simultaneously an interaction test and an accessibility test. A
component that cannot be queried this way has an accessibility bug, not an
untestable design.

Controls spawn local tasks internally, so a plain `#[test]` that constructs one
directly needs a local executor installed first.

### Snapshots and previews

`TestHost::capture_snapshot` writes PNG artifacts under the canonical
`<suite>/<case>/<stage>.png` layout when `WATERUI_TEST_ARTIFACTS_DIR` is set.

For a visual check during development, give each public component a `#[preview]`
function:

```rust,ignore
#[preview]
fn button_styles() -> impl View {
    vstack((
        button("Automatic"),
        button("Prominent").style(ButtonStyle::BorderedProminent),
        button("Plain").style(ButtonStyle::Plain),
    ))
    .spacing(8.0)
}
```

```bash
water preview button_styles --platform macos --path ./app --output button.png
```

Preview symbols are `waterui_preview_<crate_name>_<function_name>`, so names
must be unique within a crate.

## Public API shape

Export the constructors, the view types, and the style enums; keep configs and
internals private:

```rust,ignore
pub use button::{Button, ButtonStyle, button};
// ButtonConfig and friends stay crate-private.
```

When a type must be public for macro expansion but has no business in the docs,
mark it `#[doc(hidden)]`.

One rule to hold onto: never degrade a public trait to make it object-safe.
Expose the friendliest signature — `-> impl Future`, `-> impl View`, generic
methods — and if you need dynamic dispatch internally, add a private object-safe
shim trait with a blanket impl and store `Box<dyn XxxImpl>` behind a public
wrapper. `CustomViewRenderer` in `waterui-core` is the reference: implementors
write a plain `async fn render_to_rgba`, and the boxing lives out of sight
behind `ViewRenderer`.

## What's next

The [next chapter](../09-philosophy.md) steps back from the code to the design
principles these conventions come from.

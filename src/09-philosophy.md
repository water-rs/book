# Philosophy

> **In this chapter, you will:**
>
> - Learn what "native" means in WaterUI, and what it deliberately excludes
> - See how a component's semantics are defined once and realized per platform
> - Understand why style is an attribute, and why defaults belong to the framework
> - Learn the reactivity rules that shape every API, and why layout is frozen
> - Read the principles a contribution is measured against

These rules are constraints on every WaterUI feature, refactor, and review. They
override convenience, and they explain why much of the API looks the way it does.
The source of truth is the design-principles section of the framework's
`AGENTS.md`; this chapter explains the reasoning.

## Style is an attribute, not a component

`Toggle` covers switch and checkbox: `.style(ToggleStyle::Checkbox)` picks one,
and the default, `ToggleStyle::Automatic`, is a checkbox on macOS and a switch
elsewhere. `Picker` covers menu, radio, and segmented through
`.style(PickerStyle::...)`. `List` has no style method at all: inside a
`NavigationSplitView` sidebar it takes the platform's sidebar presentation, and
its sections get the platform's section chrome, inset-grouped on iOS. You
choose the presentation with an attribute (`.style(...)`), theme tokens, an
environment plugin, or the backend's platform default.

There is no `CheckboxToggle` or `GroupedList`, and there will not be. Semantic
identity is fixed; visual presentation is a property of the surrounding context.
See [Styling](03-ui/11-styling.md) for the mechanics.

## Every backend realizes a component with its platform's look

A component's semantics and API are defined once, in Rust. Each backend then
realizes that definition in its own way:

- **Apple** realizes components with AppKit and UIKit, driven through `objc2`.
  Where the platform's control carries the whole semantic, the backend uses that
  control. Where it does not, the backend builds a composition of AppKit or UIKit
  views, written in Rust, inside the backend.
- **Hydrolysis**, Android included, draws the component itself, through a style
  package that supplies the platform's look.

A shared composition, built from `vstack`, `hstack`, `padding`, and theme tokens
and rendered the same way by every backend, is the exception, reserved for a
short list of simple containers. `Card` is one: its body is a `vstack` of title,
subtitle, and content, padded and decorated from the `CardTheme` in the
environment. Everything else, `Badge` included, is a view each backend realizes.

A control that is missing, or that lacks part of a semantic, is never a reason to
drop the feature on a platform. The backend builds it. This is a change from
older versions of the framework, which composed existing components in Rust
wherever possible because every native component needed a hand-written FFI
binding. The Apple backend now wraps UIKit and AppKit directly, so Rust composes
native views freely, and the Android backend is self-drawn by Hydrolysis rather
than a set of View bindings. See [Backends](08-internals/04-backends.md) for how
each one is built.

## Native means platform-coupled

A native implementation projects WaterUI semantics directly into the platform's
canonical object model, lifecycle, accessibility, input, graphics, or media
pipeline. It can come from the OS or from an official extension package
inseparable from the platform.

A package is *not* native when it supplies a largely self-contained engine or
runtime that owns the domain, is meaningfully portable, and substantially expands
the dependency closure. Being preinstalled is not the test. Under this
definition ExoPlayer/Media3, WaterKit, Zenwave, Hydrolysis, FFmpeg, GStreamer,
Flutter, and React Native are all *not* native, however useful they are.

Layers are classified independently. Native controls, decoders, surfaces, or
services around an application-owned playback or rendering engine do not make
that engine native. `NativeView` is an internal backend-leaf marker; it is not
evidence that a realization is native.

## Apple bridges native; Hydrolysis draws

The Apple backend bridges AppKit and UIKit. Hydrolysis is the self-drawn
realization, and it is how Android is supported. Neither is a rescue for the
other.

Content that is drawn rather than controlled, such as particle systems and QR
codes, is self-drawn on every backend. It is never handed to a third-party
parallel engine and called native.

The self-drawn realization is a deliberate backend, never a runtime fallback. A
failed native bridge is an error to fix; it must not silently switch
realizations.

Two components show where the line falls:

- **Video** uses AVPlayer and AVKit on Apple platforms, and the browser's
  `<video>` element on the web where the GPU video path is not built. Every other
  platform uses the WaterKit and GPU-surface player, without ExoPlayer or Media3.
  Native controls, codecs, protected surfaces, media sessions, and output devices
  may still serve as layers around that player. See [Media](04-rich/01-media.md).
- **Maps** use MapKit on Apple platforms. Platforms without a suitable map
  primitive use WaterUI's MapLibre-style vector realization on Cherenkov and
  wgpu. A bundled portable map engine is not a native map. See
  [Maps](04-rich/02-maps.md).

The trade-off is that pixel-identical rendering across platforms is not a goal. A
button looks like a platform button on each platform. For the rendering pipeline
behind the self-drawn path, see [Rendering](08-internals/01-rendering.md).

## Bridges stay proportional

A bridge makes reachable only the platform code and narrow support code that the
selected WaterUI features need. Hiding a complete third-party framework or engine
behind FFI, reflection, service registration, umbrella dependencies, or broad
keep rules roots the whole dependency graph. It defeats R8, linker dead
stripping, and Cargo feature pruning, and every app pays for it.

An unused feature removes its Rust code, platform code, resources, and transitive
dependencies from the package. Backend dependencies are feature-granular, and a
new runtime dependency requires a measured before-and-after release-artifact size
on every affected packaging format.

## Default appearance is the framework's job

Backends read theme tokens rather than hard-coding platform colors. The tokens
include `Foreground`, `Background`, `Surface`, `SurfaceVariant`, `Border`,
`Accent`, `MutedForeground`, `TertiaryForeground`, `DisabledForeground`,
`AccentForeground`, `AccentContainer`, `Tertiary`, `TertiaryContainer`,
`SelectionContainer`, `SelectionForeground`, `UnemphasizedSelection`, `Error`,
and `ErrorForeground`, all in `waterui::theme::color`. A component's realization
reads the token rather than hard-coding `.label`, `.systemBackground`, or
`NSColor.windowBackgroundColor`; the platform's own colors enter through the
theme the backend installs into the environment.

The test is simple: view code with no extra modifiers must produce
platform-correct output. If app code has to reach into a backend to make defaults
right, that exposes a backend bug, and it is fixed in the backend. See
[Styling](03-ui/11-styling.md) for using tokens and
[Environment](02-core/03-environment.md) for how they flow down the tree.

## Unsupported means only the platform could supply it

A capability is explicitly unsupported on a platform only when it depends on an
asset or service that the platform alone can supply and that the platform does
not have. SF Symbols is the example: Apple platforms ship them, and Android has
no OS icon catalog. Portable code depends on an icon-set crate such as
`waterui-icons-lucide`, `waterui-icons-material-icon`, or
`waterui-icons-fontawesome7`, and bundling a Material font and calling it
"system" is the dishonest alternative. See [Icons](03-ui/12-icons.md).

This is a narrow exception. A missing control, or a control lacking part of a
semantic, is never such a case: the backend builds it. Each genuinely
unsupported capability is documented, never hidden behind a fallback.

## Fine-grained reactivity

WaterUI uses precise per-`Binding` and per-`Computed` updates. There is no
virtual DOM, no tree diff, and no reconciliation pass:

```text
Binding<i32> changes from 0 to 1
  |
  v
Computed<Str> = "Count: 1"   (only this recomputes)
  |
  v
the label's text property    (only this property updates)
```

Update cost is proportional to the number of affected signals, not to the size of
the tree. No other widget is touched. Collections report precise changes in the
same way, so a list never diffs its old rows against new ones: a row's identity
is its item's id (for `Identifiable` items) or its index, never something
inferred by comparing trees.

This is a hard constraint on API design. An API that would force a structural
recompute to change one value is rejected. APIs over dynamic state accept
signals (`impl IntoComputed<T>`, `impl Signal<Output = T>`, or `Binding<T>`) so
callers never need `watch(...)`.

`Dynamic::watch` is the escape hatch, not the tool. It replaces the watched
subtree and discards state owned inside it. The direct replacements are:

```rust,ignore
text!("{status}")                                // reactive text
Photo::new(url).blur(blur.clone())               // reactive value
Lazy::for_each(rows.clone(), row_view)           // dynamic set of views
```

`Photo` (`waterui::media::Photo`) requires the `media` feature, and `.blur`
requires the `effects` feature, which the default `gpu` feature turns on.

Reading a value with `.snapshot()` inside a view body freezes it at build time,
so it does not belong there. The patterns are covered in
[Reactive state](02-core/02-reactive.md).

### No React-style state slots

Component identity is never inferred from body call order. There are no
renderer-provided local state slots, no hook-like storage, no body-position keys,
and no cursor-indexed state. A backend is never taught a slot mechanism to
accommodate a component; the component is refactored instead.

Mutable UI state is an explicit `Binding`, `Computed`, or `impl Signal`, owned at
the right semantic level and passed through the API. Views are consumed rather
than retained: `View::body(self, env)` takes `self` by value, so a view struct is
moved when its body is evaluated. A component recreated by `when(...)` or
`watch(...)` loses its instance state, and that is correct, since a new instance
is being initialized. If preserving state across a rebuild seems necessary, the
state is owned at the wrong level. See [The View trait](02-core/01-view.md).

## Layout is frozen

The layout system is frozen as of 0.5.0. That covers the proposal protocol,
`StretchAxis`, stack sizing and allocation, every container in `waterui-layout`,
and the leaf contracts backends implement. Layout and gestures behave
identically on every backend; typesetting (shaping, line breaking, line metrics)
belongs to the text leaf, and each platform decides it.

The framework's `docs/layout-spec.md` describes these semantics normatively. A
difference between that document and the code is a bug in the code, and a backend
that deviates is fixed in the backend. Changing the semantics themselves is a
major-version decision, not a pull-request detail. See
[Layout](03-ui/02-layout.md) and [The layout engine](08-internals/03-layout-engine.md).

## Fail fast

An unexpected state surfaces early with a clear message rather than degrading
silently into a plausible-looking default. A contract violation panics in a
debug build and is logged through `tracing` in a release build, because the
framework runs inside your application and must not take it down casually. A
color token that no backend or theme installed is one example: a debug build
panics naming the token, and a release build logs that error and draws opaque
black. Tooling and tests crash outright.

Silent fallbacks hide the bug and move the failure somewhere harder to diagnose.
See [Error handling](06-advanced/04-error-handling.md).

## Signals, not streams

Reactive streams model sequences of events over time; signals model the current
value of a piece of state. UI wants the latter. A text field always has a current
value, a label always shows current text, and a subscriber connecting late needs
the value now rather than a replay.

`Binding<T>` is state plus change notification, and `Computed<T>` is a derived
value. Every signal answers `snapshot()` with its current value and `watch(...)`
with a subscription to the changes that follow.

## Rust all the way

Application logic, UI composition, state, and layout algorithms are all Rust, and
so are the backends, apart from the thin native host each keeps beside its crate
(a Swift package, a Kotlin host library). This buys memory safety without a
garbage collector and one language for business logic and UI. The cost is the boundary with the
platform, which each backend keeps narrow.

## The water metaphor

Water takes the shape of its container without changing what it is. The same view
tree flows through the Apple backend and Hydrolysis unchanged, and each gives it
a local shape. The API follows: `StretchAxis::MainAxis` means "expand along
whatever axis the parent uses," `Foreground` is a slot rather than a hex value,
and a body font resolves to the platform's own text face depending on where it
lands.

## Principles for contributors

1. **Style is an attribute.** Add a variant to an existing semantic component
   rather than a parallel type.
2. **Realize per backend.** Define semantics once in Rust; Apple composes or
   wraps AppKit and UIKit, Hydrolysis draws through style packages. A shared
   composition, such as `Card`, is the rare exception.
3. **Keep bridges proportional.** An unused feature removes its code,
   resources, and dependencies from the package.
4. **Defaults come from theme tokens.** Fix a wrong default in the backend, not
   in view code.
5. **Signals, never slots.** Accept `impl IntoComputed<T>` or
   `impl Signal<Output = T>`; own state explicitly.
6. **Type safety over runtime checks.** Encode invariants in the type system;
   prefer generics and traits over enums and type erasure.
7. **Never degrade a public trait for object safety.** Expose the friendliest
   signature and erase types privately behind a shim.
8. **Fail fast.** A contract violation panics in a debug build and is logged in
   a release build; it never hides behind a silent default.
9. **Foundations change deliberately.** `waterui-core`, the reactivity,
   animation, and layout primitives, and the shared backend contracts change
   only with the maintainer's explicit approval of that change.

To put these principles to work in your own crate, continue with
[Library authoring](08-internals/05-library-authoring.md).

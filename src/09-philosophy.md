# Philosophy

> **In this chapter, you will:**
>
> - Learn what "native" means in WaterUI, and what it deliberately excludes
> - See why style is an attribute rather than a separate component type
> - Understand the reactivity rules that shape every API in the framework
> - Read the principles a contribution is measured against

The rules below are constraints on every WaterUI feature, refactor, and review.
They override convenience, and they explain why a lot of the API looks the way it
does.

## Native means platform-coupled

A native realization projects WaterUI semantics into the target platform's own
object model, lifecycle, accessibility, input, graphics, or media pipeline. It
can come from an OS framework or from an official extension inseparable from that
platform — Android's View-based Material Components qualify, because they are
coupled to Android's view, resource, accessibility, and graphics pipelines.

A package is *not* native when it ships a largely self-contained engine that owns
the domain instead of bridging into the platform, is portable to other platforms,
and substantially expands the dependency closure. Being preinstalled is not the
test. Under this definition ExoPlayer/Media3, Flutter, React Native, FFmpeg, and
WaterUI's own Hydrolysis and Dew are all *not* native implementations, however
useful they are.

Layers are classified independently. Native controls, decoders, surfaces, or
media sessions around an application-owned playback engine do not make that
engine native.

### Bridge native first, then draw

Every semantic component gets a native bridge on each platform that has a
suitable primitive. Where a platform has none, it goes straight to the shared
self-drawn realization — never to a third-party parallel engine relabelled as
native.

The self-drawn path is a deliberate backend, not a rescue. A failed native
bridge is an error to fix; it must not silently switch realizations at runtime.
Some components have no platform primitive anywhere and therefore start
self-drawn: particle systems and QR codes, for instance.

The trade-off is that pixel-identical rendering across platforms is not a goal.
A button looks like an iOS button on iOS and a Material button on Android. If you
need identical pixels everywhere, or you are on a target with no native widget
set, that is what the self-drawn renderers are for.

### Bridges must stay proportional

A bridge may only make reachable the platform code the selected WaterUI features
actually need. Hiding a complete third-party framework behind FFI, reflection,
service registration, or broad keep rules defeats R8, linker dead-stripping, and
Cargo feature pruning, and every app pays for it. Unused features must drop their
Rust code, platform code, resources, and transitive dependencies from the
packaged artifact.

## Style is an attribute, not a component

`Toggle` covers switch and checkbox. `Picker` covers menu, radio, and wheel.
`List` covers plain, inset-grouped, and sidebar. You choose the presentation with
an attribute — `.style(...)`, theme tokens, an environment plugin, or the
backend's platform default.

There is no `CheckboxToggle` or `GroupedList`, and there will not be. Semantic
identity is fixed; visual presentation is a property of the surrounding context.

## Compose in Rust before binding native

Only a widget backed by a real platform primitive that cannot be expressed by
composing existing primitives belongs on the FFI. `Form`, `Card`, `Badge`,
`LabeledContent`, and `GroupBox` are Rust-side composers built from `vstack`,
`hstack`, `padding`, and theme tokens; they ship zero new C-ABI types.

Adding a new FFI entry point requires evidence that no Rust-side composition
produces the same result. Each one is a surface every backend must implement
forever.

## Fine-grained reactivity

WaterUI uses precise per-`Binding`/`Computed` updates. There is no virtual DOM,
no tree diff, and no reconciliation pass:

```text
Binding<i32> changes from 0 to 1
  |
  v
Computed<Str> = "Count: 1"   (only this recomputes)
  |
  v
UILabel.text = "Count: 1"    (only this property updates)
```

Update cost is proportional to the number of affected signals, not to the size of
the tree. No other widget is touched, and no identity heuristics (keys, indices)
are needed to work out what stayed the same.

This is a hard constraint on API design, not just an implementation detail. An
API that would force a structural recompute to change one text value is rejected.
New surfaces accept `impl IntoComputed<T>`, `impl Signal<Output = T>`, or
`Binding<T>` whenever the underlying state can change.

`Dynamic::watch` is the exception, not the tool. It replaces the watched subtree
and discards state owned inside it. The replacements are direct:

```rust,ignore
text!("{status}")                                // reactive text
Photo::new(url).blur(blur.clone())               // reactive value
ForEach::new(rows.clone(), |row| row_view(row))  // dynamic set of views
```

### No React-style state slots

Component identity is never inferred from body call order. There are no
renderer-provided local state slots, no hook-like storage, no body-position keys.
Mutable UI state is an explicit `Binding`/`Computed` owned at the correct
semantic level and passed through the API.

Views are consumed rather than retained: `View::body(self, env)` takes `self` by
value, so a view struct is moved when its body is evaluated. A component
recreated by `when(...)` or `watch(...)` loses its instance state, and that is
correct — a new instance is being initialized. If preserving state across a
rebuild seems necessary, the state is owned at the wrong level.

## Defaults are the framework's job

When a backend renders a primitive it reads theme tokens — `Foreground`,
`Background`, `Surface`, `SurfaceVariant`, `Border`, `Accent`,
`MutedForeground`, `AccentForeground` — rather than hard-coding `.label`,
`.systemBackground`, or `NSColor.windowBackgroundColor`.

The test is simple: view code calling `.foreground()`, `.background()`, or
`text("...")` with no extra modifiers must produce platform-correct output. If
app code has to reach into a backend to make defaults right, that is a backend
bug.

## Asymmetries are documented, not faked

Apple platforms ship SF Symbols; Android has no OS-supplied icon catalog. The
honest answer is that the primitive is supported on one and explicitly
unsupported on the other, and that portable code depends on a packaged icon-set
crate. The dishonest answer is bundling a Material font and calling it "system."

Surfacing the asymmetry as documentation is the right outcome. Hiding it behind a
fallback is not.

## Fail fast

An unexpected state crashes with a clear message rather than degrading into a
plausible-looking default. A missing theme token panics with the slot name
instead of resolving to transparent. `.floating()` panics when `FloatingStyle`
tokens are absent instead of drawing an unstyled box. A view Dew cannot render
panics rather than rendering something else.

Silent fallbacks hide the bug and move the failure somewhere harder to diagnose.

## Signals, not streams

Reactive streams model sequences of events over time; signals model the current
value of a piece of state. UI wants the latter. A text field always has a current
value, a label always shows current text, and a subscriber connecting late needs
the value now rather than a replay.

`Binding<T>` is state plus change notification, `Computed<T>` is a derived value.
Both are synchronous, glitch-free, and main-thread-safe.

## Rust all the way

Application logic, UI composition, state, and layout algorithms are all Rust; the
native backends are adapters. This buys memory safety without a garbage
collector, one language for business logic and UI, and `no_std` support in the
core and FFI crates for embedded targets.

The cost is the FFI boundary. Every Rust/native interaction crosses a C ABI or
JNI. WaterUI keeps that cost down with 128-bit type IDs for O(1) dispatch (no
string comparisons), ownership transfer instead of cross-boundary reference
counting, panic catching at the boundary, and generated bindings — `ffi/waterui.h`
is produced by `cbindgen` and never hand-edited.

## The water metaphor

Water takes the shape of its container without changing what it is. The same view
tree flows through Apple, Android, GTK4, Hydrolysis, and Dew unchanged, and each
gives it a local shape. The API follows: `StretchAxis::MainAxis` means "expand
along whatever axis the parent uses," `Foreground` is a slot rather than a hex
value, and `font::Body` resolves to San Francisco, Roboto, or the system font
depending on where it lands.

## Principles for contributors

1. **Bridge native first.** Use the platform primitive where one exists; go to
   the self-drawn realization only where none does.
2. **Style is an attribute.** Add a variant to an existing semantic component
   rather than a parallel type.
3. **Keep the FFI surface minimal.** Compose in Rust before adding a C-ABI type.
4. **Type safety over runtime checks.** Encode invariants in the type system;
   prefer generics and traits over enums and type erasure.
5. **Never degrade a public trait for object safety.** Expose the friendliest
   signature and erase types privately behind a shim.
6. **No global state.** Pass context through `Environment`, never through statics
   or singletons.
7. **Fail fast.** Panic with a clear message instead of falling back.
8. **Less code is better.** Import a maintained crate rather than reimplementing
   it. Every line is a line that can break.

# Testing your UI

> **In this chapter, you will:**
>
> - Write a `#[waterui::test]` that mounts a view and drives it through the accessibility tree
> - Find elements with `Query`, `Selector`, roles, labels and `a11y_id` identifiers
> - Tap, type, drag and wait without ever sleeping
> - Mount a rendered, styled session to sample animation phases and capture images you read by eye
> - Run the suite with `cargo nextest`

A preview tells you whether a view looks right. A test tells you whether it behaves right, and it does so on every commit without a simulator or a device. `waterui-testing` mounts your view on Hydrolysis's headless runtime and exposes the **accessibility tree** as the test surface: you find a control the way a screen reader would, by role and label, and you operate it with the same actions assistive technology uses.

That choice has a consequence worth stating up front. A control you cannot find or operate through this tree is a control a VoiceOver or TalkBack user cannot reach either, so an untestable component is a bug in the component, not a gap in your coverage. See [Accessibility](../06-advanced/05-accessibility.md) for how labels reach the tree.

## Set up the dev-dependencies

Add the harness to the crate that owns the views:

```toml
[dev-dependencies]
waterui-testing = "0.5"   # the same release line as your `waterui` dependency
hydrolysis-m3 = "0.3"     # only for rendered tests that need a style package
```

A style-free semantic test needs no style package. The `#[waterui::test]` macro looks up `waterui-testing` in your `Cargo.toml`, under a renamed key too, and fails to compile with a message naming the crate when it is missing.

## Your first test

The mounting form names a function that takes no arguments and returns a view. The macro mounts it and hands you the session:

```rust
use waterui::prelude::*;
use waterui_testing::{Role, SemanticApp};

fn greeting() -> impl View {
    vstack((
        text!("Hello, WaterUI"),
        button("Continue").action(|| {}),
    ))
}

#[waterui::test(greeting, viewport = (360, 320))]
fn greeting_shows_text_and_button(app: &mut SemanticApp) {
    app.query().label("Hello, WaterUI").assert_exists();
    app.query().role(Role::BUTTON).label("Continue").assert_exists();
}
# fn main() {}
```

The macro expands to a plain `#[test]`, so do not add one yourself; combining the two is a compile error. The function takes exactly one parameter and returns `()`. The attribute accepts a view path first, then `viewport = (width, height)` in logical points, `theme = <style>` and the bare `offscreen` flag, covered below. Without `viewport` the session is 390 by 844. The session's environment also carries your crate's `i18n/` translation catalog, so localized strings resolve the way they do in the app.

In the prelude the same attribute is spelled `ui_test`, because a glob import of a macro named `test` would shadow the built-in `#[test]` in every file that imports the prelude. Write it qualified as `#[waterui::test(..)]`.

The default session is the **semantic runtime**: a GPU-free pipeline whose accessibility tree comes from the view tree and the widgets' own semantics. It carries no style package, so it answers no geometry. That is enforced by the types: an element resolved from a `SemanticApp` has no `bounds()`, its queries have no pointer gestures, and the compiler tells you so before a test runs.

## Own the state with the manual-mount form

When a test must read or write a `Binding` the view closes over, omit the view path. The function receives the configured `UiBuilder` by value and mounts it in its own body:

```rust
use waterui::prelude::*;
use waterui_testing::UiBuilder;

#[waterui::test(viewport = (360, 200))]
fn stepper_updates_binding(ui: UiBuilder) {
    let quantity: Binding<i32> = binding(2);
    let for_view = quantity.clone();
    let mut app = ui.mount(move || stepper("Quantity", &for_view));

    app.query().label("Quantity").increment();
    assert_eq!(quantity.snapshot(), 3);

    app.query().label("Quantity").decrement();
    assert_eq!(quantity.snapshot(), 2);
}
# fn main() {}
```

`UiBuilder::mount` takes an `Fn` closure that builds the view, because the runtime may rebuild it. Clone the bindings in and read the originals with `.snapshot()` afterwards; reading a binding's current value is fine in a test, just never in a view body. Use the mounting form when the view is self-contained and the manual form when the test needs the state.

The same builder is available outside the macro as `waterui_testing::ui()`, with `.viewport(w, h)`, `.scale_factor(f)` and `.environment(env)` to configure the session. It does not install the translation catalog the macro adds. Tests may be `async fn`; the macro blocks on them until they finish.

## Find elements

`app.query()` returns a `Query`. Every method narrows it, and a terminal method resolves it:

| Narrowing | Matches |
|---|---|
| `.role(Role::BUTTON)` | the accessibility role (`BUTTON`, `LABEL`, `TEXT_INPUT`, `SWITCH`, `SLIDER`, `LIST_ITEM`, `DIALOG` and more) |
| `.label("Wi-Fi")` / `.label_contains("Wi-")` | the accessible label, exact or by substring |
| `.identifier("settings.wifi")` | the identifier set by `.a11y_id(..)` |
| `.value("42")` / `.value_contains("4")` | the node's value |
| `.placeholder(..)` / `.placeholder_contains(..)` | a text field's placeholder |
| `.enabled(bool)`, `.selected(bool)`, `.checked(bool)`, `.mixed()`, `.expanded(bool)`, `.busy(bool)` | state |
| `.hidden(true)` | hidden nodes, which every query skips by default |
| `.content_type(ContentType)` | a text field's content type (`waterui_testing::ContentType`) |
| `.within(&element)` / `.children_of(&element)` | only inside a previously resolved element |

| Terminal | Result |
|---|---|
| `.assert_exists()`, `.assert_not_exists()`, `.assert_ui_focus()` | panics with a description of the selector and its candidates |
| `.exists()` | `bool` |
| `.single()` | the one `ElementRef`; panics when zero or several match |
| `.optional()` | `Option<ElementRef>`; panics when several match |
| `.all()` | an `ElementSet` with `len()`, `is_empty()`, `iter()` and indexing by position or `NodeId` |

Most controls are reachable by role and label, because the framework requires every control to carry one. When a view has no natural label, tag it. The identifier is also exposed to `XCUITest` and Android automation, so it is not test-only scaffolding:

```rust
use waterui::prelude::*;

fn settings_row(enabled: &Binding<bool>) -> impl View {
    toggle("Wi-Fi", enabled).a11y_id("settings.wifi")
}
```

A resolved `ElementRef` keeps a snapshot of its node. `element.node()` returns a `NodeSnapshot` with `role()`, the text accessors `label()`, `identifier()`, `value()` and `placeholder()` (each an `Option<&str>`), `enabled()`, `checked()`, `expanded()` and `children()`, so you can assert on state directly:

```rust
use waterui::prelude::*;
use waterui_testing::{Role, SemanticApp};

fn submit() -> impl View {
    button("Submit").action(|| {}).disabled(true)
}

#[waterui::test(submit)]
fn submit_reports_disabled(app: &mut SemanticApp) {
    let element = app.query().role(Role::BUTTON).label("Submit").single();
    assert!(!element.node().enabled());
}
# fn main() {}
```

An element handle is tied to the tree revision it came from. After an interaction rebuilds the tree, resolve it again: driving a stale handle panics with a message telling you to re-query. Chaining from `app.query()` each time, as the examples here do, always resolves against the current tree.

## Interact

Interactions return `()` and panic when the runtime reports the accessibility action unhandled, so the call itself is the assertion. A `tap()` on a disabled or missing element fails the test on the spot:

```rust
use waterui::prelude::*;
use waterui_testing::{Role, SemanticApp};

fn name_form() -> impl View {
    let name: Binding<Str> = binding(Str::from(""));
    vstack((
        field("Name", &name),
        text!("Saved: {name}"),
    ))
}

#[waterui::test(name_form)]
fn typing_updates_the_summary(app: &mut SemanticApp) {
    // Test strings that include Chinese also include Japanese and Korean.
    for name in ["你好,世界", "こんにちは世界", "안녕하세요 세계"] {
        app.query().role(Role::TEXT_INPUT).label("Name").set_text(name);
        app.query().label(format!("Saved: {name}")).assert_exists();
    }
}
# fn main() {}
```

| Call | Dispatches |
|---|---|
| `.tap()` | the click action |
| `.focus()` | focus |
| `.set_text("..")` | replaces a text field's contents |
| `.increment()` / `.decrement()` | a stepper or slider step |
| `.scroll_down()` | a scroll action |
| `.expand()` / `.collapse()` | the expand and collapse actions; no Hydrolysis widget advertises them yet, so today these calls fail as unhandled |

For input with no element to address, the session offers `app.text_input("..")`, `app.press_named_key("Tab")` and `app.press_character_key("a")`, plus `press_named_key_with` and `press_character_key_with`, which take `Modifiers`. Each `press` call is a full key stroke, press then release. To hold a key across calls, use `key_down(KeyCode, Modifiers)` and `key_up(KeyCode, Modifiers)`. `KeyCode` and `Modifiers` are re-exported from `waterui_testing`.

After every interaction the session settles: it pumps until no input is queued, no spawned work is pending and no animation or patch is scheduled, for at most one second of virtual time. You do not wait for a tap to take effect. The flip side is that a tap which starts a transition returns after the transition has finished; [sampling animation phases](#sample-animation-phases-with-pump_for) covers how to land inside one.

### Pointer gestures

Taps by accessibility action do not go through gesture recognizers. To test a drag, a hover or a magnify gesture you need real coordinates, which only the rendered runtime has (next section). On an `OffscreenApp` the query gains `hover()`, `hover_at(nx, ny)`, `tap_at(nx, ny)`, `drag_between(fx, fy, tx, ty)`, `drag_by(dx, dy)`, `drag_by_with(dx, dy, options)` and `magnify(factor)`. The coordinates of `hover_at`, `tap_at` and `drag_between` are normalized, from 0 to 1 inside the element; `drag_by` takes a delta in points from the element's center. `DragOptions` paces a drag:

```rust,ignore
use waterui::prelude::*;
use waterui_testing::{DragOptions, Role, Styled, UiBuilder};

#[waterui::test(theme = hydrolysis_m3::Material3::defaults(), viewport = (360, 200))]
fn slider_follows_a_drag(ui: UiBuilder<Styled<hydrolysis_m3::Material3>>) {
    let volume: Binding<f64> = binding(0.5);
    let for_view = volume.clone();
    let mut app = ui.mount_offscreen(move || slider("Volume", &for_view));

    let options = DragOptions::default()
        .with_steps(12)
        .with_frame_per_step(true);
    app.query()
        .role(Role::SLIDER)
        .label("Volume")
        .drag_by_with(80.0, 0.0, options);
    assert!(volume.snapshot() > 0.5);
}
```

`with_frame_per_step(true)` pumps one frame between samples so a recognizer sees a real motion timeline, with velocity, instead of every sample arriving at once. The default is six steps with a mouse pointer and no frame between samples. `.with_pointer(PointerKind::Touch)` dispatches the drag as a finger. Scroll views claim touch drags only once `app.set_touch_scroll_config(..)` has supplied the platform's touch parameters, such as Hydrolysis's `TouchScrollConfig::android_default()`. Without it, a touch drag still dispatches but no scroll view claims it.

## Wait on conditions, never on the clock

Work that finishes later, such as a spawned task, a transition or a rebuild, is awaited with a condition. `Query::wait_for_existence` returns a `bool` instead of panicking, so a bare call is a wait that cannot fail. Assert it:

```rust
use core::time::Duration;
use waterui::prelude::*;
use waterui_testing::UiBuilder;

#[waterui::test]
fn status_reaches_ready(ui: UiBuilder) {
    let status: Binding<Str> = binding(Str::from("Loading"));
    let for_view = status.clone();
    let mut app = ui.mount(move || text!("{for_view}"));

    // A change made outside an interaction is not settled for you.
    status.set(Str::from("Ready"));
    assert!(
        app.query()
            .label("Ready")
            .wait_for_existence(Duration::from_secs(2)),
        "the status label should read Ready within two seconds"
    );
}
# fn main() {}
```

`wait_for_nonexistence` and `wait_for_value_eq(value, timeout)` work the same way. For several conditions at once, build `Expectation`s on the session with `expect_exists`, `expect_not_exists`, `expect_value_eq` or `expect_ui_focus`, and hand them to `wait_for`, which returns a `WaitResult` (`Completed`, `TimedOut`, `IncorrectOrder`, `InvertedFulfillment` or `Interrupted`):

```rust
use core::time::Duration;
use waterui_testing::{Selector, SemanticApp, WaitOptions, WaitResult};

fn saved_then_idle(app: &mut SemanticApp) {
    let saved = app.expect_exists(Selector::default().label("Saved"));
    let spinner_gone = app.expect_not_exists(Selector::default().label("Saving"));
    let result = app.wait_for(
        &[saved, spinner_gone],
        WaitOptions::new(Duration::from_secs(2)).enforce_order(true),
    );
    assert_eq!(result, WaitResult::Completed);
}
```

`enforce_order(true)` requires the non-inverted expectations to be met in list order. `Expectation::inverted()` flips an expectation, so the wait fails if it ever becomes true. `WaitOptions::default()` uses a two-second timeout without ordering.

Waits pump hot while the runtime has scheduled work and touch wall-clock time only for work outside it, such as a decoder thread or a network fetch.

> **Never call `std::thread::sleep` in a test.** The animation clock advances when frames are pumped, not when wall-clock time passes. A sleep freezes it, every deferred step lands at once on the next frame, and a capture meant to show a transition mid-flight silently shows its end state. The test still passes and the image still looks plausible, which is what makes it dangerous.

## Rendered sessions with a style package

The semantic runtime has no pixels. To read geometry, dispatch pointer gestures or capture an image, carry a style into the builder with `theme =` and mount the rendered runtime with the `offscreen` flag. The style is any value implementing Hydrolysis's `Style` trait, re-exported as `waterui_testing::Style`, such as `hydrolysis_m3::Material3::defaults()`. The parameter becomes `&mut OffscreenApp`:

| Session | Mounted by | Has |
|---|---|---|
| `SemanticApp` | default; `ui().mount(..)` | queries, accessibility actions, waits, key and text input |
| `SemanticApp` with style tokens | `ui().theme(style).mount(..)` | the same, with the style's tokens in the view's environment |
| `OffscreenApp` | `offscreen` flag; `ui().theme(style).mount_offscreen(..)` | all of the above, plus `bounds()`, pointer gestures, `pump_for`, `snapshot()` |

`offscreen` requires `theme =`, because the rendered runtime is styled by construction, and requires a view path. In the manual form, pass `theme =` without a view path, name the style in the parameter type, for example `ui: UiBuilder<Styled<hydrolysis_m3::Material3>>`, and call `mount_offscreen` yourself, as the slider test above does. A whole `App` mounts through `waterui_testing::mount_app(app, style)`, which sizes the session from its first window's declared frame.

A styled session resolves the font families its style package names strictly: against the fonts installed on the host plus the `local_path` font files that your package's dependency graph declares under `[[package.metadata.waterui.assets.font]]`. A family that is neither fails the test with its name. Fonts declared by registry name or `remote_path` are staged only by the `water` CLI, so install them on the machine that runs the tests.

### Sample animation phases with `pump_for`

The clock is virtual: each pump advances it by one fixed frame of 16 ms. On an `OffscreenApp`, `pump_for` advances it by exactly the duration you ask for, so the result does not depend on how loaded the machine is. Because an interaction settles until animations finish, start the transition by changing state the test owns, then step through it:

```rust,ignore
use core::time::Duration;
use waterui::animation::Animation;
use waterui::prelude::*;
use waterui_testing::{Styled, UiBuilder};

#[waterui::test(theme = hydrolysis_m3::Material3::defaults(), viewport = (390, 844))]
fn fade_in_phases(ui: UiBuilder<Styled<hydrolysis_m3::Material3>>) {
    let opacity: Binding<f32> = binding(0.0);
    let for_view = opacity.clone();
    let mut app = ui.mount_offscreen(move || {
        text("Hello").opacity(for_view.with(Animation::linear(Duration::from_millis(400))))
    });

    opacity.set(1.0);
    app.pump_for(Duration::from_millis(200));
    app.capture_snapshot("gallery", "fade-in", "halfway");
    app.pump_for(Duration::from_millis(400));
    app.capture_snapshot("gallery", "fade-in", "settled");
}
```

To start a transition from input instead, use the `queue` variants, which skip the settle: `queue_pointer_down(x, y)` and `queue_pointer_up(x, y)` on the `OffscreenApp`, and `queue_key_press`, `queue_key_release` and `queue_text_input` on the session. The next pump processes the event.

`pump_for` advances the clock without reading pixels back. `snapshot()` pumps one more frame and returns a `Snapshot` with `width()`, `height()` and `rgba8()`. For work that depends on real I/O, such as a remote image, `pump_until(timeout, || ready)` pumps frames, blocks on the local executor once the runtime is idle, and returns whether the condition became true before the timeout.

### Snapshots are for your eyes

`app.capture_snapshot(suite, case, stage)` writes a PNG to `<suite>/<case>/<stage>.png` under the directory named by the `WATERUI_TEST_ARTIFACTS_DIR` environment variable, and under `waterui-testing-artifacts` in the system temporary directory when it is unset. It returns a `CapturedSnapshot` whose `path()` is the file. When you need a file at a specific path, save the raw snapshot; `save_png` creates the missing parent directories:

```rust,ignore
let shot = app.snapshot();
shot.save_png("target/visual/card.png")
    .expect("the snapshot directory must be writable");
```

A visual test means **looking at the image**: open the PNG and judge it. Do not write changed-pixel counts, opaque-pixel thresholds, bounding-box approximations or dominant-color checks to stand in for appearance. They pass on broken output and fail on harmless drift, and they teach you nothing about what the user sees. Use semantic assertions for behavior and your own eyes for appearance. If you want an image per commit, capture it with a stage name that says what it shows (`halfway`, `settled`), have CI upload the `WATERUI_TEST_ARTIFACTS_DIR` directory, and review the images there.

Check layout the same way: through the elements' `bounds()` on a rendered session, and how they relate to each other. Never attach accessibility metadata to a decorative fill only so a test can find it.

## Run the suite

Run tests with `cargo nextest run`, which runs each test in its own process:

```bash
cargo nextest run -p my-app
cargo nextest run -p my-app -E 'test(greeting_shows_text_and_button)'
cargo nextest run -p my-app -E 'test(typing_updates_the_summary)' --no-capture
cargo test --doc
```

`--no-capture` shows output and forces serial execution, so scope it with `-E`. Each test gets its own process, so a test that relies on a sibling's global initialization is order-dependent. Fix it rather than switching to `cargo test`. Doctests are the one thing nextest cannot run, which is why `cargo test --doc` stays. Print diagnostics with `tracing`, not `println!`.

From a generated project, `water preview test` asserts on a previewed view; see [the preview chapter](01-preview.md). Frame-time budgets belong to [benchmarks](04-benchmarks.md), and an agent can drive the same tree interactively through [the MCP server](06-mcp.md).

## What's next

Once behavior is covered, measure it. [Benchmarks](04-benchmarks.md) mounts the same views on the rendered runtime and enforces frame-time and rebuild budgets.

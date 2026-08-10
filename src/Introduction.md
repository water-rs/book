# Introduction

> **In this chapter, you will:**
> - Learn what WaterUI is and how it reaches each platform
> - See which backends exist and what each one renders with
> - Find your way around the workspace and this book
> - Read a working counter written in WaterUI

> **Pinned to upstream:** every example and API name in this book is
> verified against [waterui {{waterui_branch}} `{{waterui_commit_short}}`](https://github.com/water-rs/waterui/commit/{{waterui_commit}})
> ({{waterui_committed_at}}, "{{waterui_subject}}"). When the submodule
> bumps, the chapters bump with it.

## What is WaterUI?

WaterUI is a cross-platform, reactive, declarative UI framework for Rust. You
describe your interface as a tree of `View` values; the framework decides how
each node is realised on the current platform.

Realisation is **native first**. Where a platform provides a canonical primitive
for a semantic component -- a button, a text field, a list -- WaterUI bridges to
it: UIKit/AppKit on Apple platforms, Android View on Android, GTK4 on Linux.
Where no suitable platform primitive exists, or where the target has no widget
toolkit at all, WaterUI uses one of its own renderers. That is a deliberate
choice per component, not a fallback after a failed native call.

```text
                            ┌─ Apple backend (Swift)   → UIKit / AppKit
Rust View tree ─ FFI (C ABI)┼─ Android backend (Kotlin)→ Android View
                            ├─ GTK4 backend            → GTK4 widgets
                            ├─ Hydrolysis              → GPU, self-drawn
                            └─ Dew                     → CPU, self-drawn
```

Updates are **fine-grained**. `Binding<T>`, `Computed<T>`, and signal-aware
component inputs update the affected value in place. There is no structural
diff pass over the tree, and changing one label does not rebuild its siblings.

## Backends

| Backend | Targets | Realisation |
|---------|---------|-------------|
| Apple | iOS, macOS | UIKit / AppKit through a Swift package |
| Android | Android | Android View through Kotlin and JNI |
| GTK4 | Linux | GTK4 widgets through `gtk4-rs` |
| Hydrolysis | macOS, Linux, Windows, Web | Self-drawn GPU renderer (Vello on `wgpu`) |
| Dew | ESP32-S3, ESP32-C3 | Self-drawn CPU renderer with dirty-area banding |

Hydrolysis redraws the whole scene every frame on the GPU and targets high
refresh rates. Dew is its opposite: CPU rasterisation, dirty rectangles sliced
into bands so peak pixel memory is one band rather than a frame, sized for
microcontrollers. `hydrolysis-m3` layers a Material Design 3 theme package on
top of Hydrolysis.

WaterUI is pre-1.0 (`waterui 0.2.x`), and the upstream roadmap still lists
self-rendering milestones as open, so component coverage in Hydrolysis and Dew
trails the native bridges. Pick one backend to start; you do not need the rest.

## Workspace layout

You depend on the single `waterui` crate, which re-exports the rest through
`waterui::prelude::*`. The table is a map for reading the source, not a list of
dependencies to add.

| Crate | Path | Role |
|-------|------|------|
| `waterui` | `/` | Facade: prelude, widgets, macro re-exports |
| `waterui-internal` | `src/` | Implementation behind the facade |
| `waterui-core` | `core/` | `View`, `Environment`, `AnyView`, layout and accessibility contracts |
| `waterui-layout` | `components/foundation/layout/` | Stacks, grids, `ScrollView`, `Spacer`, absolute layout |
| `waterui-text` | `components/foundation/text/` | `Text`, fonts, styled text |
| `waterui-controls` | `components/foundation/controls/` | `Button`, `Toggle`, `Slider`, `Stepper`, `TextField`, `Label` |
| `waterui-form` | `components/foundation/form/` | Form builder, `Picker` |
| `waterui-navigation` | `components/foundation/navigation/` | Navigation stacks, tabs, split views, routing |
| `waterui-shape` | `components/foundation/shape/` | Shape primitives |
| `waterui-icon` | `components/foundation/icon/` | Icon system; icon sets live under `components/icon/` |
| `waterui-graphics` | `components/visual/graphics/` | Colours, gradients, GPU surface, image analysis |
| `waterui-image` / `waterui-svg` / `waterui-canvas` | `components/visual/` | Images, SVG, canvas drawing |
| `waterui-media` / `waterui-video` | `components/multimedia/` | Photos, audio, video playback |
| `waterui-chart` / `waterui-map` | `components/data/` | Charts and maps |
| `waterui-barcode` | `components/codes/barcode/` | Barcode and QR rendering |
| `waterui-particle` | `components/effects/particle/` | Particle systems |
| `waterui-webview` / `waterui-chromium` | `components/platform/` | Embedded web views and Chromium/CDP |
| `waterui-assets` | `components/assets/runtime/` | Asset loading, `asset!`, bundles |
| `waterui-macros` | `macros/` | `text!`, `#[form]`, `#[preview]`, `#[derive(Identifiable)]` |
| `waterui-locale` | `utils/locale/` | Locale resolution and `catalog!` |
| `nami` | `utils/nami/` | The reactive engine behind `waterui::reactive` |
| `filtrate` | `utils/filtrate/` | GPU filter and effect runtime |
| `waterui-testing` | `testing/` | Semantic UI tests over the accessibility tree |
| `waterui-ffi` | `ffi/` | C ABI bridge; owned by the CLI, not by your app |
| `waterui-cli` | `cli/` | The `water` command |

Backends live under `backends/`: `apple/` and `android/` are git submodules,
`gtk/`, `hydrolysis/`, `hydrolysis_m3/`, and `dew/` are workspace crates, and
`core/` holds the shared backend contracts.

`waterui-canvas` is a workspace crate that the `waterui` facade does not
re-export at this checkpoint.

## Prerequisites

You should be comfortable with Rust ownership, traits, generics, and closures
-- if not, work through
[The Rust Programming Language](https://doc.rust-lang.org/book/) first -- and
with a terminal, since `water` and `cargo` do the building. Having one platform
toolchain installed (Xcode, Android Studio, or GTK4 development libraries) lets
you run the examples on real hardware.

## How to use this book

The eight parts build on each other: **Getting Started** (toolchain, CLI,
first app, project layout), **Core Concepts** (`View`, reactivity, environment,
modifiers), **Building UIs** (text, layout, controls, forms, lists,
navigation), **Rich Content** (media, maps, web views, barcodes),
**Graphics and Effects** (canvas, GPU surfaces, shaders, filters, particles,
gradients), **Advanced Patterns** (animation, gestures, async, errors,
accessibility, i18n, plugins), **Developer Tools** (the preview system), and
**Under the Hood** (rendering, FFI, layout engine, backend architecture).

Most chapters contain runnable examples. Create a scratch project with
`water create "Scratch" --mode playground` and paste as you read. Chapters that
discuss workspace-only internals say so.

## A taste of WaterUI

```rust,ignore
use waterui::app::App;
use waterui::prelude::*;

pub fn main() -> impl View {
    let counter = Binding::i32(0);

    vstack((
        text!("Count: {counter}"),
        hstack((
            button("Decrement")
                .action(|State(c): State<Binding<i32>>| *c.get_mut() -= 1)
                .state(&counter),
            button("Increment")
                .action(|State(c): State<Binding<i32>>| *c.get_mut() += 1)
                .state(&counter),
        )),
    ))
}

pub fn app(env: Environment) -> App {
    App::new(main, env)
}
```

That is the whole user crate: a root view and a public `app(env)` constructor.
The `water` CLI generates the FFI companion crate that native backends load, so
you never write `waterui_ffi::export!()` yourself. The same code runs on every
supported target without a `#[cfg]` branch.

## Contributing

- **Book source**: [github.com/water-rs/book](https://github.com/water-rs/book)
- **Framework source**: [github.com/water-rs/waterui](https://github.com/water-rs/waterui)

Continue to [The Water CLI](01-getting-started/01-cli.md) to install the
toolchain and scaffold a project.

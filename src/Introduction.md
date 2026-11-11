# Introduction

> **In this chapter, you will:**
> - Learn what WaterUI is and how it reaches each platform
> - See which backends are mainline, which are experimental, and what each one renders with
> - Find your way around the framework repository and this book
> - Read a working counter written in WaterUI

> **Pinned to upstream:** every example and API name in this book is
> verified against [waterui {{waterui_branch}} `{{waterui_commit_short}}`](https://github.com/water-rs/waterui/commit/{{waterui_commit}})
> ({{waterui_committed_at}}, "{{waterui_subject}}"). When the submodule
> bumps, the chapters bump with it.

## What is WaterUI?

WaterUI is a cross-platform, reactive, declarative UI framework for Rust. You
describe your interface as a tree of `View` values; the framework decides how
each node is realized on the current platform. Component semantics and API are
defined once in Rust, and every backend realizes them with its own platform's
look.

On Apple platforms that means the platform's own controls. Where a control
carries the whole semantic, the backend uses it; otherwise it composes AppKit
or UIKit views. On Android, the web and the desktop, the Hydrolysis backend
draws every component itself. Which one you get is a property of the backend
you build for, not a fallback after a failed native call.

```text
                                  ┌─ Apple backend → AppKit / UIKit (objc2)
Rust View tree → backend contracts┤
                                  └─ Hydrolysis    → HWUI RenderNode display lists
                                                     or Cherenkov (GPU, or CPU)
```

Updates are **fine-grained**. `Binding<T>`, `Computed<T>`, and signal-aware
component inputs update the affected value in place. There is no structural
diff pass over the tree, and changing one label does not rebuild its siblings.
The reactive engine is the [`nami`](https://crates.io/crates/nami) crate from
crates.io, re-exported as `waterui::reactive`; the
[reactive chapter](02-core/02-reactive.md) covers it.

## Backends

Two backends are mainline. Their integration checks define the framework's
release contract.

| Backend | Targets | Realization |
|---------|---------|-------------|
| Apple | macOS, iOS | AppKit and UIKit, driven from Rust through `objc2`; no FFI layer in between |
| Hydrolysis | Android, Windows, Linux, Web, and macOS on request | Self-drawn. Renders through Cherenkov, the 2D engine in the same repository; on Android it can also paint through HWUI |

Hydrolysis is the self-drawn realization and also the Android backend. On
Android it has two painters: the default `gpu` painter renders through
Cherenkov into a `SurfaceView`, and the `hwui` painter records its layer tree
into HWUI `RenderNode` display lists. `water run --platform android --painter
hwui` picks the second, and the `painter` key of the `[hydrolysis]` table in
`Water.toml` sets the project default. Cherenkov has a GPU renderer (`cherenkov-gpu`), which
Hydrolysis uses on phones, tablets and desktops, and a CPU rasterizer
(`cherenkov-cpu`) for microcontroller-class targets; the renderer is chosen per
target at build time, never as a runtime fallback. Hydrolysis is
theme-independent: style packages such as `hydrolysis-m3`, a Material Design 3
package, implement the public widget and theme contracts on top of it.

Android, Windows and the web build with Hydrolysis by default, and macOS with
the Apple backend. On Linux the `water` CLI defaults to the experimental GTK
backend, so pass `--backend hydrolysis` to build with Hydrolysis there; the same
flag selects Hydrolysis on macOS.

GTK, WinUI and TUI backends are **experimental**. They have no stability or
completeness guarantee and live in their own repositories, outside the
framework tree; they never hold back a mainline release.

### Platform support

| Tier | Platforms | Meaning |
|------|-----------|---------|
| 1 | macOS, iOS, Android | Tested on real devices |
| 2 | Windows, Linux | Built and tested in CI; failures fixed promptly |
| 3 | GTK, WinUI and TUI backends | Experimental |

A few constraints are worth knowing before you pick a target:

- Apple support is ARM64 only: `aarch64-apple-darwin`, `aarch64-apple-ios`
  and `aarch64-apple-ios-sim`. Intel Apple targets are retired, and asking for
  one fails with an unsupported-target error.
- GPU rendering on Linux and Android goes through Vulkan only, and on Windows
  through DX12 only. A device without that API fails with an unsupported-device
  error rather than switching to another API.
- GL serves only web browsers without WebGPU, for compatibility. It gets no
  significant investment and is expected to go away once WebGPU is widespread.
- Windows does not support dynamic linking; development builds link statically.

WaterUI is pre-1.0, so public APIs still change between releases. Pick one
backend to start; you do not need the rest.

## Repository layout

You depend on the single `waterui` crate. It is a facade that re-exports the
rest through `waterui::prelude::*` and named modules, and heavyweight
components are optional Cargo features: turning one off removes its
dependencies from your build. The facade crate itself is a thin
`facade.rs` over the implementation in `src/`. The map below is for reading
the framework source, not a list of dependencies to add.

| Path | Crate | Role |
|------|-------|------|
| `src/`, `facade.rs` | `waterui`, `waterui-internal` | The facade, its prelude, and the implementation behind it |
| `core/` | `waterui-core` | `View`, `Environment`, `AnyView`, layout and accessibility contracts |
| `components/` | component crates, grouped by domain | `foundation/` (layout, text, controls, forms, navigation, shapes, color, icons), `visual/` (`waterui-graphics`), `multimedia/`, `data/`, `platform/`, `assets/`, `devtools/`, `icon/` (icon sets) |
| `backends/core` | `waterui-backend-core` | Contracts shared by every backend |
| `backends/apple` | `waterui-apple` | The Apple backend, with its thin Swift host (`Sources/WaterUI`, published by the root `Package.swift`) |
| `backends/hydrolysis` | `hydrolysis` | The self-drawn backend, with its Android host (Gradle project) |
| `graphics/cherenkov/` | `cherenkov`, `cherenkov-*` | The 2D engine Hydrolysis renders through |
| `graphics/filtrate/` | `filtrate`, `filtrate-*` | The GPU filter library behind `.filter(...)` and `.effect(...)` |
| `cli/` | `waterui-cli` | The `water` command |
| `macros/` | `waterui-macros` | `text!`, `#[preview]`, `#[derive(Identifiable)]` and friends |
| `utils/`, `testing/` | `waterui-locale`, `waterui-url`, `waterui-testing`, ... | Supporting crates and the semantic UI test harness |

Several things you may remember from earlier versions of WaterUI are gone from
this tree: the Dew CPU renderer, the hand-written Android View backend, the
C ABI bridge crate, and the in-tree copy of `nami`. The CLI generates the glue
a native host needs, so your crate never writes it.

Self-drawn components and style packages live in their own `water-rs`
repositories and own their implementations, tests and releases. Charts,
barcodes and particles are examples: the `waterui` facade re-exports them as
`waterui::chart`, `waterui::barcode` and `waterui::particle` behind the
optional features of the same names, but their source is not under the
framework's `components/` directory. Canvas drawing stays in the tree, in
`waterui-graphics`, and you reach it as `waterui::canvas` with the `canvas`
feature; the [graphics part](05-graphics/01-canvas.md) covers it.

## Prerequisites

You should be comfortable with Rust ownership, traits, generics, and closures
-- if not, work through
[The Rust Programming Language](https://doc.rust-lang.org/book/) first -- and
with a terminal, since `water` and `cargo` do the building. Having one platform
toolchain installed (Xcode for Apple targets, the Android SDK and NDK for Android) lets
you run the examples on real hardware; `water doctor` reports what is missing.

## How to use this book

The parts build on each other: **Getting Started** (toolchain, CLI, first
app, project layout, platforms), **Core Concepts** (`View`, reactivity,
environment, modifiers), **Building UIs** (text, layout, controls, forms,
lists, conditional rendering, navigation, windows, menus, overlays, styling,
icons), **Rich Content**
(media, maps, web views, barcodes, assets, charts), **Graphics and Effects**
(canvas, GPU surfaces, shaders, filters, particles, gradients), **Advanced
Patterns** (animation, gestures, suspense, errors, accessibility, i18n,
plugins, resolvers, keyboard focus, drag and drop, TypeScript, widgets,
embedding), **Developer Tools** (preview, testing, benchmarks, the inspector,
MCP, packaging), and **Under the Hood** (rendering, the layout engine, backend
architecture, library authoring).

Most chapters contain runnable examples. Create a scratch project with
`water create "Scratch"` and paste as you read; the
[CLI chapter](01-getting-started/01-cli.md) and
[first app](01-getting-started/03-first-app.md) walk through it. Chapters that
discuss framework internals say so.

## A taste of WaterUI

```rust
use waterui::app::App;
use waterui::prelude::*;

fn counter() -> impl View {
    let count = Binding::i32(0);

    vstack((
        text!("Count: {count}"),
        hstack((
            button("Decrement")
                .action(|State(count): State<Binding<i32>>| *count.get_mut() -= 1)
                .state(&count),
            button("Increment")
                .action(|State(count): State<Binding<i32>>| *count.get_mut() += 1)
                .state(&count),
        )),
    ))
    .padding()
}

pub fn app(env: Environment) -> App {
    App::new(counter, env)
}
```

That is the whole user crate: a root view and a public `app(env)` constructor.
`text!("Count: {count}")` takes the binding itself, not its value, so only that
label updates when the count changes. Reading a binding with `.snapshot()` is
for event handlers and `.map()` closures; in a view body it freezes the value
and the label never updates. The `water` CLI generates and manages the native
host projects, so you never write FFI glue. The same code runs on every
supported target without a `#[cfg]` branch.

## Contributing

- **Book source**: [github.com/water-rs/book](https://github.com/water-rs/book)
- **Framework source**: [github.com/water-rs/waterui](https://github.com/water-rs/waterui)

Continue to [The Water CLI](01-getting-started/01-cli.md) to install the
toolchain and scaffold a project.

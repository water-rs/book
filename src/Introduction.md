# Introduction

> **In this chapter, you will:**
> - Discover what WaterUI is and why it exists
> - Understand how native rendering differs from web-view approaches
> - See the full range of supported platforms and backends
> - Get a taste of WaterUI with a working counter example

> **Pinned to upstream:** every example and API name in this book is
> verified against [waterui {{waterui_branch}} `{{waterui_commit_short}}`](https://github.com/water-rs/waterui/commit/{{waterui_commit}})
> ({{waterui_committed_at}}, "{{waterui_subject}}"). When the submodule
> bumps, the chapters bump with it.

Imagine writing your UI once in Rust and having it render as a truly native app
on iOS, Android, macOS, and Linux -- no web views, no custom rendering, just
real platform widgets. That is what WaterUI gives you. If you have ever wished
for the safety of Rust's type system combined with the ergonomics of SwiftUI or
Jetpack Compose, you are in the right place.

## What is WaterUI?

WaterUI is a **cross-platform, reactive, declarative UI framework** for Rust.
You describe *what* your interface should look like, and the framework takes care
of *how* it renders -- on every platform.

Unlike electron-style approaches that draw their own pixels inside a web view,
WaterUI renders to **native platform widgets**. On Apple platforms (iOS and
macOS) it bridges to SwiftUI/UIKit/AppKit through a Swift backend. On Android it
bridges to Android Views via JNI/Kotlin. On Linux it delegates to GTK4.
The result is an application that looks, feels, and performs like a first-class
citizen on each operating system.

```text
Rust View Tree  --->  FFI (C ABI)  --->  Native Backend  --->  Platform UI
                                          Swift / Kotlin / GTK4
```

### Key Features

- **Cross-platform**: iOS, Android, macOS, and Linux from one Rust codebase.
  The default native backends are Apple, Android, and GTK4; Hydrolysis provides
  an experimental self-drawn path for macOS, Linux, Windows, and Web.
- **Type-safe**: Leverage Rust's type system, ownership, and lifetimes to
  eliminate whole categories of runtime errors at compile time.
- **Reactive**: WaterUI's `Binding<T>`, `Computed<T>`, and `Signal` types
  automatically propagate changes through the view tree so the UI stays in sync
  with your data.
- **Declarative**: Describe your UI as a composition of `View` values. Layout,
  styling, and interaction are expressed through method chaining and tuple
  composition rather than imperative mutation.
- **Native rendering**: Each backend maps Rust views to the platform's own
  widgets, giving you native text rendering, accessibility, animations, and
  input handling for free.

## Supported Backends

| Backend | Platform(s) | Technology | Status |
|---------|-------------|------------|--------|
| Apple | iOS, macOS | SwiftUI / UIKit / AppKit via Swift | Stable |
| Android | Android | Android Views via Kotlin / JNI | Stable |
| GTK4 | Linux | GTK4 via gtk4-rs | Stable |
| Hydrolysis | macOS, Linux, Windows, Web | Self-drawn (Vello / tiny-skia / wgpu) | Experimental |

> **Note:** You only need one backend to get started. Most readers begin with
> whichever platform they already have tooling for -- macOS if you have a Mac,
> Linux with GTK4, or Android if you have Android Studio installed.

## Framework Architecture

WaterUI is organised as a Cargo workspace. The table below lists the most
important crates. You do not need to depend on them individually -- the
top-level `waterui` crate re-exports everything through `waterui::prelude::*`.

| Crate | Path | Role |
|-------|------|------|
| `waterui` | `/` | Facade crate, re-exports components, prelude, macros |
| `waterui-core` | `core/` | `View` trait, `Environment`, `AnyView`, reactive primitives |
| `waterui-layout` | `components/foundation/layout/` | `VStack`, `HStack`, `ZStack`, `ScrollView`, `Spacer`, grids |
| `waterui-text` | `components/foundation/text/` | `Text` view, fonts, styled text, markdown |
| `waterui-controls` | `components/foundation/controls/` | `Button`, `Toggle`, `Slider`, `Stepper`, `TextField` |
| `waterui-navigation` | `components/foundation/navigation/` | Navigation containers, `TabView` |
| `waterui-form` | `components/foundation/form/` | `#[form]` derive macro, form builder |
| `waterui-media` | `components/multimedia/media/` | Photos, video, audio playback |
| `waterui-graphics` | `components/visual/graphics/` | GPU surfaces, filters, gradients, image analysis |
| `waterui-canvas` | `components/visual/canvas/` | Workspace canvas crate; not re-exported by `waterui` at this checkpoint |
| `waterui-icon` | `components/foundation/icon/` | Cross-platform icon system |
| `waterui-webview` | `components/platform/webview/` | Embedded web views |
| `waterui-macros` | `macros/` | Proc macros: `text!`, `#[form]`, `#[preview]` |
| `waterui-ffi` | `ffi/` | C FFI bridge, `export!()` macro |
| `waterui-cli` | `cli/` | The `water` CLI for scaffolding, building, running, packaging |
| `waterui-str` | `utils/str/` | Shared string utilities |
| `waterui-url` | `utils/url/` | URL handling utilities |
| `waterui-locale` | `utils/locale/` | Localisation and formatting |
| `waterui-assets` | `components/assets/` | Asset loading and management |
| `nami` | `utils/nami/` (vendored submodule) | Fine-grained reactive implementation behind `waterui::reactive`; app code should use WaterUI re-exports |

### Backend Crates

| Crate | Path | Role |
|-------|------|------|
| `waterui-backend-core` | `backends/core/` | Shared backend abstractions |
| Apple backend | `backends/apple/` | Swift Package (git submodule) |
| Android backend | `backends/android/` | Gradle project (git submodule) |
| `waterui-gtk` | `backends/gtk/` | GTK4 backend implementation |
| Hydrolysis | `backends/hydrolysis/` | Self-drawn renderer (experimental) |

> **Tip:** You will rarely interact with individual crates directly. The
> `waterui::prelude::*` import gives you everything you need in day-to-day
> development.

## Prerequisites

Before starting this book, you should be comfortable with:

- **Basic Rust** -- ownership, borrowing, traits, generics, and closures. If
  you are new to Rust, we recommend working through
  [The Rust Programming Language](https://doc.rust-lang.org/book/) first.
- **The command line** -- you will use the `water` CLI and `cargo` extensively.
- **One target platform** -- having Xcode (for Apple targets), Android Studio
  (for Android), or GTK4 development libraries (for Linux) installed will let
  you run examples on real hardware.

## How to Use This Book

The book is structured in eight parts that build on each other:

1. **Getting Started** -- Install the toolchain, learn the CLI, create your
   first app, and understand the project layout.
2. **Core Concepts** -- The `View` trait, WaterUI reactive state,
   environment-based dependency injection, and modifiers.
3. **Building UIs** -- Text, layout, controls, forms, lists, conditional
   rendering, and navigation.
4. **Rich Content** -- Media, maps, web views, and barcodes.
5. **Graphics and Effects** -- Canvas drawing, GPU rendering, shaders, filters,
   particles, and gradients.
6. **Advanced Patterns** -- Animation, gestures, async views, error handling,
   accessibility, internationalisation, and plugins.
7. **Developer Tools** -- The preview system and hot reload.
8. **Under the Hood** -- How WaterUI renders, the FFI bridge, the layout
   engine, and backend architecture.

Most chapters contain runnable code examples. Clone the repository and use
`water create "Counter" --mode playground` to set up sandbox projects as you
follow along. Chapters that discuss workspace-only internals call that out
explicitly.

## A Taste of WaterUI

Here is a minimal counter application to give you a feel for the framework:

```rust,ignore
use waterui::prelude::*;
use waterui::app::App;

pub fn main() -> impl View {
    let counter = Binding::i32(0);

    vstack((
        text!("Count: {counter}"),
        hstack((
            button("Decrement")
                .action(|State(c): State<Binding<i32>>| c.set(c.get() - 1))
                .state(&counter),
            button("Increment")
                .action(|State(c): State<Binding<i32>>| c.set(c.get() + 1))
                .state(&counter),
        )),
    ))
}

pub fn app(env: Environment) -> App {
    App::new(main, env)
}
```

This is the user crate's `src/lib.rs`: it defines your root view and the
public `app(env)` constructor. The CLI generates a companion FFI crate that
exports the C entry points native backends need, so you do not write
`waterui_ffi::export!()` in this file. The same Rust view code runs on the
supported native targets without platform-specific `#[cfg]` branches.

## Contributing

This book is open source. Found a typo, an unclear explanation, or want to add
a chapter?

- **Book source**: [github.com/water-rs/book](https://github.com/water-rs/book)
- **Framework source**: [github.com/water-rs/waterui](https://github.com/water-rs/waterui)
- **Issues and pull requests**: contributions are welcome on either repository

Head to [The Water CLI](01-getting-started/01-cli.md) to install your tools and
create your first project.

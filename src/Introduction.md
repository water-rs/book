# WaterUI Tutorial Book

Welcome to the complete guide for building cross-platform applications with WaterUI! This book will take you from a complete beginner to an advanced WaterUI developer, capable of building sophisticated applications that run on desktop, web, mobile, and embedded platforms.

## What is WaterUI?

WaterUI is a modern, declarative UI framework for Rust that enables you to build applications using a single codebase for multiple platforms. It combines the safety and performance of Rust with an intuitive, reactive programming model inspired by SwiftUI and React.

### Key Features

- **🚀 Cross-Platform**: Write once, deploy everywhere - desktop, web, mobile, embedded
- **🦀 Type-Safe**: Leverage Rust's powerful type system for compile-time correctness
- **⚡ Reactive**: Automatic UI updates when data changes
- **📝 Declarative**: Describe what your UI should look like, not how to build it

## Meet the `water` CLI

Every chapter assumes you have the WaterUI CLI installed so you can scaffold, build, and package projects without leaving the terminal.

```bash
cargo install --path cli --locked
```

From there you can bootstrap a playground app and run it on any configured backend:

```bash
water create --name "Water Demo" \
  --bundle-identifier com.example.waterdemo \
  --backend swiftui --backend android --backend web --yes

water run --platform web --project water-demo
water package --platform android --project water-demo
```

Use `water doctor --fix` whenever you need to validate the local toolchain, and `water devices --json` to pick a simulator/emulator when scripting. The CLI mirrors the repository layout you are about to explore, so the hands-on examples in each chapter directly match real projects.

## Framework Layout

The WaterUI workspace is a set of focused crates:

- `waterui-core`: the `View` trait, `Environment`, resolver system, and plugin hooks.
- `waterui/components/*`: reusable primitives for layout, text, navigation, media, and form controls.
- `nami`: the fine-grained reactive runtime that powers bindings, signals, and watchers.
- `waterui-cli`: the developer workflow described above.

This book mirrors that structure—learn the core abstractions first, then layer components, and finally explore advanced topics such as plugins, animation, and async data pipelines.

### Workspace Crates (excluding backends)

| Crate | Path | Highlights |
| ----- | ---- | ---------- |
| `waterui` | `waterui/` | Facade crate that re-exports the rest of the stack plus hot reload, tasks, and metadata helpers. |
| `waterui-core` | `waterui/core` | `View`, `Environment`, resolver system, plugins, hooks, and low-level layout traits. |
| `waterui-controls` | `waterui/components/controls` | Buttons, toggles, sliders, steppers, text fields, and shared input handlers. |
| `waterui-layout` | `waterui/components/layout` | Stacks, frames, grids, scroll containers, padding, and alignment primitives. |
| `waterui-text` | `waterui/components/text` | The `Text` view, typography helpers, and localization-ready formatting APIs. |
| `waterui-media` | `waterui/components/media` | Photo/video/Live Photo renderers plus media pickers. |
| `waterui-navigation` | `waterui/components/navigation` | Navigation bars, stacks, programmatic paths, and tab containers. |
| `waterui-form` | `waterui/components/form` | FormBuilder derive macro, color pickers, secure fields, and validation helpers. |
| `waterui-graphics` | `waterui/components/graphics` | Experimental drawing primitives and utilities that feed the canvas/shader chapters. |
| `waterui-render-utils` | `waterui/render_utils` | Shared GPU/device glue used by multiple backends and native wrappers. |
| `waterui-derive` | `waterui/derive` | Proc-macros (`FormBuilder`, `View` helpers) consumed by the higher-level crates. |
| `waterui-cli` | `waterui/cli` | The `water` binary you installed earlier for scaffolding, running, and packaging apps. |
| `waterui-ffi` | `waterui/ffi` | FFI bridge used by native runners (Swift, Kotlin, C) plus hot reload integration. |
| `waterui-color`, `waterui-str`, `waterui-url` | `waterui/utils/{color,str,url}` | Utility crates for colors, rope strings, and URL handling shared by every component. |
| `window` | `waterui/window` | Cross-platform window/bootstrapper that spins up render loops for each backend. |
| `demo` | `waterui/demo` | Showcase app exercising all components—great for cross-referencing when you read later chapters. |

Outside `waterui/` you will also find the `nami/` workspace, which hosts the reactive runtime along with its derive macros and examples. Treat `nami` as part of the core mental model because every binding, watcher, and computed signal ultimately comes from there.

### Prerequisites

Before starting this book, you should have:

- **Basic Rust Knowledge**: Understanding of ownership, borrowing, traits, and generics
- **Programming Experience**: Familiarity with basic programming concepts
- **Command Line Comfort**: Ability to use terminal/command prompt

If you're new to Rust, we recommend reading [The Rust Programming Language](https://doc.rust-lang.org/book/) first.

## How to Use This Book

1. **Clone the repository** and run `mdbook serve` so you can edit and preview chapters locally.
2. **Explore the source** under `waterui/` whenever you want to dig deeper into a crate.
3. **Use the CLI** at the start of each part to scaffold a sandbox project for experimentation.

## Roadmap

WaterUI is evolving quickly. Track milestones and open issues at [waterui.dev/roadmap](https://waterui.dev/roadmap).

## Contributing

This book is open source! Found a typo, unclear explanation, or want to add content?

- **Source Code**: Available on [GitHub](https://github.com/water-rs/waterui/tree/main/tutorial-book)
- **Issues**: Report problems or suggestions
- **Pull Requests**: Submit improvements

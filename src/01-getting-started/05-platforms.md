# Platforms and backends

> **In this chapter, you will:**
> - Learn which backend renders your app on each platform
> - Choose a backend in `Water.toml` and understand how the CLI resolves it
> - Pick the Android painter and the Hydrolysis style package
> - Know which platforms are supported, experimental, or not supported

Your view code does not change between platforms. A `vstack` is a `vstack`
on an iPhone, a Pixel and a Linux desktop, and layout and gestures behave the
same everywhere. What changes is the **backend**: the piece that turns your
view tree into pixels and input on one platform. Choosing a backend is a
deliberate decision you record once, not a fallback the framework reaches for
when something fails.

## Two ways to realize a component

WaterUI has two mainline backends.

- **Apple** realizes components with AppKit on macOS and UIKit on iOS. It uses
  the platform's own control where that control carries the whole semantic,
  and otherwise composes AppKit or UIKit views itself. With no style set, a
  `Toggle` is a checkbox `NSButton` on macOS and a `UISwitch` on iOS.
- **Hydrolysis** is self-drawn. It lays your tree out and paints it through
  Cherenkov, WaterUI's 2D engine, so a `Toggle` is drawn by the framework in
  the look of the selected style package. Hydrolysis is the backend for
  Android, Windows and the web, and an option on Linux and macOS.

Either way, the component's meaning (its semantics and its API) is defined
once in Rust. The backend decides how it looks on that platform, which is why
a toggle is a switch on one platform and a checkbox on another without any
change in your code.

## Platform support

| Tier | Platforms | What it means |
|---|---|---|
| 1 | macOS, iOS, Android | Tested on real devices |
| 2 | Windows, Linux | Built and tested in CI, failures fixed promptly |
| 3 | GTK, WinUI and TUI backends | Experimental |

Apple support is ARM64 only: `aarch64-apple-darwin`, `aarch64-apple-ios` and
`aarch64-apple-ios-sim`. Intel macOS and Intel iOS simulators are retired.
Default target selection leaves them out, and asking for one explicitly fails
with an unsupported-target error. x86 remains supported on other operating
systems. Windows does not support dynamic linking, so development builds link
statically.

## Which backend runs where

| Platform | Default backend | Also available |
|---|---|---|
| macOS | `apple` (AppKit) | `hydrolysis` |
| iOS and iOS simulator | `apple` (UIKit) | none |
| Android | `hydrolysis` | none |
| Linux | `gtk4` (experimental) | `hydrolysis` |
| Windows | `hydrolysis` | `winui` (experimental) |
| Web | `hydrolysis` | none |

Backend names are `apple`, `gtk4`, `hydrolysis` and `winui`. The Android
backend is Hydrolysis: Android's widgets are not used, and Hydrolysis draws
every component itself, as it does on the other platforms.

The GTK, WinUI and TUI backends are experimental. They live in their own
repositories, carry no stability guarantee, and their failures never hold up
the mainline backends. On Linux the CLI still defaults to `gtk4` for an
undeclared project; declare `hydrolysis` (next section) to use the mainline
renderer there.

ESP32 is not supported.

## Choose a backend in Water.toml

Declare the backend per platform with a `[platforms.<platform>]` table:

```toml
[platforms.linux]
backend = "hydrolysis"

[platforms.macos]
backend = "apple"
```

The platform keys are `ios`, `ios-simulator`, `macos`, `android`, `linux`,
`windows` and `web`. `water create` writes no declarations, so a new project
uses the defaults in the table above until you add one.

`water build`, `water run`, `water package` and previews all read the
declaration. The resolution rules are short:

1. If `Water.toml` declares a backend for the platform, that is the backend.
2. A `--backend` flag may **confirm** the declaration but never contradict it.
   A different value is an error before anything builds.
3. With no declaration, an explicit `--backend` is used.
4. With neither, the platform's default applies.

An unknown platform name, an unknown backend name, or a backend the platform
cannot run (`apple` on Linux, say) is an error that lists the valid names.
Nothing is substituted.

## Android: two painters

On Android, Hydrolysis draws through one of two painters:

| Painter | How it draws | Minimum |
|---|---|---|
| `gpu` (default) | A `SurfaceView`-backed GPU band | API 26 |
| `hwui` | `RenderNode` display lists through the host view hierarchy | API 34 |

The generated app uses the higher of the painter's minimum and the framework
floor. Select the painter in `Water.toml`:

```toml
[hydrolysis]
painter = "hwui"
```

Leaving it out means `gpu`. `water run` and `water package` also take
`--painter gpu` or `--painter hwui`, which overrides the `Water.toml` value
for that command and is an error on any platform other than Android. A
painter the pinned host cannot provide is an error when the project is
generated, never a silent switch to the other one.

## GPU API policy

GPU rendering compiles in one graphics API per platform, which keeps the
package small:

- **Linux and Android**: Vulkan only. On Android this covers the `gpu`
  painter; `hwui` draws through Android's own display lists.
- **Windows**: DX12 only.
- **macOS**: Metal.
- **Browsers**: WebGPU. GL exists only as a compatibility path for browsers
  without WebGPU, gets no significant investment, and is planned for removal
  once WebGPU is widespread.

No other API is selected, compiled in, or fallen back to. A device without the
required API fails with a clear unsupported-device error.

## Style packages

Hydrolysis itself is theme-independent. How a button or a switch looks comes
from a style package that implements Hydrolysis's public widget and theme
contracts. The Hydrolysis project the CLI generates draws with Material 3 from
the `hydrolysis-m3` package.

Previews use the same package. `--theme material3` names it; it is the only
value today and the default for a Hydrolysis preview, and passing `--theme`
to a preview on another backend is an error:

```bash
water preview my_view --backend hydrolysis --theme material3
```

Your view code never names a style package. With no modifiers, a backend
draws from theme tokens such as `Accent` and `Surface`, so the same tree gets
the platform's look without extra code. See
[Styling](../03-ui/11-styling.md) for the tokens.

## Platform-only capabilities

Some capabilities depend on an asset or service that only one platform can
supply. SF Symbols is the standard case: Apple ships the catalog, Android has
no OS icon catalog. Such a capability is documented as unsupported where it is
absent. WaterUI does not bundle a lookalike and call it "system".

For icons that must look the same everywhere, use an icon-set crate
(`waterui-icons-lucide`, `waterui-icons-material-icon` or
`waterui-icons-fontawesome7`), as described in
[Icons](../03-ui/12-icons.md).

This is different from a missing control. A control that a backend lacks, or
one that lacks part of its semantics, is a backend bug to fix, not a capability
to mark unsupported.

## Your code stays the same

Platform-specific behavior belongs in the backend, not in conditional app
code. The same `app` entry point, the one `water create` generates, serves
every platform:

```rust
use waterui::app::App;
use waterui::prelude::*;

fn root() -> impl View {
    vstack((
        text("Hello"),
        text("你好 / こんにちは / 안녕하세요"),
    ))
}

pub fn app(env: Environment) -> App {
    App::new(root, env)
}
```

Switching `[platforms.linux]` between `gtk4` and `hydrolysis` changes how this
renders, not what you write.

## Next steps

- Generate a project and see the manifest in full:
  [Project structure and Water.toml](04-project-structure.md).
- Render a view on the backend you chose:
  [Preview](../07-tools/01-preview.md).
- Learn how Hydrolysis and the Apple backend work inside:
  [Backends](../08-internals/04-backends.md) and
  [Rendering](../08-internals/01-rendering.md).
- Ship the result: [Packaging](../07-tools/07-packaging.md).

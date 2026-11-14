# The water CLI

> **In this chapter, you will:**
> - Install the prebuilt `water` binary and keep it current with `water update`
> - Create, run, build and package a project with the platform and backend flags
> - Choose a framework channel (`dev`, `nightly`, `stable`) for a project
> - Find the command that owns preview, testing, benchmarks, the inspector and MCP

Every WaterUI project is entry-owning: WaterUI owns the program entry, and
`water` generates and manages the Xcode, Gradle and desktop host projects for
you. You never edit those by hand, and you never invoke `xcodebuild`, `gradle`
or `adb` directly for an app. `water` is the one entry point for scaffolding,
building, launching, previewing, testing and packaging.

## Install

Install the prebuilt binary. Nothing is compiled on your machine.

```bash
# macOS and Linux
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/water-rs/waterui/releases/download/cli-latest/waterui-cli-installer.sh | sh
```

```powershell
# Windows
powershell -ExecutionPolicy Bypass -c "irm https://github.com/water-rs/waterui/releases/download/cli-latest/waterui-cli-installer.ps1 | iex"
```

Then check your toolchains:

```bash
water doctor          # report what is missing
water doctor --fix    # install what it can
```

`water doctor --fix` asks before an install that changes the system outside
`~/.water`, such as the Visual Studio Build Tools; `--yes` answers that
prompt when nobody is at the terminal.

`water update` re-runs the newest release's installer over a binary the
installer put in place, and `water update --check` only reports the newest
release. A `water` that came from Homebrew or `cargo install` belongs to that
tool, so update it there.

If you work on the framework itself, install the CLI from your checkout
instead:

```bash
cargo install --path cli --locked
```

Contributors only: everyone else should use the installer, because a source
install compiles the CLI and its whole dependency graph.

## Create a project

```bash
water create "My App"
cd my-app
water run
```

The name becomes the folder (`my-app`). There are no modes or package types to
choose; `create` always produces the same entry-owning shape, described in
[Project structure](04-project-structure.md).

| Flag | Meaning |
| --- | --- |
| `--bundle-id <ID>` | Bundle identifier. Defaults to `dev.waterui.<name>`. |
| `--template app\|web` | `app` (default) is the standard Rust shell. `web` adds a `web/` Vite frontend mounted through `include_web!`. |
| `--channel dev\|nightly\|stable` | Framework channel (see below). Defaults to `stable`. |
| `--waterui-path <PATH>` | Use a local WaterUI checkout. Conflicts with `--channel` and `--framework-manifest`. |
| `--framework-manifest <FILE>` | Pin to a certified `framework.json` on disk. Conflicts with `--channel` and `--waterui-path`. |
| `--package-manager bun\|pnpm\|npm\|yarn`, `--vite-template <NAME>` | Frontend options for `--template web`. The package manager defaults to `bun`. |

`water init` does the same for the current directory, taking the same
`--bundle-id`, `--channel`, `--waterui-path` and `--framework-manifest` flags. With `--web new` it
scaffolds a Vite frontend into `web/`; with `--web <path>` it adopts an
existing web project, either copied into `web/` or referenced in place
(`--web-mode copy|reference`).

## Framework channels

A channel decides which WaterUI revision a project builds against. It is not a
Rust toolchain channel.

| Channel | What you get |
| --- | --- |
| `stable` | Formal crates.io releases with compatible native backends. The default. |
| `nightly` | An immutable revision whose full integration suite passed, with the tested backend combination. With no certified nightly, selecting it fails rather than falling back to `dev`. |
| `dev` | The integration branch, resolved to an exact commit. Compilation-checked, not test-certified. |

`create` persists the channel and the exact resolved versions. Ordinary builds
never upgrade them silently. To inspect or change the selection later:

```bash
water channel                 # show the current selection
water channel nightly         # select or update a channel
water channel dev --rev <SHA> # pin dev to an exact commit
```

## Run, build, package

```bash
water run                                   # the host platform
water run --platform ios                    # an iPhone or the iOS Simulator
water run --platform android --device <ID>
water run --platform linux --backend hydrolysis
water build --platform macos --release
water package --platform android --arch arm64 --distribution
```

`--platform` takes `ios`, `android`, `macos`, `linux`, `windows` and `web`
(`web` for `run` and `package`). `build` and `package` also accept
`ios-simulator`. For `run`, `macos`, `linux` and `windows` mean this machine:
`--platform linux` on a Mac is an error.

Without `--backend` or a `[platforms.<platform>]` declaration in `Water.toml`,
Apple platforms use `apple`; Android, Windows and the web use `hydrolysis`;
and Linux uses the experimental `gtk4`, which is why the example above passes
`--backend hydrolysis`. The platform tiers and what each backend realizes are
in [Platforms](05-platforms.md).

Flags shared across `run`, `build` and `package`:

| Flag | Meaning |
| --- | --- |
| `-p, --platform` | Target platform. `run` defaults to the host; `build` and `package` require it. |
| `-b, --backend apple\|hydrolysis\|gtk4\|winui` | Must agree with any platform declaration in `Water.toml`. `gtk4` and `winui` are experimental and ask for confirmation (`-y` skips it). |
| `--path <DIR>` | Project directory. Defaults to `.`. |
| `--release` / `--debug` | Optimization profile. `run` and `build` apply a light optimization by default to backends whose per-frame cost is high, and `--debug` turns it off. `package` builds `release` unless you pass `--debug`. |
| `--offline` | Same as `cargo build --offline`. A declared font missing from the font cache then fails the command instead of being fetched. |

`run` adds:

| Flag | Meaning |
| --- | --- |
| `-d, --device <ID>` | iOS device name, identifier or UDID; Android serial or AVD name. Without it the last-used device wins, then a single unambiguous candidate, then a picker. |
| `--painter gpu\|hwui` | The painter the Android Hydrolysis host draws with. Only with `--platform android --backend hydrolysis`; `[hydrolysis] painter` in `Water.toml` is the project default. |
| `--logs error\|warn\|info\|debug\|verbose` | Streams device logs at or above that level and sets the application's own log level, so `debug` shows its `tracing::debug!` output. |
| `--native-logs` | Include platform logs such as `NSLog`, not only WaterUI's. |
| `--env <NAME>` | Export a variable from your shell into the app. Repeatable. |
| `--profiling` | Release optimization with symbols kept for a profiler. |
| `--no-dev-server` | Do not start the frontend dev server for `include_web!` mounts. |
| `--tui` | Run in this terminal with the experimental TUI backend. |

`build` adds `-a, --arch` (`arm64`, `x86-64`, `armv7`, `x86`; Apple targets are
ARM64 only, Android defaults to `arm64`) and `--output-dir` to copy the built
library. `package` adds `--distribution` for store builds, `--unsigned` for
hosts with no signing identity, `--device <UDID>` to provision an iOS build
for one device, `--painter` as on `run`, and `--arch` (long form only) as a
comma-separated list that an Android package requires. Signing, store builds
and the artifacts each platform produces are covered in
[Packaging](../07-tools/07-packaging.md).

Intel Apple targets are retired: `--arch x86-64` (or any non-ARM64
architecture) on an Apple platform fails with an error saying Apple targets
only support `arm64`.

## Preview and test a view

```bash
water preview my_view --platform macos --output preview.png
water preview --expr 'text("Hello")' --frame 390x844
water preview test --all --code-file checks.rs
```

`preview` renders a `#[preview]` function, or with `--expr` an expression
returning `impl View` with the prelude in scope, to a PNG without running the
app. `-f, --frame` (`WIDTHxHEIGHT`, default `375x667`), `-o, --output`
(default `preview.png`), `--content-timeout`, `--scenario` and `--theme`
shape the capture. `preview test` runs semantic assertions against the same
render. See [Preview](../07-tools/01-preview.md) and
[Testing](../07-tools/03-testing.md).

## Benchmarks, inspector, MCP

```bash
water bench [FILTER]       # run #[waterui::bench] functions
water inspector            # attach the inspector to a debug build
water mcp                  # serve the app to an agent over MCP
```

- `bench` takes `--samples`, `--warmups`, `--repetitions`, `--format
  human|json|html`, and `--max-*` caps that tighten a bench's own budgets. See
  [Benchmarks](../07-tools/04-benchmarks.md).
- `inspector` attaches to a debug build on this machine, or to a remote target
  with `--platform` and `--target host:port`. See
  [Inspector](../07-tools/05-inspector.md).
- `mcp` speaks MCP on stdio, with `--viewport` (default `390x844`) and
  `--scale` (default `2`). See [MCP](../07-tools/06-mcp.md).

## Devices

```bash
water devices                        # all platforms
water devices --platform android
water device capture --id <ID> -o shot.png
water device tap --id <ID> --x 100 --y 200 --diff
water device swipe --id <ID> --from 100,600 --to 100,200
water device text --id <ID> --input "你好 こんにちは 안녕하세요"
water device describe --id <ID>
```

`devices` lists iOS, Android and macOS targets. `device` drives one of them:
`capture` takes a screenshot (on macOS it can also select a process with
`--pid`), `tap` and `swipe` send gestures, `text` types, and `describe` dumps
the on-screen elements for automation. `--diff` on a gesture captures before
and after images. The device identifier is a UDID for iOS, a serial for
Android, and `local` for macOS; `describe` takes only an iOS or Android
identifier. Device automation is the subject of
[Automation](../appendix/automation.md).

## Maintenance

| Command | Purpose |
| --- | --- |
| `water clean` | Remove build artifacts. `-b` picks a backend, `-r` walks every project under `--path`, `--global-cache` cleans `~/.water/build_cache`. |
| `water gc build-cache` | Remove stale managed caches. `--dry-run` only reports; `--shared-target` removes the shared Cargo target directory now. |
| `water fetch` | Download the project's declared fonts into the font cache. |
| `water completions <SHELL>` | Print a completion script for `bash`, `zsh`, `fish`, `elvish` or `powershell`, for example `water completions zsh > ~/.zfunc/_water`. |
| `water update [--check]` | Replace the CLI binary, or report the newest release. |

## Machine-readable output

`--json` is global and goes before or after the subcommand. Status, warning
and error messages become one JSON object per line on stdout, and `devices`,
`device` and `doctor` print their results as JSON:

```bash
water --json devices
```

## Where things go wrong

When a build fails, start with `water doctor` to find a missing or outdated
toolchain. When the app builds but misbehaves, `water run --logs debug` shows
its `tracing::debug!` output.
The [troubleshooting appendix](../appendix/troubleshooting.md) lists the common
failures.

Next: [Setup](02-setup.md) installs the platform toolchains `doctor` asks for,
and [Your first app](03-first-app.md) builds something with what you just
installed.

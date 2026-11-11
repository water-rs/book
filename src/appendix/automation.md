# Automation and CI

> **In this chapter, you will:**
>
> - Script the `water` CLI with `--json` and `-y` so no step waits for a prompt
> - Run view tests and benchmarks in CI with `water preview test`, `#[waterui::test]` and `water bench`
> - Pin the framework channel so a CI run never drifts
> - Build a GitHub Actions workflow that installs the prebuilt CLI and caches compilation

A CI runner has no terminal to answer a prompt and no person to read colored output.
The `water` CLI is built for that case: every prompt has a flag that answers it, every
command that lists something can emit JSON, and the framework revision a project builds
against is recorded in the project rather than looked up at build time.

## Deterministic CLI runs

### JSON output

`--json` is a global flag. It switches status, error, and success messages from
human-readable text to machine-readable JSON:

```bash
water --json devices
```

Pipe it through `jq` to pull out fields:

```bash
# The first available iOS simulator
water --json devices --platform ios \
  | jq -r 'first(.ios[] | select(.kind == "simulator" and .available)) | .udid'
```

The `devices` payload carries a `type` and a `platform` field plus one section per
scanned platform:

- `ios`: a list of `name`, `udid`, `state`, `available`, and `kind` (`"device"` for a
  paired physical device, `"simulator"` otherwise). Physical devices come first.
- `android`: an object with `emulators` (`name`, `running`) and `devices` (`id`, `abi`).
- `macos`: a single `{ "id": "local", "name": "Current Machine" }` entry.

Sections that were not scanned are omitted.

### Non-interactive mode

A command that can prompt has a `--yes` flag, and an unattended pipeline must pass it:

| Command | What `--yes` answers |
|---|---|
| `water build`, `water run`, `water package` (`-y`) | The confirmation an experimental backend (GTK4, WinUI) asks for |
| `water clean --recursive`, `water clean --global-cache` (`-y`) | The confirmation before deleting build caches |
| `water doctor --fix --yes` (no short form) | Windows installs that modify the system outside `~/.water`: Visual Studio Build Tools, the LLVM MSI, the long-paths setting |

Without `--yes`, a non-interactive run fails instead of hanging: an experimental backend
stops with an error that asks for `--yes`, and `water doctor --fix` skips the
system-wide installs and reports them. A command that has to pick a device, such as
`water run`, also fails on a non-interactive run when more than one candidate is
present, listing them; pass `--device` to name one.

## Scripting with water commands

### Building for several platforms

`--platform` is a flag, not a positional argument. `water build` accepts `ios`,
`ios-simulator`, `android`, `macos`, `linux`, and `windows`:

```bash
#!/bin/bash
set -euo pipefail

water build --platform ios-simulator
water build --platform android --arch arm64
water build --platform linux --backend hydrolysis
```

Useful flags from `water build`:

- `--backend <apple|gtk4|hydrolysis|winui>` selects a backend; it must agree with any
  platform declaration in `Water.toml`. Without either, each platform uses its default:
  Apple on Apple platforms, Hydrolysis on Android and Windows, and GTK4 on Linux. GTK4
  and WinUI are experimental, so a Linux build that keeps the default needs `-y` on a
  non-interactive run.
- `--arch <arm64|x86-64|armv7|x86>` selects the architecture. Apple targets support only
  `arm64`; Android defaults to `arm64`.
- `--release` builds optimized output, `--debug` builds fully unoptimized.
- `--offline` is the same switch as `cargo build --offline`; `CARGO_NET_OFFLINE` is
  honored the same way. A declared font missing from the font cache then fails the
  build instead of being fetched. Run `water fetch` beforehand to fill the cache.
- `--path <dir>` points at the project, and `--output-dir <dir>` copies the built library
  out (Apple backend and Android builds only).

Apple targets are ARM64 only: `aarch64-apple-darwin`, `aarch64-apple-ios` and
`aarch64-apple-ios-sim`. Intel Apple targets are unsupported.

### Device discovery

```bash
DEVICE_ID=$(water --json devices --platform ios \
  | jq -r 'first(.ios[] | select(.kind == "simulator" and .available)) | .udid')
water run --platform ios --device "$DEVICE_ID"
```

`water run` stays attached to the app and streams its logs until it is stopped, so give
a CI step that runs it a `timeout-minutes` bound.

### Release packages

`water package` builds the `release` profile unless you pass `--debug`, so
`--release` only states the default. It is the command for artifacts you ship; see
[Packaging](../07-tools/07-packaging.md) for signing and stores:

```bash
water package --platform ios --backend apple --unsigned
water package --platform android --backend hydrolysis --arch arm64 --unsigned
water package --platform linux --backend gtk4 -y
```

`--arch` is required for Android and takes a comma-separated list (`--arch arm64,x86-64`).
`--unsigned` leaves a release build unsigned for a host with no signing identity (a CI
runner); it applies to iOS device builds with the Apple backend and Android builds with
the Hydrolysis backend. Sign the artifact with `codesign` or `apksigner` afterwards. For
a signed Android package, `[signing.android]` in `Water.toml` names the keystore and key
alias, and the passwords come from `WATERUI_ANDROID_STORE_PASSWORD` and
`WATERUI_ANDROID_KEY_PASSWORD`, stored as CI secrets; they are never written into the
project.

## Testing in CI

WaterUI has three layers of automated checks, each with its own chapter:

1. **Semantic tests.** `#[waterui::test]` functions drive a view through the
   accessibility tree and run under `cargo nextest run`. See
   [Testing](../07-tools/03-testing.md).
2. **Preview tests.** `water preview test` runs the same kind of assertions against a
   `#[preview]` function without writing a test target.
3. **Benchmarks.** `#[waterui::bench]` functions run under `water bench` against
   frame-time and counter budgets. See [Benchmarks](../07-tools/04-benchmarks.md).

### Semantic tests with nextest

```bash
cargo nextest run
cargo test --doc
```

`cargo nextest` runs each test in its own process, so a test must not rely on a
sibling's global setup. It cannot run doc-tests; `cargo test --doc` covers those.

Because the assertions run against the accessibility tree, they are
simultaneously interaction tests and accessibility tests. Prefer them to pixel
diffs: a snapshot fails on any antialiasing change, while a semantic query fails only
when the UI changed meaning.

When `WATERUI_TEST_ARTIFACTS_DIR` is set, `waterui-testing` writes snapshot images
beneath it as `<suite>/<case>/<stage>.png`, ready to upload as a workflow artifact.

### Preview tests

`water preview test` drives a preview through the accessibility tree. `--all` discovers
and tests every `#[preview]` function in the crate:

```bash
water preview test --all --path ./app
```

Other flags: `--platform` (the native preview platform by default), `--theme` (the
Hydrolysis style package, `material3` by default), `--frame WIDTHxHEIGHT` (`375x667` by
default), `--expr` to treat the target as an expression returning `impl View`, and
`--offline`.

The automation body is Rust, supplied inline with `--code` or from a file with
`--code-file`; it receives `app: &mut waterui_testing::SemanticApp`:

```bash
water preview test my_form --code-file ci/checks/my_form.rs --path ./app
```

### Benchmarks

`water bench` runs every `#[waterui::bench]` function. A positional argument filters by
bench name; `--warmups`, `--samples` and `--repetitions` set the measurement shape:

```bash
water bench --path ./app --format json --output bench.json
```

CI can tighten the budgets declared in the attributes with `--max-p95-us`,
`--max-mean-us`, `--max-rebuild-ratio`, `--max-scene-layers`,
`--max-gpu-surface-layers` and `--max-clip-layers`. `--gha <path>` writes a
`github-action-benchmark` JSON file.

Timing on a shared CI runner is noisy, so gate a merge on the deterministic counters
(rebuild ratio, layer counts) and treat wall-clock results from shared runners as
informational. Wall-clock time and energy come from quiet real devices.

### Reviewing images

If a change affects pixels, render it with `water preview` and look at the image:

```bash
water preview my_button --platform macos --path ./app --output current/button.png
```

Judge rendered output by reading the image, never by a pixel count, brightness
threshold or similar heuristic. Rendered output varies between GPUs, drivers and
antialiasing paths, so a byte-level comparison fails on differences nobody can see and
tells you nothing about the ones that matter. Upload the PNGs as workflow artifacts and
review them in the pull request.

## Pinning the framework

A project records its framework channel in `Water.toml`. The CLI resolves a channel
when you create the project or change it explicitly, then persists the channel and the
exact resolved versions. An ordinary `water build` or `water run` never upgrades them,
so a CI run builds what the commit declares.

```bash
water create my-app --channel nightly
water channel                  # show the current selection
water channel nightly          # select a channel explicitly
water channel dev --rev <sha>  # pin dev to one commit of its history
```

| Channel | Meaning |
|---|---|
| `stable` | Published crates.io releases (the default) |
| `nightly` | An immutable revision whose required integration suite passed |
| `dev` | The integration branch, resolved to an exact commit; compilation-checked, not test-certified |

`--rev` is valid only beside `dev`. `water channel nightly` fails when no certified
nightly exists; it does not fall back to `dev`. Commit the resulting `Water.toml` so
every runner uses the same revision. Run `water channel` in a CI step if you want the
selection in the job log.

## GitHub Actions

Install the prebuilt CLI with the release installer rather than `cargo install`, which
compiles it from source on every run:

```yaml
name: CI
on: [push, pull_request]

env:
  GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - uses: taiki-e/install-action@nextest
      - run: cargo fmt --check
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo nextest run
      - run: cargo test --doc

  build-ios:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - run: curl --proto '=https' --tlsv1.2 -LsSf https://github.com/water-rs/waterui/releases/download/cli-latest/waterui-cli-installer.sh | sh
      - run: water doctor --fix --yes
      - run: water build --platform ios-simulator

  build-android:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: curl --proto '=https' --tlsv1.2 -LsSf https://github.com/water-rs/waterui/releases/download/cli-latest/waterui-cli-installer.sh | sh
      - run: water doctor --fix --yes
      - run: water build --platform android --arch arm64
```

On Windows, the installer is `waterui-cli-installer.ps1` run through `powershell`.

A few points on the workflow:

- **`water doctor --fix --yes`.** `doctor` checks the Rust toolchain and required
  targets, the platform SDKs, simulators, Android tooling, and Linux system packages. With
  `--fix` it installs what it can, such as missing Rust targets, `sccache` and
  `cargo-nextest`, and reports the rest; `--yes` lets it run the Windows installs that
  touch the system outside `~/.water`. It exits successfully even when something is still
  missing, so read its report in the job log; the build step that follows is the one
  that fails.
- **The toolchain.** The framework declares its minimum Rust as `rust-version` (1.99 at
  the revision this book tracks). `water doctor` checks the toolchain against the highest
  of the CLI's, your project's and the selected framework's `rust-version`, and `--fix`
  updates it. A plain `cargo` job on an older pinned toolchain fails when Cargo reads
  that requirement.
- **`GITHUB_TOKEN`.** The CLI queries the GitHub REST API to resolve framework
  revisions. It reads `WATERUI_GITHUB_TOKEN` first and then `GITHUB_TOKEN`.
  Unauthenticated requests share sixty an hour across every machine behind one address,
  so the workflow passes the job's token to every step.
- **Self-update.** `water update` upgrades the CLI in place and `water update --check`
  reports the newest release. It reads `WATERUI_GITHUB_TOKEN` for the same reason.

### Caching

The plain `cargo` job in the workflow caches with `Swatinem/rust-cache`, which keeps the
Cargo registry and the `target` directory between runs. WaterUI's own CI does the same:
it measured a 2–8% hit rate for a GitHub-cache-backed `sccache` on hosted runners, while
that cache crowded the repository's Actions cache quota.

The `water` CLI routes its own Cargo builds through `sccache` whenever it finds one,
either the copy `water doctor --fix` installs under `~/.water/tools` or one on `PATH`.
Caching works because the CLI builds its generated crate without incremental
compilation, since `sccache` refuses to cache an incremental compile. `sccache` keeps its
cache in a local directory by default, so it pays off on a long-lived self-hosted runner.
The CLI leaves the wrapper alone when `RUSTC_WRAPPER` is already set, and skips `sccache`
when `WATERUI_DISABLE_SCCACHE` is `1`, `true`, `yes` or `on`.

WaterUI keeps managed backend builds under `~/.water/build_cache`. On a long-lived
self-hosted runner, prune stale entries, or inspect them first:

```bash
water gc build-cache --path ./app --dry-run
water gc build-cache --path ./app
water gc build-cache --shared-target   # drop the shared Cargo target directory now
```

`water clean` removes WaterUI build artifacts for the project and leaves your Cargo
`target` directory alone; add `--global-cache` for the managed cache, or
`--recursive -y` to sweep every project under a path.

## Environment variables

These are the variables the CLI and `waterui-testing` read, confirmed in their source:

| Variable | Purpose |
|---|---|
| `CARGO_NET_OFFLINE` | Same effect as `--offline` on `build`, `run`, `package`, `preview` and `preview test` |
| `WATERUI_GITHUB_TOKEN` | GitHub token for framework resolution and `water update`; `GITHUB_TOKEN` is the fallback for resolution |
| `WATERUI_LOG` | Log level the CLI hands to the app's `tracing` filter; set by `--logs` on `water run`. `RUST_LOG` overrides it when set |
| `WATERUI_TEST_ARTIFACTS_DIR` | Root directory for `waterui-testing` snapshot images |
| `WATERUI_ANDROID_STORE_PASSWORD`, `WATERUI_ANDROID_KEY_PASSWORD` | Passwords for the Android release keystore and signing key |
| `WATERUI_SKIP_RUST_BUILD` | Set by the CLI on native builds so Xcode and Gradle do not rebuild the Rust library; do not set it yourself |
| `WATERUI_ANDROID_UNSIGNED`, `WATERUI_ANDROID_ABIS` | Set by the CLI on the Gradle build it runs (`--unsigned`, `--arch`); they are outputs, not knobs |
| `RUSTC_WRAPPER` | A compiler wrapper you choose; when it is set, the CLI uses it instead of routing builds through `sccache` itself |
| `WATERUI_DISABLE_SCCACHE` | `1`, `true`, `yes` or `on` stops the CLI from routing builds through `sccache` |

`water test-runner` reads `WATERUI_IOS_SIM_UDID` and the `WATERUI_IOS_DEVICE*` variables.
It is the hidden Cargo and nextest target runner for Apple destinations, not a command
you call by hand.

## Debugging in CI

```bash
water run --platform ios --logs debug
```

`--logs debug` streams device logs at or above the given level. Apple platforms route
them through `os_log`, Android through `logcat`, and everything else to stderr. Add
`--native-logs` when you need the platform's own output alongside WaterUI's. It is
noisy, but necessary when you debug native code. Diagnostics from your own code go
through `tracing`, so they appear at the level you choose.

If your project includes an mdBook, `mdbook test` compiles the Rust code blocks in your
markdown; blocks marked `rust,ignore` are skipped.

## What's next

If CI is green but something still misbehaves, the [Troubleshooting](troubleshooting.md)
appendix covers the common failures. To write the tests these jobs run, continue with
[Testing](../07-tools/03-testing.md) and [Benchmarks](../07-tools/04-benchmarks.md).

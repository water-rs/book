# Automation and CI

> **In this chapter, you will:**
>
> - Script the `water` CLI for deterministic, non-interactive builds
> - Run preview-based semantic and performance tests in CI
> - Set up a multi-platform GitHub Actions workflow
> - Debug CI failures with structured logging

## Deterministic CLI runs

### JSON output

`--json` is a global flag. It switches every status, error, and success message
from human-readable ANSI to machine-readable JSON:

```bash
water --json devices
```

Pipe it through `jq` to pull out fields:

```bash
# The first iOS simulator's identifier
water --json devices | jq -r '.ios[0].udid'
```

The `devices` payload has one section per platform — `ios`, `android`
(`emulators` and `devices`), `macos`, and `esp32` — and omits sections that were
not scanned.

### Non-interactive mode

Subcommands that may prompt accept `-y`/`--yes`:

```bash
water clean -y
water backend remove apple -y
```

Not every command prompts; check `water <command> --help`. A forgotten prompt
hangs the pipeline.

## Scripting with water commands

### Building for multiple platforms

`--platform` is a flag, not a positional argument:

```bash
#!/bin/bash
set -euo pipefail

water build --platform ios-simulator
water build --platform android
water build --platform linux
water build --platform esp32c3
```

Accepted values are `ios`, `ios-simulator`, `android`, `macos`, `linux`,
`windows`, `esp32s3`, and `esp32c3`.

### Device discovery

```bash
DEVICE_ID=$(water --json devices | jq -r '.ios[0].udid')
water run --platform ios --device "$DEVICE_ID"
```

### Preview rendering

```bash
water preview my_component \
    --platform macos \
    --path ./app \
    --output previews/my_component.png
```

This builds the project as a dylib, loads it into a preview host, and captures
the rendered output — the same command whether you are eyeballing a change
locally or regenerating snapshots before a diff.

## Preview-based testing

Beyond rendering an image, `water preview` has two subcommands built for CI.

### Semantic assertions

`water preview test` drives a preview through WaterUI's accessibility tree.
`--all` discovers and runs every `#[preview]` function in the crate:

```bash
water preview test --all --theme material3 --platform macos --path ./app
```

The automation body is Rust, supplied inline with `--code` or from a file with
`--code-file`, and receives `app: &mut waterui_testing::SemanticApp`:

```bash
water preview test my_form \
    --theme material3 \
    --code-file ci/checks/my_form.rs
```

Because the assertions run against the accessibility tree, they are
simultaneously interaction tests and accessibility tests. Prefer these to pixel
diffs: a snapshot fails on any antialiasing change, while a semantic query fails
only when the UI actually changed meaning.

### Performance measurement

`water preview perf` profiles a preview through the offscreen GPU pipeline and
emits a JSON report:

```bash
water preview perf --all \
    --theme material3 \
    --warmups 10 --samples 120 --repetitions 7 \
    --path ./app
```

The automation body here receives `perf: &mut waterui_testing::PerfApp<_, _, _>`.
Add `--flamegraph <path>` to also write a CPU call-stack SVG.

### Image comparison

If you do need pixel comparison, generate and diff explicitly:

```bash
water preview my_button --platform macos --path ./app --output current/button.png
compare -metric RMSE reference/button.png current/button.png diff/button.png
```

## GitHub Actions

```yaml
name: CI
on: [push, pull_request]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo fmt --check
      - run: cargo clippy -- -D warnings
      - run: cargo test

  build-ios:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-apple-ios-sim
      - run: cargo install waterui-cli
      - run: water doctor
      - run: water build --platform ios-simulator

  build-android:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-linux-android
      - run: cargo install waterui-cli
      - run: water build --platform android
```

WaterUI requires rustc 1.95 or newer, so a pinned toolchain older than that will
fail before any WaterUI-specific step runs.

### Environment validation

Run `water doctor` early. It checks the Rust toolchain and required targets,
Xcode and the macOS/iOS SDKs, iOS simulators, Android run targets, GTK4, and
Linux system packages. `water doctor --fix` installs what it can — missing Rust
targets, for instance — and reports the rest.

### Caching

```yaml
- uses: actions/cache@v4
  with:
    path: |
      ~/.cargo/registry
      ~/.cargo/git
      target
    key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
```

Do not cache Xcode derived data or Gradle build directories; they are fragile
across runners and produce failures that are hard to diagnose.

WaterUI keeps managed backend builds under `~/.water/build_cache`. On a long-
lived self-hosted runner, prune stale entries:

```bash
water gc build-cache --path ./app
```

## Testing

```bash
cargo test
cargo test -p waterui-core
```

Component tests written with `#[waterui::test(...)]` expand to ordinary `#[test]`
functions, so they need no custom runner. When `WATERUI_TEST_ARTIFACTS_DIR` is
set, snapshot artifacts are written beneath it as `<suite>/<case>/<stage>.png`,
ready to upload as a workflow artifact.

If your project includes an mdBook, `mdbook test` compiles the Rust code blocks
in your markdown; blocks marked `rust,ignore` are skipped.

## Release builds

`water package` requires an explicit `--backend`, and `--release` selects
optimized output:

```bash
water package --platform ios     --backend apple      --release
water package --platform android --backend android --arch arm64 --release
water package --platform linux   --backend gtk4       --release
```

`--arch` is required for Android and accepts a comma-separated list
(`arm64`, `x86_64`, `armv7`, `x86`). Add `--distribution` for App Store or Play
Store packaging.

## Clean builds

```bash
water clean
```

This removes WaterUI-specific build artifacts. Avoid `cargo clean`, which wipes
the whole target directory and forces a full rebuild.

## Environment variables

| Variable                                | Purpose                                                             |
|-----------------------------------------|---------------------------------------------------------------------|
| `RUST_LOG`                              | Tracing filter for the runtime (e.g. `debug`)                       |
| `WATERUI_DISPATCH_DEBUG`                | Logs the view dispatch tree in any backend using `ViewDispatcher`   |
| `WATERUI_TEST_ARTIFACTS_DIR`            | Root directory for `waterui-testing` snapshot artifacts             |
| `WATERUI_HYDROLYSIS_RENDER_DIAG`        | Per-frame render diagnostics from Hydrolysis                        |
| `WATER_HYDROLYSIS_FORCE_FALLBACK_ADAPTER` | Allows a software wgpu adapter — diagnostics only, never in a release job |

## Debugging in CI

```bash
water run --platform ios --logs debug
RUST_LOG=debug cargo test
```

`--logs debug` streams device logs at or above the given level, showing view
dispatch, signal updates, and FFI calls. Apple platforms route these through
`os_log`, Android through `logcat`, and everything else to stderr. Add
`--native-logs` when you need the platform's own output (`NSLog`, `print`)
alongside WaterUI's — noisy, but necessary when debugging native code.

## FFI header verification

This one is for WaterUI contributors only; application authors never need it.
`ffi/waterui.h` is checked in and generated, never hand-written, so CI verifies
it has not drifted:

```bash
cargo run --bin generate_header --features cbindgen --manifest-path ffi/Cargo.toml
git diff --exit-code ffi/waterui.h
```

## What's next

If CI is green but something still misbehaves, the
[Troubleshooting](troubleshooting.md) appendix covers the common failures.

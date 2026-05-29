# Automation and CI

> **In this chapter, you will:**
>
> - Script the `water` CLI for deterministic, non-interactive builds
> - Set up GitHub Actions workflows for multi-platform CI
> - Run tests, validate FFI headers, and generate preview-based visual tests
> - Debug CI failures with structured logging

WaterUI projects can be fully automated for continuous integration, deployment,
and development workflows. This appendix gives you the practical recipes to make that happen.

## Deterministic CLI Runs

The `water` CLI is designed for both interactive and automated use. When running
in scripts or CI, use these flags to ensure deterministic behavior.

### JSON output

`--json` is a global flag on `water` that switches every status, error, and
success message to machine-readable JSON instead of human-readable ANSI
output:

```bash
water --json devices
```

Pipe through `jq` to parse fields:

```bash
# Inspect the first iOS simulator's identifier
water --json devices | jq '.ios[0].udid'
```

### Non-interactive mode

Subcommands that may prompt for confirmation accept `-y`/`--yes` to
auto-confirm in scripts:

```bash
water clean -y
water backend remove apple -y
```

Check `water <command> --help` to see exactly which subcommands accept
`--yes`; not every command prompts.

> **Tip:** In CI, set `CI=1` if your scripts depend on it, and pass `-y` to
> any command that may prompt. A forgotten prompt will hang the pipeline.

## Scripting with Water Commands

### Building for Multiple Platforms

```bash
#!/bin/bash
set -euo pipefail

# Build for each target you care about. The platform is a flag, not a
# positional argument.
water build --platform ios-simulator
water build --platform android
water build --platform linux
```

### Device discovery

```bash
# Capture the device list
water --json devices > devices.json

# Run on a specific device
DEVICE_ID=$(water --json devices | jq -r '.ios[0].udid')
water run --platform ios --device "$DEVICE_ID"
```

### Preview generation

Generate preview images for visual regression testing:

```bash
water preview my_component \
    --platform macos \
    --path ./app \
    --output previews/my_component.png
```

This builds the project as a dylib, loads it into a preview host, and captures
the rendered output. Use the same command in CI to regenerate "current"
snapshots before diffing them against your committed reference images.

## CI/CD Integration Patterns

### GitHub Actions

Here is a minimal GitHub Actions workflow for a WaterUI project:

```yaml
name: CI
on: [push, pull_request]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Check formatting
        run: cargo fmt --check
      - name: Run clippy
        run: cargo clippy -- -D warnings
      - name: Run tests
        run: cargo test

  build-ios:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-apple-ios-sim
      - name: Install water CLI
        run: cargo install waterui-cli
      - name: Doctor check
        run: water doctor
      - name: Build for iOS Simulator
        run: water build --platform ios-simulator

  build-android:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-linux-android
      - name: Install water CLI
        run: cargo install waterui-cli
      - name: Build for Android
        run: water build --platform android
```

### Environment Validation

Always run `water doctor` at the start of your CI pipeline to verify the
environment is correctly configured:

```bash
water doctor
```

This checks for:
- Rust toolchain version and required targets.
- Platform SDKs (Xcode, Android SDK, GTK4 development libraries).
- Required tools (`cargo`, `rustc`, `xcodebuild`, `adb`, `gradle`).

Use `water doctor --fix` to automatically install missing components where
possible (e.g., adding Rust targets via `rustup`).

### Caching

Cache the Cargo build directory to speed up CI builds:

```yaml
- uses: actions/cache@v4
  with:
    path: |
      ~/.cargo/registry
      ~/.cargo/git
      target
    key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
```

> **Warning:** Do not cache platform-specific build artifacts (Xcode derived data, Gradle build directories) as they are more fragile and can cause hard-to-debug failures.

## Testing

### Rust Unit and Integration Tests

Run the full test suite:

```bash
cargo test
```

Run tests for a specific crate:

```bash
cargo test -p waterui-core
cargo test -p waterui-layout
cargo test -p waterui-ffi
```

### Book Validation

If your project includes an mdBook (like this book), validate that all code
examples compile:

```bash
mdbook test
```

This extracts Rust code blocks from markdown files and runs them as doctests.
Code blocks marked with `rust,ignore` are skipped.

### FFI Header Verification (WaterUI contributors only)

If you are contributing to WaterUI itself, your CI should verify the checked-in
C header is up to date. Application authors do not need this step.

```bash
# Generate the header
cargo run --bin generate_header --features cbindgen --manifest-path ffi/Cargo.toml

# Check for differences
git diff --exit-code ffi/waterui.h
```

A drift in `ffi/waterui.h` fails CI, reminding the developer to regenerate and
commit it.

### Preview-Based Visual Tests

For visual regression testing, generate preview images and compare them:

```bash
# Generate current previews
water preview my_button --platform macos --path ./app --output current/button.png
water preview my_card   --platform macos --path ./app --output current/card.png

# Compare against reference images (using any image diff tool)
# For example, with ImageMagick:
compare -metric RMSE reference/button.png current/button.png diff/button.png
```

## Build Automation Scripts

### Regenerating FFI Bindings (WaterUI contributors only)

If you have modified an FFI API in your fork of WaterUI, regenerate the C
header from the upstream `waterui` checkout:

```bash
cargo run --bin generate_header --features cbindgen --manifest-path ffi/Cargo.toml
cargo build -p waterui-ffi
```

### Release Builds

For production releases, the `water` CLI handles platform-specific packaging.
`--platform` is a flag and `--release` switches to optimized output:

```bash
water package --platform ios --backend apple --release
water package --platform android --backend android --arch arm64 --release
water package --platform linux --backend gtk4 --release
```

### Clean Builds

When you need a fresh start (rarely necessary):

```bash
water clean
```

This removes WaterUI-specific build artifacts. Avoid `cargo clean` as it removes
the entire Cargo target directory, which wastes significant rebuild time.

## Environment Variables

The `water` CLI and WaterUI runtime respect these environment variables:

| Variable                  | Purpose                                    |
|---------------------------|--------------------------------------------|
| `RUST_LOG`                | Controls tracing log level (e.g., `debug`) |
| `WATERUI_DISPATCH_DEBUG`  | Enables view dispatch tracing (any Rust-side backend that uses `ViewDispatcher`, including Hydrolysis) |
| `CARGO_TARGET_DIR`        | Cargo build directory (do not customize when working inside the WaterUI monorepo) |

## Debugging in CI

When CI builds fail, use debug logging to get more information:

```bash
# Run with verbose output
water run --platform ios --logs debug

# Or set the environment variable
RUST_LOG=debug cargo test
```

The `--logs debug` flag enables tracing output from the WaterUI runtime, showing
view dispatch, signal updates, and FFI calls. On Apple platforms, this uses
`os_log`; on Android, `logcat`; on other platforms, `stderr`.

## What's Next

If your CI pipeline is passing but something still is not working, head to the [Troubleshooting](troubleshooting.md) appendix for solutions to common development issues.

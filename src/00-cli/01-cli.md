# WaterUI CLI Workflow

WaterUI ships a first-party CLI named `water`. Install it from the workspace checkout so that every example in this book can be scaffolded, run, and packaged without leaving your terminal.

```bash
cargo install --path cli --locked
```

## Quick Start with Playground

For quick experimentation and prototyping, use **Playground Mode**. This creates a minimal project where platform backends are automatically managed and hidden in `.water/`.

```bash
water create "My Experiment" --playground
cd my-experiment
water run --platform ios
```

This is the fastest way to get started. You focus on the Rust code, and the CLI handles the native integration details automatically.

## Scaffold a Full Project

For production applications where you need full control over the native projects (e.g. to add entitlements, modify `Info.plist`, or add custom native code), create a standard project:

```bash
water create "Water Demo" \
  --bundle-id com.example.waterdemo \
  --platform ios,android \
  --dev
```

- `--dev` keeps the generated project pinned to the local WaterUI sources while new releases are cooking.
- `--json` disables interactive prompts so commands can run inside scripts.
- Supply `--platform` to specify target platforms (`ios`, `android`, `macos`).

The command produces a Rust crate, `Water.toml`, and visible backend-specific folders under `apple/` and `android/` that you can open in Xcode or Android Studio.

## Run with Hot Reload

`water run` detects connected simulators and devices, builds your crate, and launches the app. **Hot Reload is enabled by default**, allowing you to see code changes instantly without restarting the app.

```bash
water run --platform ios --path water-demo
```

The CLI watches your source files, recompiles changes into a dynamic library, and injects it into the running application via a WebSocket connection.

- Use `--device <name>` to target a specific simulator/emulator from `water devices`.
- Add `--release` (to `water build` or `water package`, though `run` defaults to debug) if needed.
- Disable hot reload with `--no-hot-reload` if you want a standard build cycle.

## Package Native Artifacts

Produce distributable builds when you are ready to ship:

```bash
water package --platform android --path water-demo --release --arch arm64
water package --platform ios --path water-demo
```

Android packaging requires specifying the target architecture with `--arch` (e.g., `arm64`, `x86_64`). Apple builds honour the standard Xcode environment variables when invoked from scheme actions.

## Inspect and Repair Toolchains

Run doctor and devices early in each chapter to ensure your environment is healthy:

```bash
water doctor --fix
water devices
```

- `doctor` validates Rust, Xcode, Android SDK/NDK prerequisites. With `--fix` it attempts to repair missing components, otherwise it prints actionable instructions.
- `devices` lists available simulators and connected devices.

## Automation Tips

All commands accept `--json`. JSON output disables interactive prompts. Supply required arguments (like `--platform` and `--device`) up front to avoid stalling non-interactive shells.

Because every walkthrough in this book starts from a real CLI project, keep this reference handy: it is the quickest path to recreating any example locally.

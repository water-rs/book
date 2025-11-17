# WaterUI CLI Workflow

WaterUI ships a first-party CLI named `water`. Install it from the workspace checkout so that every example in this book can be scaffolded, run, and packaged without leaving your terminal.

```bash
cargo install --path cli --locked
```

## Scaffold a Project

Create a new playground app with the runtime backends you need:

```bash
water create --name "Water Demo" \
  --bundle-identifier com.example.waterdemo \
  --backend swiftui --backend android --backend web \
  --yes --dev
```

- `--dev` keeps the generated project pinned to the local WaterUI sources while new releases are cooking.
- `--yes` skips prompts so commands can run inside scripts.
- Repeat `--backend` for each platform you plan to target. You can always run `water add-backend <name>` later.

The command produces a Rust crate, `Water.toml`, and backend-specific folders under `apple/`, `android/`, and `web/`.

## Run with Hot Reload

`water run` detects connected simulators and browsers, recompiles your crate, and streams code changes to the selected backend:

```bash
water run --platform web --project water-demo
```

- Use `--device <name>` to target a specific simulator/emulator from `water devices`.
- Add `--release` once you need optimized builds for profiling.
- Disable the file watcher with `--no-watch` if your CI only needs a single build.

## Package Native Artifacts

Produce distributable builds when you are ready to ship:

```bash
water package --platform android --project water-demo --release
water package --platform ios --project water-demo
```

Android packaging accepts `--skip-native` when custom Rust artifacts are supplied. Apple builds honour the standard Xcode environment variables (`CONFIGURATION`, `BUILT_PRODUCTS_DIR`, etc.) when invoked from scheme actions.

## Inspect and Repair Toolchains

Run doctor and devices early in each chapter to ensure your environment is healthy:

```bash
water doctor --fix
water devices --json
```

- `doctor` validates Rust, Swift, Android, and web prerequisites. With `--fix` it attempts to repair missing components, otherwise it prints actionable instructions.
- `devices` emits a machine-readable list when `--json` is present, which is perfect for CI pipelines or automation scripts.

## Automation Tips

All commands accept `--format json` (or `--json`). JSON output disables interactive prompts. Supply `--yes`, `--platform`, and `--device` up front to avoid stalling non-interactive shells.

Because every walkthrough in this book starts from a real CLI project, keep this reference handy: it is the quickest path to recreating any example locally.

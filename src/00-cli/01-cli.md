# WaterUI CLI Reference

The `water` CLI is your primary tool for managing WaterUI projects. It handles scaffolding, running, and packaging your applications across different platforms.

## Installation

Ensure you have the CLI installed (see [Setup](../01-getting-start/01-setup.md)):

```bash
cargo install --path cli --locked
```

## Project Modes

WaterUI supports two project modes:

### 1. Playground Mode (Recommended for Learning)

```bash
water create "My Experiment" --playground
```

- **Best for**: Prototyping, learning, and simple apps.
- **Structure**: Hides native projects in `.water/`.
- **Experience**: Zero-config. Just run `water run` and go.

### 2. Full Project

```bash
water create "My App" --bundle-id com.example.app --platform ios,android
```

- **Best for**: Production apps needing custom native code (Info.plist, AndroidManifest.xml).
- **Structure**: Generates explicit `apple/` and `android/` folders.
- **Experience**: Full control over the native build process.

## Commands

### `create`

Scaffolds a new project.

```bash
# Playground
water create "Demo" --playground

# Production
water create "Production App" --bundle-id com.org.app --platform ios
```

### `run`

Builds and runs the application on a connected device or simulator.

```bash
# Auto-detect device
water run

# Target specific platform
water run --platform ios
water run --platform android

# Target specific device
water run --device "iPhone 15"
```

**Hot Reload** is enabled by default. Save your `.rs` files to see changes instantly.

### `package`

Produces distributable artifacts (IPA, APK/AAB).

```bash
water package --platform ios --release
water package --platform android --release --arch arm64
```

### `doctor`

Checks your development environment for missing dependencies.

```bash
water doctor
# Attempt to fix issues automatically
water doctor --fix
```

### `devices`

Lists all detected simulators and physical devices.

```bash
water devices
```
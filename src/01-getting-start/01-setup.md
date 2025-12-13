# Installation and Setup

Before we dive into building applications with WaterUI, let's set up a proper development environment. This chapter will guide you through installing Rust, setting up your editor, and creating your first WaterUI project.

## Installing Rust

WaterUI requires Rust 1.87 or later with the 2024 edition. The easiest way to install Rust is through rustup.

### On macOS, Linux, or WSL

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### On Windows

1. Download the installer from [rustup.rs](https://rustup.rs/)
2. Run the downloaded `.exe` file
3. Follow the installation prompts
4. Restart your command prompt or PowerShell

### Verify Installation

After installation, verify that everything works:

```bash
rustc --version
cargo --version
```

You should see output like:
```text
rustc 1.87.0 (a28077b28 2024-02-28)
cargo 1.87.0 (1e91b550c 2024-02-27)
```

> **Note**: WaterUI requires Rust 1.87 or later. If you have an older version, update with `rustup update`.

## Editor Setup

While you can use any text editor, we recommend VS Code for the best WaterUI development experience.

### Visual Studio Code (Recommended)

1. **Install VS Code**: Download from [code.visualstudio.com](https://code.visualstudio.com/)

2. **Install Essential Extensions**:
   - **rust-analyzer**: Provides IntelliSense, error checking, and code completion
   - **CodeLLDB**: Debugger for Rust applications
   - **Better TOML**: Syntax highlighting for Cargo.toml files

3. **Optional Extensions**:
   - **Error Lens**: Inline error messages
   - **Bracket Pair Colorizer**: Colorizes matching brackets
   - **GitLens**: Enhanced Git integration

### Other Popular Editors

**IntelliJ IDEA / CLion**:
- Install the "Rust" plugin
- Excellent for complex projects and debugging

**Vim / Neovim**:
- Use `rust.vim` for syntax highlighting
- Use `coc-rust-analyzer` for LSP support

**Emacs**:
- Use `rust-mode` for syntax highlighting
- Use `lsp-mode` with rust-analyzer

## Installing the WaterUI CLI

All examples in this book assume you have the `water` CLI available. From the repository root run:

```bash
cargo install --path cli --locked
water --version
```

The first command installs the current checkout; the second verifies that the binary is on your `PATH`.

### Verify Your Toolchain

Run the built-in doctor before continuing:

```bash
water doctor --fix
```

This checks your Rust toolchain plus any configured Apple and Android dependencies. Repeat it whenever you change machines or SDK versions. To discover connected simulators/emulators (useful for later chapters), run:

```bash
water devices
```

## Creating Your First Project

We will let the CLI scaffold a runnable playground that already references the in-repo workspace:

```bash
water create "Hello WaterUI" \
  --bundle-id com.example.hellowaterui \
  --platform ios,android \
  --dev
cd hello-waterui
```

Flags explained:

- `"Hello WaterUI"` is the project name. The folder name will be derived from it (e.g., `hello-waterui`).
- `--platform` specifies the target platforms (iOS, Android, macOS).
- `--dev` points dependencies at the checked-out workspace so each chapter’s code compiles against your local sources.

The generated project includes:

```text
hello-waterui/
├── Cargo.toml          # crate manifest
├── Water.toml          # WaterUI-specific metadata + enabled backends
├── src/lib.rs          # starting point for your app views
├── apple/, android/    # platform-specific backends (depending on --platform)
└── .water/             # CLI metadata and cached assets
```

## Hello, World!

Open `src/lib.rs` inside the newly created project and replace the body with a tiny view:

```rust
use waterui::prelude::*;

pub fn home() -> impl View {
    text("Hello, WaterUI! 🌊")
}
```

### Building and Running

Instead of calling `cargo run` directly, use the CLI so it can manage backends for you:

```bash
water run --platform ios
```

The same command auto-detects desktop/mobile simulators when you provide the platform. Once the dev server starts, every change you save in `src/lib.rs` hot-reloads into the selected target.

If you prefer to run the Rust crate alone (useful for unit tests or CLI tools), you can still execute `cargo test` or `cargo run` in parallel with the `water` commands; both workflows share the same sources.

## Troubleshooting Common Issues

### Rust Version Too Old

**Error**: `error: package requires Rust version 1.87`

**Solution**: Update Rust:
```bash
rustup update
```

### Windows Build Issues

**Error**: Various Windows compilation errors

**Solutions**:
1. Ensure you have the Microsoft C++ Build Tools installed
2. Use the `x86_64-pc-windows-msvc` toolchain
3. Consider using WSL2 for a Linux-like environment

```
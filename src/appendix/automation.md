# Automation and Troubleshooting

Use this appendix whenever you need to integrate WaterUI workflows into CI or recover from a broken toolchain.

## Deterministic CLI Runs

Pass `--format json` (or `--json`) to every `water` command to disable prompts. Provide `--yes`, `--platform`, and `--device` flags explicitly so scripts never block. Capture the resulting JSON to feed dashboards or orchestrators.

```bash
water devices --json | jq '.[0].name'
water run --platform web --project water-demo --json --yes
```

## Keeping Examples Green

All chapters rely on `mdbook test` and `cargo test` inside the tutorial workspace. Run both locally before opening a pull request:

```bash
mdbook test
cargo test -p waterui-tutorial-book
```

`mdbook test` compiles each code fence, so avoid `ignore` blocks—use `no_run` only when the example cannot execute inside a test harness.

## Diagnosing Platforms

- `water doctor --fix` performs end-to-end checks across Rust, Swift, Android, and the web toolchain. Keep its output in CI logs for future reference.
- `water clean --yes` removes Cargo, Gradle, and DerivedData caches when subtle build issues appear.
- `water build android --release --no-sccache` is helpful when debugging native toolchains separately from packaging.

Use these recipes whenever an exercise references platform-specific behaviour or when you need to automate deployments explained in earlier chapters.

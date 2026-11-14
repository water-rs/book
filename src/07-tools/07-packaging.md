# Packaging and distribution

> **In this chapter, you will:**
>
> - Produce a release package for each platform with `water package`
> - Choose a backend, an Android painter and architectures, and an iOS device
> - Build store-distribution packages and unsigned artifacts for CI
> - Declare signing, permissions, deployment targets, the launch screen and the app icon in `Water.toml`
> - Understand what a release build leaves out compared with `water run`

`water run` and `water preview` build for fast iteration. When you want
something you can hand to a user, a tester or a store, you use `water package`.
It builds the production profile, assembles the platform's native bundle with
the generated host project, signs it, and moves the result into your project.
Like every application workflow, it goes through the CLI: you never open the
generated Xcode or Gradle project yourself (see
[Project structure](../01-getting-started/04-project-structure.md)).

## The smallest package

```bash
water package --platform macos
```

`--platform` is the flag every package needs (Android also needs `--arch`, see
below). It takes `ios`, `ios-simulator`, `android`, `macos`, `linux`, `windows`
or `web`. Packaging builds the `release` profile unless you ask otherwise, and on
success the artifact lands in `target/package/` inside your project, replacing
the previous one; the command prints the final path. On Apple platforms the
artifact is the `.app` bundle; on Android it is an APK, or an AAB for store
distribution.

`--path <dir>` points the command at another project directory. Embedded
projects (see [Embedding](../06-advanced/13-embedding.md)) are rejected: for
those, `water build` already produces the artifact the host application
consumes.

Platform support follows the framework's tiers: macOS, iOS and Android are
tested on real devices; Windows and Linux are built and tested in CI. The
GTK, WinUI and TUI backends are experimental. See
[Platforms](../01-getting-started/05-platforms.md).

## Choosing a backend

Each platform has a default backend: `apple` for macOS and iOS, `hydrolysis` for
Android, Windows and the web, and `gtk4` for Linux. A `[platforms.<name>]`
declaration in `Water.toml` replaces the default, and an explicit `--backend`
must agree with it.

```toml
[platforms.linux]
backend = "hydrolysis"
```

```bash
water package --platform linux --backend hydrolysis
```

The backend names are `apple`, `gtk4`, `hydrolysis` and `winui`. A backend that
does not run on the chosen platform is rejected before anything builds, and so
is a host that cannot build it: desktop packages build only on their own
operating system, the Apple backend needs a macOS host, and Android and the web
cross-compile from any host. The experimental backends (GTK, WinUI) ask for
confirmation first; pass `-y` in a non-interactive environment such as CI.

## Android

Android packages are drawn by Hydrolysis, the platform's only backend, so
`--backend hydrolysis` only states the default. You must say which CPU
architectures to include with `--arch`, a comma-separated list of `arm64`,
`x86-64`, `armv7` and `x86`:

```bash
water package --platform android --backend hydrolysis --arch arm64
water package --platform android --backend hydrolysis --arch arm64,x86-64
```

`--painter` selects how the Hydrolysis host draws on Android: `gpu` (the
default, a `SurfaceView`-backed GPU band, which runs down to the framework's
Android floor, API 33) or `hwui` (the `RenderNode` painter, which draws through
the host view hierarchy and raises the app's minimum to API 34).
It applies only to `--platform android --backend hydrolysis`; when omitted, the
`[hydrolysis] painter` setting in `Water.toml` is the project default.

## iOS and device provisioning

A device build is provisioned for specific hardware. Pass the device's hardware
UDID with `--device` and the CLI selects, or asks Xcode to create, a
development profile covering it:

```bash
water package --platform ios --device 00008110-001234567890401E
```

`--device` is valid only with `--platform ios`, and it cannot be combined with
`--unsigned`, because an unsigned package embeds no provisioning profile. For
the simulator, use `--platform ios-simulator`; simulator packages are not signed
per device.

## Store distribution

`--distribution` packages for people other than you, on machines that never saw
your keychain. It changes the package on two platforms:

- **Android** builds an Android App Bundle (AAB) for upload to the Play Store
  instead of an APK.
- **macOS** signs with your team's Developer ID Application certificate, the
  hardened runtime and a secure timestamp, then notarizes and staples the app.
  The `[signing.macos]` table described below drives it.

```bash
water package --platform android --backend hydrolysis --arch arm64,x86-64 --distribution
water package --platform macos --distribution
```

`--distribution` cannot be combined with `--debug`. iOS packages are signed with
development provisioning profiles only, so `water package` does not produce an
App Store build, and `--distribution` does not change an iOS package.

## Unsigned builds

A CI host or build VM often has no signing identity. `--unsigned` leaves the
artifact unsigned, with the same linkage and profile as the signed build, so
you can sign it afterwards with `codesign` or `apksigner`:

```bash
water package --platform ios --unsigned
water package --platform android --backend hydrolysis --arch arm64 --unsigned
```

It applies to iOS device builds with the Apple backend and to Android builds
with Hydrolysis. Two combinations fail by design:

- `--unsigned --distribution` on iOS, because the App Store rejects an unsigned
  bundle. On Android the pair produces an unsigned AAB for you to sign yourself.
- `--unsigned --debug` on Android, which always signs with the debug keystore.

## Profiles and the network

```bash
water package --platform macos --release   # the default, stated explicitly
water package --platform macos --debug     # unoptimized `dev` profile
water package --platform macos --offline
```

`--release` and `--debug` are mutually exclusive. A debug package is useful for
reproducing a problem in an installed bundle, but it is not what you ship.

`--offline` behaves like `cargo build --offline` (and honors
`CARGO_NET_OFFLINE`). A font declared by your project that is not already in
the font cache then fails packaging instead of being downloaded. See
[Assets](../04-rich/05-assets.md).

## What a release build leaves out

Debug builds carry tooling hooks: the exported entry points that
[`water preview`](01-preview.md) loads, and debug-only metadata records the
framework's macros write to the `.wmeta` linker section. Both are compiled only
under `debug_assertions`, so a `release` package contains neither.

Records that packaging itself needs, such as the asset bundles your code mounts
and the widget extension sources, are emitted in every profile. The CLI reads
them from the compiled library before the final link, and the final link leaves
them out of the binary you ship.

## Signing

Signing configuration lives under `[signing]` in `Water.toml` and holds no
secrets. `[signing.android]` signs every release Android package, while a debug
Android package signs with the debug keystore. `[signing.macos]` is read only by
a macOS `--distribution` package; any other macOS package gets an ad hoc
signature, or your installed development identity when the app declares a
permission macOS prompts for, so the grant survives a rebuild.

```toml
[signing.macos]
team_id = "ABCDE12345"
notary_profile = "my-notary-profile"

[signing.android]
keystore = "release.keystore"      # relative to the project root
key_alias = "upload"
```

For macOS, `team_id` is matched against the Developer ID Application
certificate, and `notary_profile` names a keychain profile created once with
`xcrun notarytool store-credentials`. A distribution package never falls back to
an ad hoc signature.

For Android, the keystore passwords are read from the environment and never
written to the project:

```bash
export WATERUI_ANDROID_STORE_PASSWORD=<keystore password>
export WATERUI_ANDROID_KEY_PASSWORD=<key password>
```

Unknown keys under `[signing.android]` are an error, so a committed
`store_password` fails parsing instead of leaking. A release Android package with
no `[signing.android]`, a missing keystore file or an unset password variable
fails before the Rust build starts, unless you pass `--unsigned`.

## Permissions

Declare what the app needs in `Water.toml`. The CLI turns each entry into the
platform declaration: `AndroidManifest` entries and Info.plist usage strings.

```toml
[permissions.internet]
enable = true
description = "Required to download map tiles"

[permissions.camera]
enable = true
description = "Scan a code to sign in"
```

The keys are `internet`, `camera`, `microphone`, `location`, `coarse_location`,
`storage`, `write_storage`, `photo_library`, `contacts`, `calendars`, `bluetooth`,
`bluetooth_admin`, `vibrate`, `wake_lock`, `activity_recognition` and
`nearby_wifi_devices`. The `description` is the text people read in the system
prompt, so write it for them. Both `enable` and `description` are required.

A missing `internet` entry makes every request on Android fail with a resolution
error rather than a permission error. It shows up first in a release package:
debug Android builds always declare `INTERNET` for the inspector connection,
while a release manifest carries only what `Water.toml` declares. An Android
build warns when a dependency declares that it needs a permission you have not
enabled, or when the dependency graph contains a known HTTP client and
`internet` is missing.

Declaring a permission is only half the work: camera, microphone and location
must also be requested at runtime through the `waterkit-permission` crate, which
your app adds as a direct dependency. Do it from an async action, never from a
view body:

```rust,ignore
use waterkit_permission::{Permission, PermissionStatus, check, request};
use waterui::prelude::*;

button("Enable microphone")
    .action_async(|State(granted): State<Binding<bool>>| async move {
        let status = check(Permission::Microphone).await;
        let status = if matches!(status, PermissionStatus::Granted) {
            status
        } else {
            match request(Permission::Microphone).await {
                Ok(status) => status,
                Err(_) => return,
            }
        };
        granted.set(matches!(status, PermissionStatus::Granted));
    })
    .state(&granted)
```

Match `PermissionStatus` with a wildcard arm, and on denial leave the feature
closed and explain how to retry. For the state handling in the snippet, see
[Reactive state](../02-core/02-reactive.md).

## Apple deployment targets

Apple builds target the framework's deployment floor, currently 26.0 for macOS
and iOS. If an API you use needs a newer OS, it fails to compile below it, and
the error names this table:

```toml
[apple.deployment-targets]
ios = "27.0"
macos = "27.0"
```

The keys are `macos`, `ios`, `tvos`, `watchos` and `visionos`. A value below the
framework floor is an error. `water create` writes `27.0` for a project that
bundles a web frontend, because its navigation policy needs OS 27. Apple builds
are ARM64 only: `aarch64-apple-darwin`, `aarch64-apple-ios` and
`aarch64-apple-ios-sim`.

## The app icon

Put one square `Icon.svg` (or `Icon.png`) at the root of `assets/`. The CLI
generates every platform format from it: full-bleed on iOS, a rounded rectangle on
macOS, adaptive layers on Android, `.ico` on Windows and a `.desktop` entry on
Linux. Exactly one root-level `Icon.*` is allowed, and a new project starts with
the WaterUI logo, so replacing that file rebrands the app everywhere. The assets
directory is `assets` unless `assets_path` in `[package]` says otherwise.

## The launch screen

The launch screen is shown from the tap on the icon until the first frame. No Rust
runs during that time, so it is declared in `Water.toml` rather than built as a view:

```toml
[launch]
background = "#0B0B0F"
background_dark = "#000000"
```

Both colors are optional and written as `#RRGGBB`; `background` defaults to
`background` in the `[theme]` table, and `background_dark` to the resolved
`background`. A malformed color fails when the manifest is read. One optional
square `Launch.svg` or `Launch.png` at the root of `assets/` is the artwork. iOS, Android and the web take it; desktop
platforms have no launch screen and ignore the table.

## App values

Some capability crates need configuration only your app knows, such as a Firebase
file or a payments merchant ID. The crate asks for it, and packaging fails naming
the missing key and the crate that requested it until you supply it under
`[app_values]`:

```toml
[app_values]
firebase_config = "google-services.json"
apple_pay_merchant_ids = ["merchant.com.example"]
cast_receiver_app_id = "CC1AD845"
```

These three keys are the whole table, and any other key is an error. The CLI
places each value where the platform reads it:

- `firebase_config` is a path relative to the project root, copied into the
  Android module as its `google-services.json`.
- `apple_pay_merchant_ids` becomes the `com.apple.developer.in-app-payments`
  entitlement. Each ID is `merchant.` followed by a reverse-DNS name.
- `cast_receiver_app_id` becomes the Android string resource
  `@string/waterui_cast_receiver_app_id`.

## Next steps

Verify a package on a real device before you submit it, and use the
[preview](01-preview.md) and [inspector](05-inspector.md) tools on the debug
profile while you iterate. If packaging fails on a toolchain problem, see
[Troubleshooting](../appendix/troubleshooting.md).

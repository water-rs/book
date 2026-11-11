# Embedding WaterUI in an existing app

> **In this chapter, you will:**
> - Declare a crate as embedded with `embedded = true` and build it with `water build`
> - Mount the result in an existing SwiftUI, UIKit or AppKit app through `WaterUIHost` and `WaterUIHostController`
> - Mount it in an existing Android activity or fragment through the generated `WaterUi` object
> - Know who owns the resource directories, and how Rust code reaches them without global state

Most of this book assumes WaterUI owns the program: `water run` starts your
app and the CLI generates the native project around it. Sometimes the program
already exists. You have a shipping iOS, macOS or Android app and want one
screen, or one feature, written in WaterUI. Embedding inverts the ownership. Your host app
keeps its entry point, its build system and its navigation; the WaterUI crate
becomes a library the host links and mounts as an ordinary native view.

## Declare the crate embedded

The crate is the same library crate you wrote in [Your first
app](../01-getting-started/03-first-app.md): one `pub fn app(env: Environment)
-> App`. The only change is one line in the `[package]` section of
`Water.toml` (see [Project structure](../01-getting-started/04-project-structure.md)):

```toml
[package]
name = "Settings Panel"
bundle_identifier = "com.example.settingspanel"
embedded = true
```

The flag defaults to `false` and is written back to `Water.toml` only when it
is set. Your Rust code does not change:

```rust
use waterui::app::App;
use waterui::prelude::*;

fn root() -> impl View {
    vstack((
        text("Embedded WaterUI").title().bold(),
        text("你好 / こんにちは / 안녕하세요"),
    ))
}

pub fn app(env: Environment) -> App {
    App::new(root, env)
}
```

An embedded app must declare a root window, which `App::new` does for you. On
Apple, the backend mounts the first window declaration into the host's view and
never starts the application's termination machinery: the host decides when
the process lives or dies. The CLI generates the native entry points around
your `app` function, so the crate needs no embedding-specific code.

### What `water build` produces

An embedded crate has no runnable app, so the commands that need one refuse:

- `water run` fails and tells you to build the artifact and run the host app.
- `water package` fails: `water build` already produces the host-consumable
  artifact.
- `water build --output-dir` is rejected. The artifact lands under
  `target/package/` in your project.

`water build` instead produces a different artifact per platform:

| Platform | Command | Artifact |
| --- | --- | --- |
| Apple | `water build --platform macos` | a local Swift package at `target/package/<crate>-apple` |
| Android | `water build --platform android --backend hydrolysis` | an AAR at `target/package/<crate>-<version>-release.aar`, also published to `mavenLocal()` |

`<crate>` is the package name in your `Cargo.toml`. Add `--release` for an
optimized Rust build. Apple support follows the framework's ARM64-only rule:
whichever Apple platform you pass, the package carries slices for ARM64 macOS,
iOS device and iOS simulator, and Intel Apple targets are unsupported. See
[Platforms](../01-getting-started/05-platforms.md) for the support tiers.
Android embedding is built by the Hydrolysis backend only;
passing another backend is a build error.

## Embed on Apple platforms

`water build` writes a Swift package with one library product, `WaterUI`. It
contains a header-free binary framework, `WaterUINative.xcframework`, holding
your Rust code together with the Apple backend's Swift host types
(`WaterUIRuntime`, `WaterUIHost` and the rest of this section), and your staged
`waterui_assets` and `fonts` as package resources. In Xcode, add
`target/package/<crate>-apple` as a local package dependency of the host app
and `import WaterUI`. The manifest declares `swift-tools-version: 6.3` and
passes linker flags SwiftPM accepts only from a local package, so reference it
by path.

Rebuild with `water build` after each Rust change. Cargo caches live outside the
package directory, and each build replaces the previous successful package
whole.

### One runtime per process

`WaterUIRuntime` holds the services every embedded instance shares. Create it
once, on the main actor, and pass the same value to every host you mount:

```swift
let runtime = await WaterUIRuntime.create()
```

`create()` is `async` because the runtime prepares its services, the GPU
among them, before it returns. Every host retains the runtime it was given, so
the runtime lives as long as any mount does.

### Mount in SwiftUI

`WaterUIHost` is a SwiftUI representable. It takes the shared runtime and a
`WaterUIResourceContext`:

```swift
import SwiftUI
import WaterUI

@main
struct HostApp: App {
    @State private var runtime: WaterUIRuntime?

    var body: some Scene {
        WindowGroup {
            if let runtime {
                WaterUIHost(runtime: runtime, resources: .module)
            } else {
                ProgressView()
                    .task { runtime = await WaterUIRuntime.create() }
            }
        }
    }
}
```

`WaterUIHost` sizes itself through SwiftUI's `sizeThatFits`: it measures the
mounted content under the proposal SwiftUI makes, so you can put it in a stack
or size it with SwiftUI's `frame` modifier like any other view. It also forwards the host's declared
environment into the mount: color scheme, layout direction, locale, and the
metrics SwiftUI resolves for the body, title, headline, subheadline, caption
and footnote text styles, so Dynamic Type follows the host. When the SwiftUI
environment changes, the WaterUI content updates in place without being
rebuilt. Use SwiftUI modifiers such as
`.environment(\.colorScheme, .dark)` or `.environment(\.locale, ...)` on the
host to steer the embedded content; see
[Environment](../02-core/03-environment.md) for how WaterUI reads the same
values.

### Mount in UIKit or AppKit

`WaterUIHostController` is a `UIViewController` on iOS and an `NSViewController`
on macOS, with the same initializer on both:

```swift
let controller = WaterUIHostController(runtime: runtime, resources: .module)
```

Add it as a child controller and its `view` as a subview, the way you would
for any other child. The content mounts when the controller loads its view,
fills that view and resizes with it. A controller you mount directly has no
SwiftUI environment to forward, so the content follows the platform instead:
the appearance its view inherits and the system locale.

If the host changes an ancestor of the controller's view in a way the content
cannot observe (toggling `isHidden` or `alpha`, clipping, applying a
transform, or reparenting inside the same window), call `updateVisibility()`
on the controller. Surfaces parked because they were not visible, such as a
[GPU surface](../05-graphics/02-gpu-surface.md), re-check and resume.

### Scenes on iOS

A WaterUI app can declare more than one window. The mount shows the first one
in the host's view; on iOS every further window is a UIKit scene, and in an
embedded app the host owns the scenes. `WaterUIHostController.sceneOwner`
returns a `WaterUISceneOwner`, a route to the windows of that mounted app. Keep
it with the host's scene session and forward the scene lifecycle to it:

```swift
// Held alongside the host's scene session.
let owner = controller.sceneOwner

// In the scene delegate:
let window: UIWindow? = owner.connect(windowScene)  // scene connected
owner.disconnect(windowScene)                        // scene disconnected
```

`connect(_:)` creates a window in the scene, fills it with the app's next
declared window (or a fresh instance of the first one once the declarations
run out), makes it key and visible, and hands it to you to retain. It returns
`nil` once the controller that owns the mount is gone, so a late connection
never resurrects a destroyed app. The scene owner retains the runtime, not the
mount. `WaterUISceneOwner` exists on iOS only: on macOS the backend opens the
further windows itself. Windows are covered in
[Windows](../03-ui/08-windows.md).

### Resources

Every host initializer takes a `WaterUIResourceContext`, a pair of file URLs: the
`assets` directory and the `fonts` directory. For the generated package, pass
`.module`: it points at the `waterui_assets` and `fonts` resources bundled in
the package itself. If you leave the argument out, the default is
`.application`, which reads `waterui_assets` and `fonts` from the host app's
main bundle. Construct one by hand with
`WaterUIResourceContext(assets:fonts:)` when the host keeps those directories
elsewhere; both URLs must be file URLs.

## Embed on Android

`water build --platform android --backend hydrolysis` assembles a
`com.android.library` module and publishes it to the local Maven repository. The
AAR carries the app's native library for every ABI (narrow the set to one with
`--arch`), the staged `waterui_assets` with your fonts inside, and a generated
`WaterUi` object. The build prints the exact coordinates and what to add to the
host. In a Kotlin-script Gradle build that looks like this:

```kotlin
// settings.gradle.kts of the host: let the host find the published AAR.
dependencyResolutionManagement {
    repositories {
        google()
        mavenCentral()
        mavenLocal()
    }
}

// build.gradle.kts of the host's app module.
dependencies {
    implementation("com.example.settingspanel:settings_panel:0.1.0")
}
```

The group is `bundle_identifier`, which must therefore be a valid Android
package name; the artifact id is the package name in your `Cargo.toml`
(`settings_panel` here) and the version is its `version`. The AAR's POM exports the
Hydrolysis host modules as `api` dependencies, and those modules publish to
the same local repository under `dev.waterui.hydrolysis`. They exist only in the
machine's `mavenLocal()`, so run `water build` on every machine that builds the
host app instead of copying the AAR between machines. The host needs the
`compileSdk` and `minSdk` the build prints, or higher.

### Mount in an activity

The generated entry point lives in the `.waterui` subpackage of the library's
group. For a `ComponentActivity`, mounting is one call:

```kotlin
import androidx.activity.ComponentActivity
import android.os.Bundle
import com.example.settingspanel.waterui.WaterUi

class SettingsActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(WaterUi.createView(this) { finish() })
    }
}
```

`createView` returns an ordinary `View`. It is an empty container at first:
the first mount in a process copies the AAR's assets into app storage off the
main thread, and the content joins the container once that finishes. System
back gestures reach the app while it has somewhere to go back to. The trailing
lambda receives the app's close requests while the view is attached. A close
request that arrives between mounts, for example during a configuration
change, waits and is delivered to the next mount. The session is retained in a `ViewModel`, so rotating the device
does not restart your app.

### Mount in a fragment or another owner

For anything that is not a `ComponentActivity`, use the overload that takes the
owners explicitly:

```kotlin
import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import androidx.fragment.app.Fragment
import com.example.settingspanel.waterui.WaterUi

class SettingsFragment : Fragment() {
    override fun onCreateView(
        inflater: LayoutInflater, container: ViewGroup?, savedInstanceState: Bundle?,
    ): View = WaterUi.createView(
        context = requireContext(),
        lifecycleOwner = viewLifecycleOwner,
        viewModelStoreOwner = this,
        onBackPressedDispatcher = requireActivity().onBackPressedDispatcher,
        key = "settings",
        onCloseRequested = { parentFragmentManager.popBackStack() },
    )
}
```

Each `key` retains its own session in the owner's `ViewModel` store, and the
default key is the native library name. Two simultaneous mounts under the same
owner need distinct keys, because one key may be mounted only once at a time.

The AAR's manifest declares the Android permissions enabled in your
`Water.toml`, which the host's manifest merger folds into the app's own, and
its consumer ProGuard rules keep the classes the native library loads by name.
A Gradle plugin or a Firebase configuration file that a dependency needs
cannot travel in a library: the build warns and names what the host app module
must declare itself.

## Who owns the resources

An embedded app can share its process with other WaterUI content: a second
mount on another screen, or a test harness. That is why asset resolution
never reads a process-wide resource root. Each mounted instance carries its own
`ResourceContext`, the asset directory and the font directory, in its
`Environment`, installed before your `app` function runs:

- On Apple, the host supplies it. `WaterUIResourceContext` is the Swift side of
  the handshake: the backend copies its two URLs into a Rust `ResourceContext`,
  registers the fonts in its font directory, and installs the context in the
  mount's environment.
- On Android, the Hydrolysis host module copies the AAR's `waterui_assets` into
  app storage once per process, and the backend builds the context from that
  directory.
- A standalone app gets its context from the backend the same way, from its own
  package, so the same Rust code runs in both modes.

In Rust you rarely touch this: `ImageAsset` and `VideoAsset` resolve themselves
against the context in the environment they render in. Code that runs outside
view rendering reads the context from the `Environment` your `app` function
receives and passes it along explicitly:

```rust
use waterui::app::App;
use waterui::prelude::*;

pub fn app(env: Environment) -> App {
    let status = match ResourceContext::from_environment(&env) {
        Some(resources) => format!("Assets at {}", resources.assets().display()),
        None => {
            waterui::log::error!("the host installed no ResourceContext");
            String::from("No resource context installed")
        }
    };
    App::new(move || text(status.clone()), env)
}
```

`from_environment` returns `None` only when the host built the view tree
without installing a context, which is a host bug, so report it rather than
substitute a directory. `ResourceContext` needs the `assets` feature, which is
on by default. The fallible loaders that take a `&ResourceContext`
(`asset!("settings.json").load(resources)` and friends) are covered in
[Assets](../04-rich/05-assets.md#loading-outside-a-view-body).

## Where to go next

- [Platforms](../01-getting-started/05-platforms.md) lists what each backend supports.
- [Packaging](../07-tools/07-packaging.md) covers the shipping path for apps WaterUI owns, the counterpart to embedding.
- [Backends](../08-internals/04-backends.md) explains how Apple and Hydrolysis realize the content a host mounts.

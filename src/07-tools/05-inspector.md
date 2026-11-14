# The inspector

> **In this chapter, you will:**
>
> - Know when the `inspector` feature is on, what `inspector-signals` adds, and what a debug build publishes
> - Attach the inspector app to a running debug build with `water inspector`
> - Reach macOS, Linux, Windows, iOS and Android targets with `--platform`, `--target`, `--device` and `WATERUI_INSPECTOR_TOKEN`
> - Read the sections the inspector shows: frames, tree, tasks, logs and signals
> - Wire the recorders into a backend of your own

`water preview` answers "what does this view look like". It does not answer "why did that frame take 40 ms", "which task blocked the main thread" or "which `Binding` just woke this subtree". The inspector answers those. It is a separate WaterUI application, built and launched by the `water` CLI, that connects over TCP to an endpoint compiled into the app you are debugging and renders what the endpoint streams.

The design is subscription-driven. Every producer (the task probe, the rendering backend, the log layer, the reactive-graph bridge) asks whether anyone subscribed to its channel before it spends anything. With no inspector attached, the cost of an event that nobody wants is one relaxed atomic load.

## Enabling the feature

The endpoint lives in the `waterui::inspector` module, which exists only when the facade's `inspector` feature is on and the target is not `wasm32`. Neither feature is in the facade's `default` set; both are in `all`.

| Feature | What it adds |
|---------|--------------|
| `inspector` | The endpoint: frames, accessibility tree, main-thread task occupancy and `tracing` logs |
| `inspector-signals` | Implies `inspector`, and adds the reactive-graph channel by bridging `nami`'s graph observer |

You usually do not add `inspector` yourself. The bundled Apple and Hydrolysis backends depend on `waterui` with `inspector` enabled, so an app built on either one has the module. A crate that does not go through those backends opts in explicitly:

```toml
[dependencies]
waterui = { version = "0.5", features = ["inspector"] }
```

Having the module compiled in does not make an app inspectable. Whether anything is prepared or bound is decided at startup, as the next section describes.

`inspector-signals` is the one you opt into deliberately, because it reports every notification in the reactive graph. The inspector app does not subscribe to it by default either (see [Signals](#signals)).

## What a debug build publishes

A backend starts the endpoint through `waterui::inspector::maybe_init_from_env`. What it returns depends on the build and the environment:

| Situation | Result |
|-----------|--------|
| `WATERUI_INSPECTOR_TOKEN` is set | The endpoint binds and listens immediately, with that token, in debug and release builds alike. Use this when you launch the app yourself and want it reachable from the start: you already know the token to give `water inspector`. |
| Debug build, no token in the environment | A runtime that is prepared but silent. Nothing is bound and nothing is advertised until the app asks for the inspector (an "Inspect element" action, which calls `InspectorRuntime::inspect_node`, or a call to `InspectorRuntime::open`). |
| Release build, no token | `None`. Release builds inspect only when the environment says so. |
| A setting is present but unparseable, or the socket cannot be bound | A warning through `tracing`, and `None`: inspection stays off. |

An app nobody is inspecting does not hold a port open and does not advertise itself.

Once bound, the endpoint is protected by a token. Without `WATERUI_INSPECTOR_TOKEN`, a debug build generates one per process (128 bits from the operating system's random source, as 32 hex characters), so an open port is never usable by whoever happens to reach it. The token is a credential: `Debug` output for the endpoint and its configuration prints `[redacted]`.

Where a debug build's self-prepared endpoint listens depends on who has to reach it:

- A desktop app binds loopback (`127.0.0.1`), because it is inspected from the machine it runs on.
- An Android or iOS app binds all interfaces (`0.0.0.0`), because it is inspected from your computer.
- The port is `0`, so the operating system picks an ephemeral one.

An endpoint configured through the environment binds `WATERUI_INSPECTOR_HOST` and `WATERUI_INSPECTOR_PORT` instead, which default to `127.0.0.1` and `0` on every platform.

A listening app also writes an advertisement file, containing its name, pid, backend, address and token, to a per-user cache directory (`waterui-inspector` under the platform cache directory). `water inspector` reads those files to find a running app on the same machine. The file is removed when the runtime drops, and stale entries whose process is gone are ignored. Set `WATERUI_INSPECTOR_DISCOVERY_DIR` to move the directory, which a sandboxed app needs when it cannot share the machine cache.

### Environment variables

`InspectorServerConfig::from_env` reads these; only the first is required.

| Variable | Meaning | Default |
|----------|---------|---------|
| `WATERUI_INSPECTOR_TOKEN` | Enables inspection and sets the credential a client must present | none (inspection off) |
| `WATERUI_INSPECTOR_HOST` | Address to bind | `127.0.0.1` |
| `WATERUI_INSPECTOR_PORT` | Port to bind | `0` (ephemeral) |
| `WATERUI_INSPECTOR_TASK_WINDOW_MS` | Aggregation window for task occupancy | `100` |
| `WATERUI_INSPECTOR_STALL_RATIO` | Fraction of the frame budget above which a poll counts as a stall | `0.90` |

## Attaching with `water inspector`

Run your app in a debug session, then in another terminal:

```bash
water inspector
```

With no arguments the command reads the advertisement files on this machine and attaches to the one it finds. It then builds the inspector app against your project and opens it. The handshake checks the token and the protocol's build commit, and rejects a client built from a different commit with an `inspector protocol mismatch` error. The command stays attached until the inspector window closes; `Ctrl-C` stops it.

If no debug build is running it fails and says so. If several are running it lists them (name, pid, address) and asks you to choose one with `--target`.

### Options

| Option | Meaning |
|--------|---------|
| `-p`, `--platform` | `local` (default), `macos`, `linux`, `windows`, `ios` or `android`: where the inspected app runs |
| `--target ip:port` | The endpoint, as a socket address: an IP address and a port, not a hostname. Required for every platform whose app cannot run on this machine |
| `-d`, `--device` | Android device serial or AVD name. Valid only with `--platform android` |
| `--path` | Project directory, defaults to the current directory |
| `--offline` | Build the inspector app without network access |

The inspector client always runs on your machine. `--platform` only says how the target's endpoint is reached.

### Reaching an app that is not on this machine

An app on another machine or device cannot be discovered, so you give both halves of the connection yourself: the address with `--target` and the credential in `WATERUI_INSPECTOR_TOKEN`. The token is never a command-line flag, because argv is readable by every process on the machine; the command rejects `--token`.

```bash
# An iOS simulator shares this Mac's loopback.
WATERUI_INSPECTOR_TOKEN=<token> water inspector --platform ios --target 127.0.0.1:50123

# An iOS device needs its routable address.
WATERUI_INSPECTOR_TOKEN=<token> water inspector --platform ios --target 192.168.1.42:50123

# A Linux app on another machine.
WATERUI_INSPECTOR_TOKEN=<token> water inspector --platform linux --target 192.168.1.20:50123
```

For `macos`, `linux`, `windows` and `ios`, the app is dialed directly at `--target`. When the platform is `local`, or matches the OS of the machine running `water`, `--target` may be omitted for discovery, or given without a token: the command then takes the token from the local advertisement whose port matches. For every other combination, `--target` and `WATERUI_INSPECTOR_TOKEN` are both required, and the command says so.

A desktop debug build binds loopback, so a desktop app on another machine is reachable only when it was started with `WATERUI_INSPECTOR_TOKEN` and a routable `WATERUI_INSPECTOR_HOST` (and, to know the port in advance, `WATERUI_INSPECTOR_PORT`).

### Android

On Android the endpoint is bound on the device, so the CLI forwards a local port to it with `adb forward` and removes the forward when the session ends. Only the port of `--target` is used: pass the port the app bound on the device, and `--device` to pick the adb target:

```bash
WATERUI_INSPECTOR_TOKEN=<token> water inspector --platform android --target 127.0.0.1:50123 --device emulator-5554
```

Without `--device`, the CLI uses the device you used last, then a single unambiguous candidate, then a picker. An emulator that is not yet booted is launched first, because `adb` cannot forward to a device that is not up.

### Opening it from inside the app

A running desktop debug build can launch the inspector itself. The bundled backends offer the action for you: the Apple backend adds "Inspect Element" to a right-click menu on macOS and triggers the same action with a long press on iOS, and Hydrolysis appends "Inspect element" to a context menu that already has items of its own.

Both actions end in `InspectorRuntime::inspect_node`, and `InspectorRuntime::open` does the same without naming a node. They bind the endpoint if needed and spawn `water inspector --target <addr>` with the token in the child's environment (plus `--path` from `WATERUI_PROJECT_DIR`, which the CLI sets when it launches the app), so "inspect this element" works from an app you double-clicked. Setting `WATERUI_CLI` names the CLI binary explicitly; otherwise it is searched on `PATH`, then in the usual install directories. If an inspector is already attached, `inspect_node` just reveals the node in it and `open` does nothing; a node requested before the inspector connects is revealed when it does. A failed launch is reported through `tracing` with the CLI's output, token removed.

On iOS and Android there is nothing to launch on the device, so the endpoint's address is logged and you attach from your computer. The token is not logged; it is in the app's advertisement file in its private cache directory. On Android that is `waterui-inspector` under the app's cache directory, which `adb shell run-as <package>` can read on a debuggable build.

## What the inspector shows

The app has a sidebar of six sections. Each section reads one channel, and the handshake tells the inspector which channels the build can produce, so a channel the app was not built with is grayed out rather than awaited forever. That is a property of the build, not of the backend: frames, tree, tasks and logs are always advertised, and signals when `inspector-signals` is on, so a section the backend never feeds is enabled but stays empty.

| Section | Channel | Source |
|---------|---------|--------|
| Overview | none | Connection state and target summary |
| Frames | `frames` | Rendering backend, via `FrameRecorder` |
| Tree | `tree` | Rendering backend, via `TreeRecorder` |
| Tasks | `tasks` | Main-thread task probe |
| Logs | `logs` | `tracing` layer |
| Signals | `signals` | Reactive-graph bridge (`inspector-signals`) |

A default subscription covers frames, tree, tasks and logs. Signals is excluded and opened on purpose from Overview.

The bundled backends do not feed every channel. Hydrolysis reports frames, the tree and tasks, and signals on its desktop runner when its own `inspector-signals` feature is on; it does not install the log layer, so Logs stays empty. The Apple backend reports the tree, tasks and logs; it records no frames and does not observe the reactive graph.

### Frames

Each frame reports its kind (`idle` when nothing was done, `refresh` when the retained tree was patched, laid out and re-encoded), time spent in layout, encode and present, total wall time, the frame budget derived from the display refresh rate, the number of composited layers and the deepest clip nesting reached. A renderer that skips work still reports the frame, because "we did nothing" is the answer to most performance questions. Clip depth is a structural cost you can act on by flattening the view tree.

### Tree

The accessibility tree as the backend sees it. Each node has a role, label, value, bounds in window coordinates, interaction state (enabled, selected, checked, expanded, hidden) and children. Updates are deltas: the first update after the tree channel gains a subscriber is the full tree, then only changed and removed nodes, and nothing at all when the tree did not change. This is the same tree [`waterui-testing`](03-testing.md) and [the accessibility chapter](../06-advanced/05-accessibility.md) exercise, so a node that is missing or mislabeled here is missing or mislabeled for a screen reader too.

### Tasks

Main-thread occupancy, aggregated inside the target process over a window (100 ms by default). For each task type it shows the number of polls, how many completed the future, total and maximum wall time, total CPU time and how many polls exceeded the frame budget. A poll that consumes more than the stall ratio of the budget is also reported individually as a stall, with a backtrace. Backtrace capture costs milliseconds on the very thread being measured, so it is rate-limited to at most one per second; a stall can arrive with an empty stack, and that is deliberate.

### Logs

The `tracing` records the app emits, with level, target, message, source file and line, and structured fields. It shows the same records `water run --logs debug` prints, so you do not choose between the terminal and the inspector. Records appear only if the backend installed the layer returned by `InspectorRuntime::tracing_layer`.

### Signals

Requires `inspector-signals`, and a backend that installs the bridge with `observe_signals` (see [Wiring a backend](#wiring-a-backend)). This channel sees why the UI updated: when a state-owning node is created (with its type name and source location), when watchers subscribe and unsubscribe, when it notifies, and when it drops. Only creation carries strings; notifications reference the node by id, so a busy graph does not resend its source locations thousands of times a second. Node ids are pointer-derived and reused after a drop, so treat a creation event as the start of an identity's life and a drop as its end. See [reactive state](../02-core/02-reactive.md) for the graph being observed.

## Backpressure

Producers publish into one bounded channel of 8192 events, and a single dispatcher thread owns every connected client. When the channel is full, an event is dropped rather than blocking the app, and counted per channel. The dispatcher tells each inspector subscribed to that channel how many events it missed, so a gap in the stream is visible instead of silent.

## Wiring a backend

The bundled Apple and Hydrolysis backends do this for you, for the channels listed [above](#what-the-inspector-shows). A backend of your own takes the pieces it can feed and ignores the rest. This sketch follows the startup the bundled backends run, on the thread that owns the UI:

```rust,ignore
// Requires the `inspector` feature; `observe_signals` also needs `inspector-signals`.
use waterui::Environment;
use waterui::inspector::{self, InspectorRuntime};

fn start(mut environment: Environment) {
    let inspector = inspector::maybe_init_from_env("my-backend");

    // Add to your `tracing_subscriber` registry.
    let layer = inspector.as_ref().map(InspectorRuntime::tracing_layer);
    // Pass to `waterui::task::monitored_local_executor_with_probes`.
    let probe = inspector.as_ref().map(InspectorRuntime::runtime_probe);
    // Observation lasts only as long as this scope is held.
    let _signals = inspector.as_ref().map(InspectorRuntime::observe_signals);

    // Publishes the frame and tree recorders, and the runtime itself.
    inspector::install(&mut environment, inspector);

    // ... run the event loop here, so `_signals` lives as long as the UI.
}
```

`maybe_init_from_env` takes the backend name the inspector reports, as a `&'static str`. `runtime_probe` returns an `Arc<dyn RuntimeProbe>` for `waterui::task::monitored_local_executor_with_probes`, which wraps your main-thread executor and measures each poll against the frame budget of a `RefreshRate`. `tracing_layer` returns an `InspectorLayer`, a `tracing_subscriber` layer. `observe_signals` must be called on the thread that owns the UI, because the reactive graph is confined to it.

`install(&mut environment, inspector)` takes the `Option<InspectorRuntime>` by value and inserts a `FrameRecorder`, a `TreeRecorder` and the `InspectorRuntime` into the `Environment`. Passing `None` does nothing, so a backend running without an inspector simply finds nothing there. The rendering code then reads the recorders back with `environment.get::<FrameRecorder>()` and `environment.get::<TreeRecorder>()`, and an "Inspect element" action reads `InspectorRuntime` to call `inspect_node`.

Both recorders answer `is_active()` with one atomic load, so a backend skips the measurement entirely when nobody is watching. `FrameRecorder::record` takes a `FrameSample`. `TreeRecorder::record_snapshot(root, focus, nodes)` takes the root `NodeId`, the focused node if any and every `TreeNode`, and diffs them into a delta itself, so every backend gets the same incremental stream. The protocol types are reachable as `waterui::inspector::protocol`, for example `waterui::inspector::protocol::FrameSample`.

## Where to go next

- [Preview](01-preview.md) renders a single view to an image when you only need to see it.
- [Benchmarks](04-benchmarks.md) turns frame-time and counter budgets into a gate you can run repeatedly.
- [The MCP server](06-mcp.md) lets an agent drive a live app turn by turn.

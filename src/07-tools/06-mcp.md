# Driving your app from an agent

> **In this chapter, you will:**
>
> - Start `water mcp` and connect an agent through the generated `.mcp.json` or a manual registration
> - Read the live accessibility tree, act on nodes by stable id, and take screenshots
> - Wait on conditions instead of polling, and step through an animation frame by frame
> - Run the edit, `restart`, verify loop, and know when to reach for the CLI instead

An agent that edits your UI needs to see the result. A screenshot of a finished build is not enough: it has to press a button, watch what changed, and confirm that the thing it just edited behaves. `water mcp` serves your app to an agent over the Model Context Protocol (MCP) so it can do exactly that, headless and without a simulator.

Everything the agent sees is the app's real accessibility tree. These are the same nodes that `#[waterui::test]` drives in [the testing chapter](03-testing.md), so what you taught the tree with `.a11y_label(..)` and `.a11y_id(..)` ([the modifiers chapter](../02-core/04-modifiers.md), [the accessibility chapter](../06-advanced/05-accessibility.md)) is what the agent reads and addresses.

## Start the server

Run `water mcp` in the project root. It starts serving on stdio at once and builds the app in the background for a headless Hydrolysis session on your host.

```bash
water mcp
water mcp --path ./my-app --viewport 800x600 --scale 1
water mcp --offline
```

| Flag | Default | Meaning |
|------|---------|---------|
| `--path` | `.` | The project directory. |
| `--viewport` | `390x844` | Viewport size in logical pixels, `WIDTHxHEIGHT`. Fixed for the whole session; there is no resize tool. |
| `--scale` | `2.0` | Display scale factor of the headless render. Must be a positive finite number. |
| `--offline` | off | Serve without network access, like `cargo build --offline`; `CARGO_NET_OFFLINE` is honored the same way. A declared font missing from the font cache then fails the build instead of being fetched. |

stdout carries the protocol, so `water mcp` refuses to run with `--json`.

The server answers `initialize` and `tools/list` immediately, so a cold first build does not stall the client handshake. A tool call waits until the build finishes. A build that fails does not end the session: every call to a tool other than `preview` reports the build error until a `restart` builds again.

## Connect a client

`water create` writes a project-level `.mcp.json` when the project has none:

```json
{
  "mcpServers": {
    "app": {
      "command": "water",
      "args": ["mcp"]
    }
  }
}
```

The file is yours: `water` never overwrites an existing one. The command carries no `--path` because a client launched in the project root already has the right working directory. Add `--viewport` or `--scale` to `args` when you want a different render.

A client that does not read `.mcp.json` takes the same registration by hand. For Codex, in `~/.codex/config.toml`:

```toml
[mcp_servers.app]
command = "water"
args = ["mcp"]
```

Cursor reads `.cursor/mcp.json`, which has the same shape as the generated file.

## The tool surface

| Tool | What it does | Returns |
|------|--------------|---------|
| `snapshot` | Read the accessibility tree | Text lines, or JSON with `format: "json"` |
| `find` | Locate nodes by criteria | The matching node lines; an error when nothing matches |
| `act` | Perform a semantic action on a node | The settled tree |
| `pointer` | Tap, drag, hover, scroll, pinch | The settled tree |
| `key` | Press a character or named key, with modifiers | The settled tree |
| `type_text` | Type into the focused input | The settled tree |
| `drop_files` | Deliver an OS file drop at a point or node | The settled tree |
| `wait` | Block until expectations hold | `fulfilled`, `timed out` and similar, plus the tree |
| `screenshot` | Capture the current frame | A PNG |
| `advance` | Step the virtual clock | `animating` or `settled` plus the tree, or a PNG |
| `restart` | Rebuild and relaunch the app | The fresh tree |
| `preview` | Render one `#[preview]` function or expression | A PNG |

`preview` is served by the `water mcp` process itself rather than the running app, so it does not wait for the app to start. Its own build still queues behind the app's on Cargo's build lock.

Every mutating tool returns the settled tree. Never follow one with a `snapshot` just to see the result.

### Reading the tree

`snapshot` prints a header line, then one node per line, indented by depth: `#<id> <role> "<label>"`, then its value, state flags, the actions it supports, and its bounds. Parts a node does not have are left out.

```text
revision=3 viewport=200x100 focus=#2
  #2 button "Save" [focused] actions=[click,focus] bounds=8,8,64,32
```

The number after `#` is what every other tool takes as `node`.

Node ids are stable across turns until `restart`. An agent can therefore keep an id from one call and act on it in the next, but must re-read the tree after a restart.

Give the nodes an agent will need a stable `a11y_id`. Labels change with copy and localization; an identifier does not.

```rust
use waterui::prelude::*;

fn counter() -> impl View {
    let count: Binding<i32> = binding(0);

    vstack((
        text!("Count: {count}").a11y_id("counter.value"),
        button("Increment")
            .action(|State(c): State<Binding<i32>>| *c.get_mut() += 1)
            .state(&count)
            .a11y_id("counter.increment"),
    ))
}
```

`find` filters without dumping the whole tree. Its criteria are `role` (the snake_case name `snapshot` prints, such as `button`, `text_input`, `check_box` or `scroll_view`), `label`, `label_contains`, `identifier` (the `a11y_id`), `value`, `value_contains`, and the state filters `enabled`, `selected`, `checked`, `mixed`, `expanded`, `busy` and `hidden`. Hidden nodes are excluded unless you pass `hidden: true`. `within` (descendants) and `children_of` (direct children) scope the search to one node id.

```json
{"role": "check_box", "checked": false}
{"identifier": "counter.increment"}
{"label_contains": "Wi-Fi", "within": 42}
```

The `wait` expectations below take the same criteria.

### Acting

Prefer `act`. It performs a semantic accessibility action on a node, so it keeps working when a layout change moves the node.

```json
{"node": 42, "action": "click"}
{"node": 7, "action": "set_value", "value": "0.8"}
{"node": 15, "action": "scroll_into_view"}
{"action": "clear_focus"}
```

The actions are `click`, `focus`, `set_value`, `increment`, `decrement`, `replace_text`, `expand`, `collapse`, `scroll_forward`, `scroll_backward`, `scroll_left`, `scroll_right` and `scroll_into_view`. `clear_focus` is app-level and takes no node.

Use `pointer` for what a semantic action cannot express. Its kinds are `tap`, `down`, `up`, `move`, `hover`, `secondary_click`, `drag`, `scroll` and `magnify`.

```json
{"kind": "tap", "x": 120, "y": 340}
{"kind": "tap", "node": 42, "x": 0.5, "y": 0.5}
{"kind": "drag", "node": 3, "x": 0.5, "y": 0.5, "to_node": 8, "to_x": 0.5, "to_y": 0.5, "steps": 8}
{"kind": "scroll", "node": 12, "x": 0.5, "y": 0.5, "dy": -3, "unit": "line"}
{"kind": "magnify", "node": 5, "x": 0.5, "y": 0.5, "factor": 2.0}
```

Without `node`, coordinates are viewport logical pixels. With `node` (or `to_node` for a drag's end), they are `0` to `1` fractions of that node's bounds, so `0.5, 0.5` is its center. `scroll` deltas are pixels unless you pass `unit: "line"`.

`key` presses and releases a single character or a W3C key name (`Enter`, `Tab`, `Escape`, `Backspace`, `Delete`, `ArrowLeft`, `ArrowRight`, `ArrowUp`, `ArrowDown`, `Home`, `End`, `PageUp`, `PageDown`, `F1` to `F12`), with optional `modifiers` from `shift`, `ctrl`, `alt` and `meta`. `type_text` enters text into the focused input, so focus it first with `act`. How focus and key handling work in the app itself is covered in [the keyboard and focus chapter](../06-advanced/09-keyboard-focus.md).

`drop_files` takes absolute `paths` and a point `x`, `y`, which a `node` anchor turns into fractions of that node's bounds, as with `pointer`. The view under the point receives the same hover-then-drop input a platform file drag produces, with every path in one payload, which exercises the handlers from [the drag and drop chapter](../06-advanced/10-drag-drop.md). There is no hover-only variant over MCP.

### Waiting

Do not poll `snapshot`. `wait` returns as soon as its expectations hold or `timeout_ms` (5000 by default) elapses.

```json
{"exists": {"role": "progress_indicator", "busy": false}}
{"focus": {"identifier": "form.name"}}
{"exists": {"label": "Saved"}, "focus": {"identifier": "form.name"}, "enforce_order": true}
{"exists": {"label": "Crash", "inverted": true}, "timeout_ms": 2000}
```

The expectations are `exists`, `not_exists`, `value_eq` (a selector plus the `value` it must reach) and `focus`, at most one of each. The result is `fulfilled`, `timed out`, `incorrect_order`, `inverted_fulfillment` or `interrupted`, followed by the tree.

`inverted: true` on an expectation forbids it. The wait fails with `inverted_fulfillment` the moment it becomes true, and a wait holding only inverted expectations runs the full timeout and then reports `fulfilled`. `enforce_order: true` requires the non-inverted expectations to fulfill in the fixed order `exists`, `not_exists`, `value_eq`, `focus`, whatever order the JSON lists them in; one landing early reports `incorrect_order`.

### Seeing the screen

`screenshot` returns a PNG of the current frame. Use it when layout or appearance matters; the tree already answers questions about content and state. Text in the tree tells you what is there, not whether it is clipped or overlaps.

## Step through an animation

Every mutating tool settles the virtual clock before returning, so a triggered animation normally lands at its end state and you never see the middle. To observe the middle:

1. Call the input tool (`pointer`, `act`, `key`, `type_text` or `drop_files`) with `settle: false`. The input is queued, not pumped, and the tool answers with a short "queued" note instead of the tree.
2. Call `advance` with `duration_ms` (one 16 ms frame by default). It steps the clock in fixed 16 ms frames and reports `animating` or `settled`, plus the tree. `duration_ms: 0` polls the status without pumping.
3. Call `advance` with `screenshot: true` to get, instead of the tree, a PNG rendered at exactly the instant the advance lands on. Repeat with `duration_ms: 16` for a per-frame flipbook until `advance` reports `settled`.

```json
{"kind": "tap", "node": 42, "x": 0.5, "y": 0.5, "settle": false}
{"duration_ms": 16, "screenshot": true}
```

The clock is virtual, so the frames do not depend on host timing, and the same sequence gives the same frames on every run. A bare `screenshot` is itself a pump: each capture advances the clock by one frame. How the animations you are stepping through are authored is in [the animation chapter](../06-advanced/01-animation.md).

## The development loop

The loop an agent runs against your project is:

1. `snapshot` to read the tree, or `find` for one node.
2. `act`, `pointer`, `key` or `type_text` by node id, reading the tree that comes back.
3. `screenshot` when appearance matters, `wait` when something is asynchronous.
4. Edit the source.
5. `restart`, then read the fresh tree and verify the change.

`restart` relaunches the app from scratch, so state resets and node ids from before it no longer refer to live nodes. Under `water mcp` it first rebuilds from the current sources, which is what makes edit, `restart`, `snapshot` a complete cycle. A call that was parked on a build when a `restart` arrived fails with a retry message rather than hanging.

To check a single component without driving the whole app, use `preview`. It takes the same target as `water preview`: a `#[preview]` function path such as `views::home`, or, with `expr: true`, an inline expression such as `text("hi")`. It also accepts `frame` (default `375x667`), `backend`, `theme`, `platform`, `device` and `content_timeout`, with the meaning the matching flags have in [the preview chapter](01-preview.md).

## Where MCP stops and the CLI starts

A live session an agent drives turn by turn is an MCP server, because the connection is the session. A pure function of its inputs is a CLI command, which keeps shell composition. `water preview` is both: it is a pure function, but its output is an image the model must look at, so it is a CLI command and the `preview` tool. That is also why frame benchmarks are `water bench` ([the benchmarks chapter](04-benchmarks.md)) rather than a tool here.

A scripted, repeatable interaction belongs in a `#[waterui::test]`, not in a transcript of an agent's tool calls. Use the MCP session to explore and to find the node ids and conditions, then write the test ([the testing chapter](03-testing.md)).

The session renders headless through Hydrolysis with the Material 3 style, at the viewport you chose, even on a Mac. It shows how your app lays out and behaves under that renderer, not how the Apple backend looks; for that, use the `preview` tool or `water preview` with the `apple` backend, or `water run`, as described in [the preview chapter](01-preview.md) and [the platforms chapter](../01-getting-started/05-platforms.md).

Next, [packaging](07-packaging.md) turns the app you have been driving into a release package for each platform.

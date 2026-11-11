# Frame benchmarks with water bench

> **In this chapter, you will:**
>
> - Write a `#[waterui::bench]` function that mounts a view and records named scenarios
> - Drive each measured frame with `redraw`, `scroll_at` and the `pointer_*` methods
> - Set budgets in the attribute and tighten them from `water bench`
> - Read the human, JSON and HTML reports and export `github-action-benchmark` data
> - Decide which budgets may gate CI and which numbers belong to a quiet device

A view that looks right can still be expensive: a scroll that rebuilds the whole
tree on every wheel tick, a clip stack that grows with the list, a surface layer
per row. `#[waterui::bench]` mounts a view in the offscreen Hydrolysis runtime,
runs a scenario for many frames, and records timing plus renderer counters. `water
bench` runs every bench in a crate, aggregates the reports and fails when a budget
is blown.

Benches sit beside your tests and use the same dev-dependencies as
[the testing chapter](03-testing.md), so read that first if `waterui-testing` is
new to you.

## A first bench

A bench names a no-argument view function, a `theme`, and receives a `&mut PerfApp`
to record scenarios on.

```rust,ignore
use waterui::prelude::*;
use waterui_testing::PerfApp;

fn feed() -> impl View {
    // The view under measurement.
    text("Feed")
}

#[waterui::bench(feed, theme = hydrolysis_m3::Material3::defaults(), viewport = (390, 844), max_rebuild_ratio = 0.1)]
fn feed_frames(perf: &mut PerfApp) {
    perf.measure("steady-redraw", |run| run.redraw());
    perf.measure("wheel-scroll", |run| {
        run.scroll_at(195.0, 600.0, 0.0, -24.0, false);
    });
}
```

This fence is `rust,ignore` because the theme comes from a style package
(`hydrolysis_m3` here), which the book crate does not depend on. Any value
implementing both `hydrolysis::Style` and `Clone` works, because every
repetition mounts the view again with a clone of the style. `theme =` is
required: a bench measures rendered frames, and rendering needs a style. The
macro rejects a bench without one.

The attribute accepts, in this shape:

| Argument | Meaning |
| --- | --- |
| first path, no `=` | The no-argument view function to mount |
| `theme = <style>` | The style to render with (required) |
| `viewport = (width, height)` | Window size in logical pixels, as `u32`s (default 390 × 844) |
| `max_p95_us`, `max_mean_us` | Frame-time budgets in microseconds |
| `max_rebuild_ratio` | Share of sampled frames that rebuilt, `0.0` to `1.0` |
| `max_scene_layers` | Most compositor layers one frame may submit |
| `max_gpu_surface_layers` | Most embedded GPU surface mounts in one frame |
| `max_clip_layers` | Most clip layers one frame may push |

Every budget is optional, and each applies to every measurement the bench records.
Repeating one is a compile error. The macro expands to a plain `#[test]` named
`waterui_bench_<function name>`; that prefix is how `water bench` finds benches.

A bench function is synchronous and, in this form, returns nothing. If it records
no measurement, the test fails with a message asking you to call `measure`.

## What a measurement does

`PerfApp::measure(name, automation)` takes a scenario name and a closure
`FnMut(&mut PerfRun<'_>)`. For each repetition it mounts a fresh app, then runs
the warmup frames, then the sampled frames. **Before every frame it calls your
closure once**, and then advances exactly one frame. Whatever the closure queued is
what that frame processes, and only the sampled frames are timed.

`PerfRun` offers:

| Method | Queues for the next frame |
| --- | --- |
| `redraw()` | A redraw with no state change |
| `scroll_at(x, y, dx, dy, is_line_delta)` | A wheel or trackpad scroll event |
| `pointer_move(x, y)` | A mouse move |
| `pointer_down(x, y)` / `pointer_up(x, y)` | A primary button press or release |
| `app()` | Nothing by itself: the full `OffscreenApp`, for queries mid-run |
| `frame()` | Nothing: advances one extra frame now and returns its `FrameTiming`, which the measurement does not record |

The `pointer_*`, `scroll_at` and `redraw` calls only queue work, so the measured
frame is the one that processes it. Actions you run through `app()`, such as
`run.app().query().label("Like").tap()`, are different: they settle the app
before returning, pumping their own untimed frames, so the measured frame that
follows sees the update already applied. To measure the cost of a state change,
change the state directly (see [benches that own state](#benches-that-own-state))
or queue input events.

Frames run on a synthetic 120 Hz clock, so a measurement does not wait on
wall-clock time. A scroll scenario that moves the content a little every frame:

```rust,ignore
perf.measure("steady-scroll", |run| {
    run.scroll_at(195.0, 600.0, 0.0, -24.0, false);
});
```

A drag is three kinds of input: a press, moves, and a release. The closure is
called once per frame, so it keeps its own frame counter. That counter lives as
long as the closure, across warmups and every repetition, while each repetition
mounts a fresh app. Write the drag as a repeating cycle rather than a single
gesture that starts at the first call:

```rust,ignore
let mut frame = 0_u32;
perf.measure("drag", move |run| {
    let phase = frame % 30;
    let x = 100.0 + phase as f32 * 4.0;
    match phase {
        0 => run.pointer_down(x, 400.0),
        29 => run.pointer_up(x, 400.0),
        _ => run.pointer_move(x, 400.0),
    }
    frame += 1;
});
```

`measure` can be called as many times as you like; each call is one named
scenario in the report.

## Benches that own state

When the view closes over a `Binding` that the scenario also mutates, use the
manual form. Leave out the view path; the function receives the configured
`UiBuilder` by value, calls `perf_with` itself, and returns the `PerfReport`.
The parameter's type names the style the `theme =` expression produces.

```rust,ignore
use waterui::prelude::*;
use waterui_testing::{PerfReport, Styled, UiBuilder};

#[waterui::bench(theme = hydrolysis_m3::Material3::defaults(), max_rebuild_ratio = 0.2)]
fn counter_updates(ui: UiBuilder<Styled<hydrolysis_m3::Material3>>) -> PerfReport {
    let count = Binding::i32(0);
    let count_for_view = count.clone();
    ui.perf_with(
        move || {
            let count = count_for_view.clone();
            text!("Count: {count}")
        },
        |perf| {
            perf.measure("increment", |_run| {
                *count.get_mut() += 1;
            });
        },
    )
}
```

`perf_with` takes the view function, which must be `Fn() -> V + 'static`, and a
closure that receives `&mut PerfApp`. That closure does not have to be
`'static`, so the scenario can borrow `count` directly. The write happens just
before each frame, so the frame that follows is the one that updates the label.

## Why the rebuild ratio matters

WaterUI updates are fine-grained: a `Binding` change reaches the nodes that read it
without recomputing the tree around them
([the reactive chapter](../02-core/02-reactive.md)). Hydrolysis keeps a retained
render tree and normally refreshes only the parts that changed. A frame counts
as *rebuilt* when it had to build that tree from scratch, or when it ran queued
local async tasks. A well-behaved scroll or text update refreshes and does not
rebuild, so the *rebuild ratio* (`rebuilt frames / sampled frames`) stays near
zero.

A regression that throws the retained tree away on every change, or that spawns
work on every input, shows up as a frame that rebuilds each time the input
arrives. Timings may still look fine on a fast machine. The ratio is a count, so
it does not move with the machine's speed, which is why `max_rebuild_ratio` is
the budget to reach for first.

The same holds for the three layer budgets. They are maxima over the sampled frames,
so a view that suddenly adds a surface or a clip per row raises them regardless of
hardware.

## Smoke mode under nextest

A plain `cargo nextest run` runs benches as tests. If none of
`WATERUI_BENCH_SAMPLES`, `WATERUI_BENCH_WARMUPS` or `WATERUI_BENCH_REPETITIONS` is
set, the bench runs in smoke mode: 0 warmups, 2 samples, 1 repetition, and no
budget enforcement. Every bench therefore keeps compiling and executing as a
cheap correctness check, and no timing is asserted.

Setting any of those variables switches to a full run. Budgets are then enforced
in the test process, so a blown budget panics and the test fails with a message
naming the bench, the measurement, the metric, the actual value and the limit.
Variables you leave unset fall back to the defaults of 10 warmups, 120 samples
and 7 repetitions. You normally never set these yourself; `water bench` does.

## Running water bench

```bash
water bench                             # every bench in the current crate
water bench scroll                      # benches whose test name contains "scroll"
water bench --path examples/stress      # another crate
water bench --samples 240 --warmups 20 --repetitions 5
water bench --format html -o report.html
water bench --format json               # to stdout, or to a file with -o
water bench --gha bench.json            # github-action-benchmark JSON
water bench --max-p95-us 8000           # tighten from the command line
```

`water bench` requires `cargo-nextest` and tells you how to install it if it is
missing. In the `--path` directory it runs `cargo nextest run --no-fail-fast`
with the filter expression `test(waterui_bench_)`, or
`test(waterui_bench_) & test(<FILTER>)` when you pass a filter, and puts the run
shape and any `--max-*` caps in the environment. Both are nextest substring
matches against the full test name, module path included.

| Flag | Default | Meaning |
| --- | --- | --- |
| `[FILTER]` | none | Substring matched against the test name |
| `--path` | `.` | Project or crate directory with the benches |
| `--samples` | 120 | Recorded frames per measurement (must be above 0) |
| `--warmups` | 10 | Unrecorded frames before sampling |
| `--repetitions` | 7 | Independent repetitions, each with a fresh app (must be above 0) |
| `--report-dir` | temp dir | Keep the per-bench report JSON files here; `.json` files already in it are deleted first |
| `--format` | `human` | `human`, `json` or `html` |
| `-o`, `--output` | stdout, or `waterui-bench-report.html` | Destination for `json` and `html` |
| `--gha` | none | Write `customSmallerIsBetter` JSON for `github-action-benchmark` |
| `--max-p95-us`, `--max-mean-us` | none | Cap the frame-time budgets |
| `--max-rebuild-ratio` | none | Cap the rebuild ratio (`0.0` to `1.0`) |
| `--max-scene-layers`, `--max-gpu-surface-layers`, `--max-clip-layers` | none | Cap the layer counts |

The `--max-*` flags only tighten. Each is merged with the attribute budget and the
smaller limit wins, so the command line can never loosen a budget a bench declares,
and it can add a budget to a bench that declares none. If the run fails, whether a
bench panicked or exceeded a budget, `water bench` exits with an error. If no
bench produced a report, it says the crate has no benches (matching the filter,
when you gave one).

The three formats share the same data. `human` prints one block per measurement:
sample, rendered and idle frame counts, mean, median, p95, min, max, rebuilt
frames, frames that missed 120 and 60 fps, phase timings (rebuild, build,
dispatch, finish, render, animation, input), layer counts and the deepest clip
nesting, measurement-cache hits and misses, and for every configured budget the
actual value, the limit and the headroom. `json` is the stable form for scripts.
`html` writes one self-contained page with per-measurement charts, including
frame timelines, a phase breakdown and CPU and memory use. `--gha` writes the frame p95 and mean in
microseconds for each measurement, named `<crate>/<bench>/<measurement> <metric>`.

## What may gate, and what may not

Frame times are real wall-clock readings of a GPU pipeline. Shared CI runners are
noisy enough that the noise exceeds the effects you are trying to catch, so
timing taken there never gates a merge. Collect wall-clock numbers on a quiet,
real device, and compare them against numbers from the same machine.

Counters are different. The rebuild ratio and the three layer counts are
deterministic for a given view and scenario, so they are safe CI gates. A sensible
split:

- In the attribute, set `max_rebuild_ratio` and the layer caps. These are
  enforced in every full run, whatever machine it runs on.
- Leave `max_p95_us` and `max_mean_us` out of CI. Pass them with `--max-p95-us` or
  `--max-mean-us` when you run on a device whose numbers you trust, or track
  the `--gha` output over time instead.

A bench measures the offscreen Hydrolysis pipeline, which is a view's per-frame
cost under the self-drawn renderer. It does not measure a native Apple backend,
and it says nothing about platforms you did not run it on.

## Next steps

- Look at what a view actually renders in [the inspector](05-inspector.md).
- Cover behavior, not cost, with [tests](03-testing.md).
- See why frames rebuild in [the rendering internals](../08-internals/01-rendering.md).

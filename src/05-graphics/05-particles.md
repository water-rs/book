# Particle systems

> **In this chapter, you will:**
> - Describe a GPU particle effect as a single builder chain
> - Aim emitters, motion, and collisions in normalized coordinates
> - Drive live parameters from bindings so an effect follows app state
> - Place a particle system in a layout and render frames offscreen for review

A particle effect is drawn content, not a control, so `ParticleSystem` is self-drawn on every backend rather than bridged to a platform emitter such as Core Animation's. Compute passes simulate every particle and one instanced draw call renders them, the same way everywhere. Its body is a [`GpuContentView`](02-gpu-surface.md) hosting the particle renderer, so it lays out and composes like any other GPU view.

> **Feature flag:** particles are the split crate `water-rs/particle`, re-exported by the facade as `waterui::particle` behind the `particle` feature. Add `waterui = { version = "0.5", features = ["particle"] }` to `Cargo.toml`; the module does not exist without it. Its public surface is `ParticleSystem`, the `particles` constructor, `BlendMode`, `EmitterShape`, and `ParticleShape`.

## Quick start

```rust
use core::f32::consts::PI;
use waterui::particle::ParticleSystem;
use waterui::prelude::*;

fn rain() -> impl View {
    ParticleSystem::new(8_000)
        .emit_from_rect(1.4, 0.08)
        .at(0.5, -0.04)
        .rate(480_000.0)
        .life(0.6, 0.8)
        .speed(2.4, 4.2)
        .angle(PI * 0.49, PI * 0.51)
        .size(0.0008, 0.0015)
        .color(
            Color::srgb_hex("#D5E8FF").with_opacity(0.45),
            Color::srgb_hex("#E8F5FF").with_opacity(0.0),
        )
        .gravity(0.0, 5.0)
        .stretch_with_velocity()
}
```

Ranges take two arguments, not a Rust range: `life(0.6, 0.8)` means "somewhere between 0.6 and 0.8 seconds", drawn per particle when it spawns. `particles(8_000)` is the free-function equivalent of `ParticleSystem::new(8_000)`.

## Coordinates and units

Positions, emitter extents, collider bounds, speeds, and accelerations are fractions of the system's own frame: x of its width, y of its height, with `[0.0, 0.0]` at the top-left corner and `[1.0, 1.0]` at the bottom-right. A configuration therefore scales with the view, and on a frame that is not square the same speed covers more points along the longer side. Sprite size is the exception: `size` is a radius measured against the frame's shorter side, so round particles stay round at any aspect ratio. Emitting slightly outside the frame (`at(0.5, -0.04)`) is how you get particles that drift in from off-screen.

Angles are radians from the +x axis with y pointing **down**: `0.0` aims right, `PI * 0.5` down, `PI` left, `PI * 1.5` up. Durations are seconds.

## The emitter

| Modifier | Meaning |
|---|---|
| `at(x, y)` | Emitter center (default `0.5, 0.5`) |
| `rate(per_second)` | Emission rate (see below) |
| `emit_from_point()` | Spawn from a single point (default) |
| `emit_from_rect(width, height)` | Spawn anywhere inside a rectangle centered on `at` |
| `emit_from_circle(radius)` | Spawn anywhere inside a disk centered on `at` |

The shape modifiers produce the `EmitterShape` values `Point`, `Rect { width, height }`, and `Circle { radius }`. The enum is `#[non_exhaustive]`, and no method takes it directly: you pick a shape through these modifiers.

`ParticleSystem::new(max_particles)` allocates a fixed pool of slots once, and particles only ever recycle dead slots. Each frame, every free slot independently has a `rate * dt / max_particles` chance of respawning, so emission throttles itself as the pool fills: with the pool empty the system emits about `rate` particles per second, and at 90% occupancy roughly a tenth of that. Saturated effects therefore ask for far more than `max_particles / average_life`. Once `rate * dt` reaches `max_particles`, every free slot refills on the next frame and a higher rate changes nothing; the rain above requests 480,000/s from an 8,000-slot pool, which at 60 fps is exactly that point. Tune `rate` by eye against the pool size rather than treating it as an exact count.

## Particle properties

| Modifier | Meaning |
|---|---|
| `life(min, max)` | Lifetime in seconds |
| `speed(min, max)` | Initial speed magnitude |
| `angle(min, max)` | Initial direction in radians |
| `size(min, max)` | Sprite radius, as a fraction of the frame's shorter side |
| `spin(min, max)` | Rotation speed in radians per second; each particle also starts at a random rotation |
| `color(start, end)` | Tint at birth and at death |
| `softness(value)` | Edge falloff, `0.0` hard to `1.0` soft (default `0.5`) |
| `shape(ParticleShape)` | `Circle` (default) or `Rect` SDF sprite |
| `stretch_with_velocity()` | Stretch the sprite along its velocity vector |

`color` takes anything that converts to a `Color` signal, and the colors resolve against the environment, so theme colors work alongside hex literals. Every particle in a system shares the same pair and fades from `start` to `end` over its own lifetime, so one system draws one color ramp; layer several systems in a `zstack` for a multicolored effect. Without `color`, particles fade from opaque white to transparent.

Rotation only shows on `ParticleShape::Rect`, since a circle looks the same at any angle. `Rect` sprites are drawn as narrow strips whose width also oscillates over time, which reads as confetti tumbling.

```rust
use core::f32::consts::{PI, TAU};
use waterui::particle::{ParticleShape, ParticleSystem};
use waterui::prelude::*;

fn confetti() -> impl View {
    ParticleSystem::new(20_000)
        .emit_from_circle(0.05)
        .at(0.5, 0.5)
        .rate(1_200_000.0)
        .life(0.8, 1.5)
        .speed(0.5, 3.0)
        .angle(0.0, TAU)
        .size(0.003, 0.008)
        .spin(-PI, PI)
        .shape(ParticleShape::Rect)
        .softness(0.0)
        .gravity(0.0, 3.0)
}
```

## Forces

`life`, `speed`, `angle`, `size`, and `spin` are fixed per particle at birth. Forces then act on every live particle each frame.

| Modifier | Meaning |
|---|---|
| `gravity(x, y)` | Constant acceleration |
| `wind(x, y)` | Constant acceleration added alongside gravity |
| `turbulence(value)` | Strength of a random horizontal velocity jitter applied each frame |
| `drag(factor)` | Velocity retained per 60 fps frame (`1.0` = none), frame-rate independent |

```rust
use waterui::particle::ParticleSystem;
use waterui::prelude::*;

fn drifting_dust() -> impl View {
    ParticleSystem::new(2_000)
        .emit_from_rect(1.5, 0.2)
        .at(0.5, 1.1)
        .rate(40_000.0)
        .life(8.0, 12.0)
        .speed(0.02, 0.08)
        .gravity(0.0, -0.01)
        .wind(0.02, 0.0)
        .turbulence(0.2)
        .drag(0.98)
}
```

## Blending

`additive()` makes overlapping particles brighten each other, which is what fire, sparks, and glow need. The default is `BlendMode::Alpha`; `BlendMode` is `#[non_exhaustive]` and `additive()` is the modifier that selects the other variant.

```rust
use core::f32::consts::PI;
use waterui::particle::ParticleSystem;
use waterui::prelude::*;

fn flame() -> impl View {
    ParticleSystem::new(3_000)
        .emit_from_rect(0.05, 0.0)
        .at(0.5, 0.82)
        .rate(180_000.0)
        .life(0.4, 0.8)
        .speed(0.5, 1.2)
        .angle(PI * 1.4, PI * 1.6)
        .size(0.03, 0.06)
        .color(
            Color::srgb_hex("#FFB433").with_opacity(0.6),
            Color::srgb_hex("#FF2A0D").with_opacity(0.0),
        )
        .gravity(0.0, -1.0)
        .softness(0.6)
        .additive()
}
```

## Collisions and interaction

| Modifier | Meaning |
|---|---|
| `collide_with_viewport()` | Keep particles inside the normalized `[0,0]..[1,1]` rectangle |
| `collide_with_rect(x, y, width, height)` | Keep particles inside a rectangle whose top-left corner is `(x, y)` |
| `collide_with_circle_obstacle(x, y, radius)` | Bounce off a disk centered on `(x, y)`; call it again for more obstacles |
| `bounce(restitution)` | Fraction of normal velocity kept on impact (default `1.0`) |
| `surface_friction(value)` | Fraction of tangential velocity kept on impact (default `1.0`) |
| `collide_with_particles(radius, strength)` | Particle-to-particle repulsion on the GPU |

```rust
use waterui::particle::ParticleSystem;
use waterui::prelude::*;

fn pinball() -> impl View {
    ParticleSystem::new(6_000)
        .emit_from_circle(0.02)
        .at(0.5, 0.18)
        .rate(90_000.0)
        .life(4.0, 6.0)
        .speed(0.5, 1.4)
        .gravity(0.0, 1.4)
        .collide_with_rect(0.08, 0.08, 0.84, 0.84)
        .collide_with_circle_obstacle(0.5, 0.36, 0.08)
        .bounce(0.82)
        .surface_friction(0.9)
        .collide_with_particles(0.01, 16.0)
}
```

A collision rectangle clamps every particle back inside it, so nothing escapes it. A circle obstacle is tested only where a particle lands each frame, so keep obstacles large relative to a particle's per-frame travel (`speed * dt`); a fast particle can step through a small one. `collide_with_particles` makes every particle walk the neighbor grid around it each frame, a cost that grows as particles crowd together, so leave it off unless the clumping is visible.

## Live parameters

Every numeric builder argument and both `color` arguments are signals. Literals become constants; a `Binding` or `Computed` stays live. The view samples each signal once per frame, so the surrounding view is never rebuilt and particles already in flight keep their trajectories. Numeric signals convert freely, so a `Binding<f64>` from a slider feeds an `f32` parameter directly.

```rust
use core::f32::consts::PI;
use waterui::particle::ParticleSystem;
use waterui::prelude::*;

fn snowfall() -> impl View {
    let density = Binding::f64(0.25);
    let drift = Binding::f64(0.0);

    vstack((
        ParticleSystem::new(4_000)
            .emit_from_rect(1.4, 0.05)
            .at(0.5, -0.03)
            .rate(density.map(|value| value * 400_000.0))
            .life(4.0, 7.0)
            .speed(0.05, 0.12)
            .angle(PI * 0.45, PI * 0.55)
            .size(0.004, 0.01)
            .gravity(0.0, 0.05)
            .wind(drift.clone(), 0.0)
            .softness(0.8),
        slider("Snowfall density", &density),
        slider("Wind drift", &drift).range(-0.3..=0.3),
    ))
}
```

`density.map(...)` derives the emission rate without reading the binding, which keeps the dependency visible to the system. See [Reactive state](../02-core/02-reactive.md) for why you derive instead of snapshot.

Which parameters respond, and when:

- **Immediately, for all live particles:** `gravity`, `wind`, `turbulence`, `drag`, collision bounds, obstacle positions and radii, `bounce`, `surface_friction`, interaction `radius` and `strength`, `color`, `softness`.
- **For the next spawns only:** `at`, `rate`, the emitter shape and its extent, `life`, `speed`, `angle`, `size`, `spin`; particles already alive keep where and how they were born.
- **Fixed when the system is built:** `max_particles`, `shape`, `additive`, `stretch_with_velocity`, the *number* of circle obstacles, and whether bounds collision or particle interaction is switched on at all. These take plain values, not signals; changing one means constructing a new `ParticleSystem`.

## Placing a system in a layout

`ParticleSystem` is a `View` whose body is a `GpuContentView` (see [GPU content](02-gpu-surface.md)). It reports no intrinsic size, so it stretches on both axes, takes whatever size it is offered, and composes like any other view:

```rust
use waterui::particle::ParticleSystem;
use waterui::prelude::*;

fn celebration_card() -> impl View {
    zstack((
        ParticleSystem::new(2_000)
            .emit_from_rect(1.0, 0.0)
            .at(0.5, -0.05)
            .rate(120_000.0)
            .life(1.4, 2.4)
            .speed(0.3, 0.6)
            .size(0.005, 0.012)
            .gravity(0.0, 0.6),
        vstack((
            text("Order placed"),
            text("Thanks for shopping with us."),
        ))
        .padding(),
    ))
}
```

On an axis its parent leaves unspecified, such as a scroll view's scrolling axis, it measures zero. Pin its dimensions with a frame (see [Layout](../03-ui/02-layout.md)):

```rust
use waterui::prelude::*;

fn pinned_celebration() -> impl View {
    celebration_card().size(320.0, 180.0)
}

# fn celebration_card() -> impl View {
#     text("Order placed")
# }
```

On a bare `ParticleSystem`, `.size(min, max)` is its own sprite-radius modifier and takes precedence over the frame modifier of the same name, so pin a bare system with `.width(320.0).height(180.0)` instead.

## Rendering offscreen

For snapshots and visual review, `ParticleSystem::render_offscreen` simulates the system without a window and returns the pixels. It is synchronous and takes a `&GpuRuntime`, which owns the GPU device and queue, so create the runtime once and reuse it. `OffscreenRenderer` renders scenes, pictures, and drawings, and `GpuRuntime::render_content` renders a single frame of `GpuContent` you own; a simulation needs many frames and `ParticleSystem` keeps its renderer private, so it has this entry point of its own.

```rust
use core::f32::consts::TAU;
use core::num::NonZeroU32;
use core::time::Duration;
use waterui::graphics::{GpuRuntime, OffscreenImage, OffscreenSize};
use waterui::particle::ParticleSystem;
use waterui::prelude::*;

async fn export_burst(path: &str) -> Result<(), Box<dyn core::error::Error>> {
    let runtime = GpuRuntime::new().await?;
    let size = OffscreenSize::try_from_pixels(600, 600).ok_or("size must be non-zero")?;
    let frames = NonZeroU32::new(8).ok_or("frame count must be non-zero")?;

    let readback = ParticleSystem::new(20_000)
        .emit_from_circle(0.05)
        .at(0.5, 0.5)
        .rate(1_200_000.0)
        .life(0.8, 1.5)
        .speed(0.5, 3.0)
        .angle(0.0, TAU)
        .size(0.003, 0.008)
        .render_offscreen(
            &runtime,
            size,
            &Environment::new(),
            frames,
            Duration::from_secs_f64(1.0 / 60.0),
        )?;

    OffscreenImage::from_readback(&readback).save_png(path)?;
    Ok(())
}
```

The caller supplies the simulation interval: an initial frame sets the simulation up, then `frames` steps of `frame_interval` advance it, with no wall-clock waiting. At 1/60 s per step, 8 frames is about 133 ms of simulated time. The pool starts empty and new particles appear at the emitter, so a capture after one or two steps shows little more than a cluster at `at`; choose `frames` times `frame_interval` to reach the moment you want to see.

The system renders into a 16-bit float target, and the returned `Readback` holds its premultiplied linear Display P3 pixels as `f32`, so bright additive overlaps keep values above `1.0`. `OffscreenImage::from_readback` converts them to straight-alpha sRGB8 for a PNG, clipping what falls outside sRGB. Emission is seeded randomly each frame, so two runs of the same configuration look alike but are not identical; review the images by eye rather than comparing hashes. The testing tools for views are covered in [Testing](../07-tools/03-testing.md).

## Next

[Animated gradients](06-gradients.md) covers the other end of the GPU component range: full-surface color fields that animate without any per-element simulation.

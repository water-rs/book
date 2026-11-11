# Maps and location

> **In this chapter, you will:**
> - Place a map, set its region, and drop annotations
> - Drive the camera and the user-location marker from reactive signals
> - Read the device location through `waterkit-location`, permission first
> - Install and configure the GPU vector map that self-drawn backends render

> **Dependencies:** The `waterui` facade has no `map` module or feature. The map is its own crate, so add it to `Cargo.toml` next to `waterui` and import from `waterui_map`:
>
> ```toml
> waterui-map = "0.5"
> waterui-map-gpu = "0.3"      # only for platforms without a native map
> waterkit-permission = "0.2"  # only to request the location permission
> ```

`waterui-map` gives you the `Map` view and its geographic types. Device location comes from `waterkit-location`, which the map crate re-exports, so the location types need no second dependency:

```rust,ignore
use waterui_map::location;                                    // the whole waterkit-location crate
use waterui_map::{Latitude, Location, Longitude, OutOfRange, Timestamp};
```

`Permission` and `PermissionStatus` are re-exported by `waterkit_location` too, but the `request` function that shows the permission prompt lives in `waterkit-permission`.

---

## Coordinates and regions

Latitude and longitude are validated newtypes, not bare `f64`. Build a coordinate from degrees and handle the range error:

```rust,ignore
use waterui_map::{Coordinate, OutOfRange};

fn landmarks() -> Result<(Coordinate, Coordinate), OutOfRange> {
    let manhattan = Coordinate::from_degrees(40.7580, -73.9855)?;
    let tokyo = Coordinate::from_degrees(35.6762, 139.6503)?;
    Ok((manhattan, tokyo))
}
```

`Coordinate::new` is the infallible constructor for values that are already `Latitude` and `Longitude`. A `Location` from the device carries exactly those, so converting one never fails:

```rust,ignore
use waterui_map::{Coordinate, Location};

fn to_coordinate(location: &Location) -> Coordinate {
    Coordinate::from_location(location)
}
// `From<Location>` and `From<&Location>` do the same thing.
```

A `Region` is a center plus a span in degrees. Smaller deltas mean a tighter zoom. `center`, `latitude_delta` and `longitude_delta` are public fields:

```rust,ignore
use waterui_map::{Coordinate, Region};

fn midtown(center: Coordinate) -> Region {
    Region::new(center, 0.030, 0.050)
}

fn close_up(center: Coordinate) -> Region {
    Region::from_coordinate(center) // 0.05 x 0.05 degrees
}
```

`Region` implements `From<Coordinate>`, so `coordinate.into()` gives you that default span in one step. `Region::default()` is centered on 0,0 with a 0.1-degree span, so an app should start from a real region.

---

## Displaying a map

```rust,ignore
use waterui::prelude::*;
use waterui_map::{Coordinate, Map, Region};

fn city_map() -> impl View {
    let paris = Coordinate::from_degrees(48.8566, 2.3522).expect("valid coordinate");
    Map::new(Region::new(paris, 0.1, 0.1))
}
```

`Map` stretches on both axes, so it fills whatever space its parent offers. Constrain it with `.size(width, height)`, `.width(...)`, or `.height(...)`, or let it fill the window under an `absolute` layer. See [Layout](../03-ui/02-layout.md).

Every constructor takes `impl IntoComputed<_>`. `Region` and `Coordinate` are constant signals, so you can pass either a plain value or a signal for them. Pass a `Binding<Region>` and the camera follows it:

| Constructor | Free function | Input |
|---|---|---|
| `Map::new(region)` | `map(region)` | `Region` or a signal of one |
| `Map::centered_on(coordinate)` | `map_centered_on(coordinate)` | `Coordinate` or a signal of one, default zoom |
| `Map::centered_on_location(location)` | `map_centered_on_location(location)` | a signal of `Location`, default zoom |

`Location` is not a constant signal, so a bare `Location` value does not compile where a location signal is expected. Wrap it in a `Binding<Location>` or `Computed<Location>`. The same applies to `.user_location(...)` and `.follows_location(...)` below.

```rust,ignore
use waterui::prelude::*;
use waterui_map::{Map, Region};

fn zoomable(region: Region) -> impl View {
    let region: Binding<Region> = binding(region);
    // Writing to `region` moves the camera; the map view is never rebuilt.
    Map::new(region)
}
```

Zooming is a handler that writes a region with scaled deltas:

```rust,ignore
use waterui::prelude::*;
use waterui_map::Region;

fn zoom(region: &Binding<Region>, scale: f64) {
    let current = region.snapshot();
    region.set(Region::new(
        current.center,
        current.latitude_delta * scale,
        current.longitude_delta * scale,
    ));
}
```

The map only reads the region and never writes it back. When the user pans or pinches, the camera moves but your binding keeps the last region you set. So `zoom` scales that region, not the view the user panned to.

---

## Annotations

```rust,ignore
use waterui::prelude::*;
use waterui_map::{Annotation, Coordinate, Map, Region};

fn annotated_map() -> impl View {
    let sf = Coordinate::from_degrees(37.7749, -122.4194).expect("valid coordinate");
    let la = Coordinate::from_degrees(34.0522, -118.2437).expect("valid coordinate");
    let center = Coordinate::from_degrees(36.0, -120.0).expect("valid coordinate");

    Map::new(Region::new(center, 5.0, 5.0)).annotations(vec![
        Annotation::new(sf, "San Francisco"),
        Annotation::new(la, "Los Angeles").subtitle("City of Angels"),
    ])
}
```

An `Annotation` is built with `Annotation::new(coordinate, title)` and an optional `.subtitle(...)`. Its fields are private; read them back with `coordinate()`, `title()` and `get_subtitle()`. Because `.annotations()` accepts `impl IntoComputed<Vec<Annotation>>`, search results or live vehicle positions can be pushed straight in from a binding:

```rust,ignore
use waterui::prelude::*;
use waterui_map::{Annotation, Map, Region};

fn search_results(region: Binding<Region>, results: Binding<Vec<Annotation>>) -> impl View {
    Map::new(region).annotations(results)
}
```

---

## Map styles

```rust,ignore
use waterui::prelude::*;
use waterui_map::{Map, MapStyle, Region};

fn satellite_view(region: Region) -> impl View {
    Map::new(region).style(MapStyle::Satellite)
}
```

`MapStyle::Standard` (the default) is a road map, `Satellite` is imagery, and `Hybrid` overlays roads on imagery.

> **Platform difference.** On Apple platforms, MapKit draws all three. The GPU vector realization draws from MapLibre styles you name: `Standard` uses the main style URL, while `Satellite` and `Hybrid` need `MapGpuOptions::satellite_style_url` and `hybrid_style_url`. Asking for one without the matching URL does not fall back to the standard style; the map reports a load failure through [`MapStatus`](#load-status). See [How your map is realized](#how-your-map-is-realized).

---

## Load status

A map that fetches its own tiles can be loading, ready, or failed. Pass a caller-owned binding to `.status(&binding)` and the map writes `MapStatus` into it, so you can draw a spinner or an error over the map instead of an empty rectangle:

```rust,ignore
use waterui::prelude::*;
use waterui_map::{Map, MapStatus, Region};

fn observed_map(region: Region) -> impl View {
    let status: Binding<MapStatus> = binding(MapStatus::default()); // Loading
    let message = status.map(|status| match status.failure() {
        Some(reason) => reason.clone(),
        None if status.is_loading() => Str::from("Loading map"),
        None => Str::from(""),
    });
    vstack((Map::new(region).status(&status), text!("{message}").caption()))
}
```

`MapStatus` is `#[non_exhaustive]` with `Loading`, `Ready` and `Failed(Str)`. After a failure the map keeps drawing whatever it last had. Apple's MapKit realization reports the platform's own load callbacks.

---

## User location

Four builders touch the location marker, and they differ in who supplies the coordinates:

| Method | Effect |
|---|---|
| `.shows_user_location(true)` | Turns the marker on and lets the platform's own location service feed it |
| `.user_location(signal)` | Turns the marker on and draws `Location` values from your signal |
| `.optional_user_location(signal)` | Same, but `None` draws no marker, which is the state to use while a permission prompt is pending |
| `.follows_location(signal)` | Marker on, and the camera re-centers on every new value at the default 0.05-degree span. This replaces the region you passed to the constructor |

```rust,ignore
use waterui::prelude::*;
use waterui_map::{Location, Map, Region};

fn tracking_map(location: Binding<Option<Location>>, region: Binding<Region>) -> impl View {
    Map::new(region).optional_user_location(location)
}
```

> **Supply the signal off Apple.** `shows_user_location(true)` alone leaves the location signal empty. MapKit fills it in from CoreLocation; the GPU realization has no platform location service to fall back on, so it draws nothing. Feed it `user_location` or `optional_user_location` if you want the marker everywhere.

Supplying the signal yourself is also what keeps camera following, the marker, and the horizontal-accuracy circle driven by one source instead of drifting apart.

---

## Interaction and chrome

```rust,ignore
use waterui::prelude::*;
use waterui_map::{Map, Region};

fn thumbnail(region: Region) -> impl View {
    Map::new(region)
        .is_interactive(false) // no pan, no zoom: good for a list cell
        .shows_compass(false)
        .shows_scale(false)
}
```

All three default to on. With `is_interactive(false)`, the GPU realization does not install its drag and magnification gestures at all. Both realizations draw the compass and the scale bar. On Apple they are MapKit's own controls. The GPU realization paints them itself in the map's corners, and because its camera is always north-up, its compass needle never turns.

---

## Reading the device location

`Location::get()` does **not** prompt. It assumes the permission is already granted, and returns `LocationError::PermissionDenied` otherwise. Ask first through `waterkit-permission`, which is a direct dependency because `waterui` does not re-export it. The app also has to declare the permission in `Water.toml` (see [Project structure](../01-getting-started/04-project-structure.md)):

```toml
[permissions.location]
enable = true
description = "Required to show user location on the map"
```

```rust,ignore
use waterkit_permission::{Permission, PermissionStatus, request};
use waterui_map::Location;
use waterui_map::location::LocationError;

async fn current_location() -> Result<Option<Location>, LocationError> {
    match request(Permission::Location).await {
        Ok(PermissionStatus::Granted) => Location::get().await.map(Some),
        Ok(status) => {
            waterui::log::warn!("location permission: {status:?}");
            Ok(None)
        }
        Err(error) => {
            waterui::log::error!("permission request failed: {error}");
            Ok(None)
        }
    }
}
```

`request` resolves to a `PermissionStatus`: `Granted`, `Denied`, `Restricted`, or `NotDetermined`. The enum is `#[non_exhaustive]`, so keep a catch-all arm. It returns a `PermissionError` (`Unsupported` or `Platform`) when the platform cannot ask at all. Treat "not granted" as a state to render, not an error to swallow.

`Location::get()` has no deadline of its own. It resolves when the platform's location service answers, so if you want a bound, wrap the awaited future in a timeout.

A `Location` exposes its data through accessors. `latitude()` and `longitude()` return the `Latitude`/`Longitude` newtypes; call `.get()` for the underlying `f64`:

| Accessor | Type |
|---|---|
| `latitude()` / `longitude()` | `Latitude` / `Longitude` |
| `altitude()` | `Option<f64>` meters above sea level |
| `horizontal_accuracy()` / `vertical_accuracy()` | `Option<f64>` meters |
| `timestamp()` | `Timestamp` |

`LocationError` is reachable as `waterui_map::location::LocationError`. It is `#[non_exhaustive]` with `PermissionDenied`, `ServiceDisabled`, `NotAvailable`, `InvalidCoordinate(OutOfRange)`, and `Platform(String)`.

---

## How your map is realized

`Map` is a semantic view; the backend you build against decides how it is drawn, and which backend draws it is not a choice in your view code.

- **Apple platforms** bridge `MKMapView` from MapKit. Styles, compass, scale, and the CoreLocation-driven blue dot all come from the system. This is the platform's native map. Depending on `waterui-map` is the opt-in: the `water` CLI sees the crate in your dependency graph and compiles the MapKit bridge into the Apple backend.
- **Self-drawn backends** (Hydrolysis, which also serves Android) have no map engine of their own. They realize a map only when the app installs one. `waterui-map-gpu` is that realization: it fetches a MapLibre style and vector tiles and draws them with the same GPU pipeline as the rest of your UI.

WaterUI hosts no tile service, so the app does two things in its `app(env)` function: it inserts `MapGpuOptions` with a MapLibre style URL, and it calls `install`. Add `waterui-map-gpu` as a dependency; the `waterui` facade does not re-export it.

```rust,ignore
use waterui::Url;
use waterui::app::App;
use waterui::prelude::*;
use waterui_map::{Coordinate, Map};
use waterui_map_gpu::{MapGpuOptions, install};

fn root_view() -> impl View {
    let paris = Coordinate::from_degrees(48.8566, 2.3522).expect("valid coordinate");
    Map::centered_on(paris)
}

pub fn app(mut env: Environment) -> App {
    env.insert(MapGpuOptions::new(Url::new(
        "https://tiles.openfreemap.org/styles/positron",
    )));
    install(&mut env);
    App::new(root_view, env)
}
```

`install` yields to a native hook that is already registered, so on Apple the same code installs nothing and the MapKit bridge draws the map. You do not need a `cfg` gate.

Skipping either step fails fast instead of drawing a blank tile grid. Hydrolysis panics when it realizes a `Map` and no realization was installed, and the installed GPU hook panics when `MapGpuOptions` is missing from the environment.

The GPU realization fetches over the network, so the app must declare the `internet` permission. Without it, Android refuses DNS resolution and every tile request fails. `waterui-map-gpu` declares that it needs the permission, so `water` warns during an Android build when your `Water.toml` lacks it:

```toml
[permissions.internet]
enable = true
description = "Required to download map tiles"
```

Beyond the style URL, `MapGpuOptions` is a builder over the resources the realization may consume: `satellite_style_url`, `hybrid_style_url`, `maximum_style_bytes`, `maximum_tilejson_bytes`, `maximum_tile_bytes`, `tile_cache_bytes`, `maximum_in_flight_tile_requests`, `request_timeout`, `network_retry_policy`, and `camera_animation`. The defaults are:

- an 8 MiB style limit, a 2 MiB TileJSON limit and a 16 MiB per-tile limit
- a 128 MiB tile cache and six tile requests in flight
- a 20-second request timeout
- a 240 ms ease-in-out camera animation

Change them when you are on a metered connection or a tight memory budget.

```rust,ignore
use std::num::NonZeroU64;
use std::time::Duration;
use waterui::Url;
use waterui_map_gpu::MapGpuOptions;

fn options() -> MapGpuOptions {
    MapGpuOptions::new(Url::new("https://tiles.openfreemap.org/styles/positron"))
        .tile_cache_bytes(NonZeroU64::new(32 * 1024 * 1024).expect("non-zero"))
        .request_timeout(Duration::from_secs(10))
}
```

`MapNetworkRetryPolicy::new(attempts, initial_delay, maximum_delay)` builds the backoff policy, where `attempts` is a `NonZeroU32` that counts the first request. It panics if a delay is zero or the maximum is shorter than the initial delay. The default is four attempts starting at 250 ms and capped at 4 s.

---

## Putting it together

A map centered on Manhattan, with a button that requests permission and then moves the camera to the user:

```rust,ignore
use waterkit_permission::{Permission, PermissionStatus, request};
use waterui::prelude::*;
use waterui_map::{Coordinate, Location, Map, MapStyle, Region};

fn located_map() -> impl View {
    let region: Binding<Region> = binding(Region::new(
        Coordinate::from_degrees(40.7580, -73.9855).expect("valid coordinate"),
        0.030,
        0.050,
    ));
    let user_location: Binding<Option<Location>> = binding(None);
    let status: Binding<Str> = binding("Location not requested");

    let map = Map::new(region.clone())
        .style(MapStyle::Standard)
        .optional_user_location(user_location.clone())
        .shows_compass(true)
        .shows_scale(true);

    let locate = button("Use my location")
        .action_async(
            |State(location): State<Binding<Option<Location>>>,
             State(region): State<Binding<Region>>,
             State(status): State<Binding<Str>>| async move {
                match request(Permission::Location).await {
                    Ok(PermissionStatus::Granted) => match Location::get().await {
                        Ok(value) => {
                            region.set(Region::from_coordinate(Coordinate::from(&value)));
                            location.set(Some(value));
                            status.set("Following your location".into());
                        }
                        Err(error) => {
                            waterui::log::error!("location request failed: {error}");
                            status.set("Location unavailable".into());
                        }
                    },
                    Ok(other) => status.set(format!("Permission: {other:?}").into()),
                    Err(error) => {
                        waterui::log::error!("permission request failed: {error}");
                        status.set("Permission request failed".into());
                    }
                }
            },
        )
        .state(&user_location)
        .state(&region)
        .state(&status);

    vstack((map, locate, text!("{status}").caption()))
}
```

Note what does *not* happen here: no `watch`, no rebuild. Writing to `region` moves the camera, writing to `user_location` moves the marker, and the `Map` view itself is constructed once. The repository's `examples/map` is a fuller version of this, with floating zoom controls and a status panel.

---

## What's next

Next up: [WebView](03-webview.md), where you embed web content in your app, JavaScript bridges, cookie management, and navigation controls included. For other data views, see [Charts](06-charts.md).

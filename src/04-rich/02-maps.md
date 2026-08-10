# Maps and location

> **In this chapter, you will:**
> - Place a map, set its region, and drop annotations
> - Drive the camera and the user-location marker from reactive signals
> - Read the device location through `waterkit-location`, permission first
> - Configure the GPU vector map that non-Apple platforms render

> **Feature flag:** Maps live behind the `map` feature on `waterui`. Enable it in `Cargo.toml` (`waterui = { version = "...", features = ["map"] }`) so `waterui::map` is available.

Two crates are involved. `waterui-map` gives you the `Map` view and its geographic types; `waterkit-location` provides device location. The map crate re-exports the location crate, so you rarely need a second dependency:

```rust,ignore
use waterui::map::location;                  // the whole waterkit-location crate
use waterui::map::{Latitude, Location, Longitude, OutOfRange, Timestamp};
```

---

## Coordinates and regions

Latitude and longitude are validated newtypes, not bare `f64`. Build a coordinate from degrees and handle the range error:

```rust,ignore
use waterui::map::{Coordinate, OutOfRange};

fn landmarks() -> Result<(Coordinate, Coordinate), OutOfRange> {
    let manhattan = Coordinate::from_degrees(40.7580, -73.9855)?;
    let tokyo = Coordinate::from_degrees(35.6762, 139.6503)?;
    Ok((manhattan, tokyo))
}
```

`Coordinate::new` is the infallible constructor for values that are already `Latitude` and `Longitude` -- which is what a `Location` from the device hands you, so converting one never fails:

```rust,ignore
use waterui::map::{Coordinate, Location};

fn to_coordinate(location: &Location) -> Coordinate {
    Coordinate::from_location(location)
}
// `From<Location>` and `From<&Location>` do the same thing.
```

A `Region` is a center plus a span in degrees. Smaller deltas mean a tighter zoom:

```rust,ignore
use waterui::map::{Coordinate, Region};

fn midtown(center: Coordinate) -> Region {
    Region::new(center, 0.030, 0.050)
}

fn close_up(center: Coordinate) -> Region {
    Region::from_coordinate(center) // 0.05 x 0.05 degrees
}
```

`Region` implements `From<Coordinate>`, so `coordinate.into()` gives you that default span in one step. `Region::default()` sits at 0,0 with a 0.1-degree span -- useful in examples, useless in an app.

---

## Displaying a map

```rust,ignore
use waterui::View;
use waterui::map::{Coordinate, Map, Region};

fn city_map() -> impl View {
    let paris = Coordinate::from_degrees(48.8566, 2.3522).expect("valid coordinate");
    Map::new(Region::new(paris, 0.1, 0.1))
}
```

`Map` stretches on both axes, so it fills whatever space its parent offers. Constrain it with `.size(width, height)`, `.width(...)`, or `.height(...)`, or let it fill the window under an `absolute` layer.

Every constructor takes `impl IntoComputed<_>`, so a plain value and a signal are both accepted -- pass a `Binding<Region>` and the camera follows it:

| Constructor | Free function | Input |
|---|---|---|
| `Map::new(region)` | `map(region)` | `Region` |
| `Map::centered_on(coordinate)` | `map_centered_on(coordinate)` | `Coordinate`, default zoom |
| `Map::centered_on_location(location)` | `map_centered_on_location(location)` | `Location`, default zoom |

```rust,ignore
use waterui::View;
use waterui::map::{Map, Region};
use waterui::reactive::binding;

fn zoomable(region: Region) -> impl View {
    let region = binding(region);
    // Writing to `region` moves the camera; the map view is never rebuilt.
    Map::new(region)
}
```

---

## Annotations

```rust,ignore
use waterui::View;
use waterui::map::{Annotation, Coordinate, Map, Region};

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

An `Annotation` carries a `coordinate`, a `title: Str`, and an optional `subtitle: Option<Str>`. Because `.annotations()` accepts `impl IntoComputed<Vec<Annotation>>`, search results or live vehicle positions can be pushed straight in from a binding:

```rust,ignore
use waterui::map::{Annotation, Map, Region};
use waterui::{Binding, View};

fn search_results(results: Binding<Vec<Annotation>>) -> impl View {
    Map::new(Region::default()).annotations(results)
}
```

---

## Map styles

```rust,ignore
use waterui::map::{Map, MapStyle, Region};
use waterui::View;

fn satellite_view(region: Region) -> impl View {
    Map::new(region).style(MapStyle::Satellite)
}
```

`MapStyle::Standard` (the default) is a road map, `Satellite` is imagery, and `Hybrid` overlays roads on imagery.

> **Apple only.** `Satellite` and `Hybrid` are honored by the native MapKit realization. The GPU vector realization used on other platforms panics on anything but `Standard`, because raster imagery needs a realization it does not have yet. On those platforms the map's look comes from the MapLibre style you supply -- see [How your map is realized](#how-your-map-is-realized).

---

## User location

Four builders touch the location marker, and they differ in who supplies the coordinates:

| Method | Effect |
|---|---|
| `.shows_user_location(true)` | Turns the marker on and lets the platform's own location service feed it |
| `.user_location(signal)` | Turns the marker on and draws `Location` values from your signal |
| `.optional_user_location(signal)` | Same, but `None` draws no marker -- the state to use while a permission prompt is pending |
| `.follows_location(signal)` | Marker on, plus the camera re-centers on every new value |

```rust,ignore
use waterui::map::{Location, Map, Region};
use waterui::{Binding, View};

fn tracking_map(location: Binding<Option<Location>>, region: Binding<Region>) -> impl View {
    Map::new(region).optional_user_location(location)
}
```

> **The signal is not optional off Apple.** `shows_user_location(true)` alone leaves the location signal empty. MapKit fills that in from CoreLocation; the GPU realization has no platform location service to fall back on and simply draws nothing. Feed it `user_location` or `optional_user_location` if you want the marker everywhere.

Supplying the signal yourself is also what keeps camera following, the marker, and the horizontal-accuracy circle driven by one source instead of drifting apart.

---

## Interaction and chrome

```rust,ignore
use waterui::map::{Map, Region};
use waterui::View;

fn thumbnail(region: Region) -> impl View {
    Map::new(region)
        .is_interactive(false) // no pan, no zoom -- good for a list cell
        .shows_compass(false)
        .shows_scale(false)
}
```

All three default to on. `is_interactive(false)` is respected everywhere: the GPU realization skips installing its drag and magnification gestures entirely. The compass and scale bar are MapKit chrome; the GPU realization draws neither, so treat them as an Apple refinement rather than a guarantee.

---

## Reading the device location

`Location::get()` does **not** prompt. It assumes the permission is already granted, which means you ask first through `waterkit-permission` (a direct dependency -- `waterui` does not re-export it):

```rust,ignore
use waterkit_permission::{Permission, request};
use waterui::map::Location;
use waterui::map::location::{LocationError, PermissionStatus};

async fn current_location() -> Result<Option<Location>, LocationError> {
    match request(Permission::Location).await {
        Ok(PermissionStatus::Granted) => Location::get().await.map(Some),
        Ok(status) => {
            tracing::warn!("location permission: {status:?}");
            Ok(None)
        }
        Err(error) => {
            tracing::error!("permission request failed: {error}");
            Ok(None)
        }
    }
}
```

`request` returns `Granted`, `Denied`, `Restricted`, or `NotDetermined` -- on Android the last one persists until the host Activity applies the callback result, so treat "not granted" as a state to render, not an error to swallow.

A `Location` exposes its data through accessors. `latitude()` and `longitude()` return the `Latitude`/`Longitude` newtypes; call `.get()` for the underlying `f64`:

| Accessor | Type |
|---|---|
| `latitude()` / `longitude()` | `Latitude` / `Longitude` |
| `altitude()` | `Option<f64>` meters above sea level |
| `horizontal_accuracy()` / `vertical_accuracy()` | `Option<f64>` meters |
| `timestamp()` | `Timestamp` |

`LocationError` is `#[non_exhaustive]` with `PermissionDenied`, `ServiceDisabled`, `Timeout`, `NotAvailable`, `InvalidCoordinate(OutOfRange)`, and `Platform(String)`.

---

## How your map is realized

You do not pick a map backend. `Map` is a semantic view, and whichever backend you build against realizes it:

- **Apple platforms** bridge `MKMapView` from MapKit. Styles, compass, scale, and the CoreLocation-driven blue dot all come from the system.
- **Self-drawn backends** (Hydrolysis and friends) install a GPU vector-map realization that fetches a MapLibre style and vector tiles and draws them with the same GPU pipeline as the rest of your UI. The backend installs it during bootstrap only when no native map hook is present, so app code neither imports it nor chooses it.

There is one seam you must handle: WaterUI hosts no tile service, so the GPU realization has nowhere to fetch from until you name a provider. Add `waterui-map-gpu` as a dependency -- the `waterui` facade does not re-export it -- and insert `MapGpuOptions` into the app environment with a MapLibre style URL:

```rust,ignore
use waterui::app::App;
use waterui::env::Environment;
use waterui::Url;
use waterui_map_gpu::MapGpuOptions;

pub fn app(mut env: Environment) -> App {
    env.insert(MapGpuOptions::new(Url::new(
        "https://tiles.openfreemap.org/styles/positron",
    )));
    App::new(root_view, env)
}
```

Realizing a GPU map without `MapGpuOptions` in the environment panics -- fast failure by design, not a blank tile grid you have to debug. Insert it whenever your app targets a platform without a native map; on Apple the value is simply unused.

Beyond the style URL, `MapGpuOptions` is a builder over the resources the realization is allowed to consume: `maximum_style_bytes`, `maximum_tilejson_bytes`, `maximum_tile_bytes`, `tile_cache_bytes`, `maximum_in_flight_tile_requests`, `request_timeout`, `network_retry_policy`, and `camera_animation`. Defaults cover an ordinary app; reach for them when you are on a metered connection or a tight memory budget.

```rust,ignore
use std::num::NonZeroU64;
use std::time::Duration;
use waterui::Url;
use waterui_map_gpu::MapGpuOptions;

let options = MapGpuOptions::new(Url::new("https://tiles.openfreemap.org/styles/positron"))
    .tile_cache_bytes(NonZeroU64::new(32 * 1024 * 1024).expect("non-zero"))
    .request_timeout(Duration::from_secs(10));
```

`MapNetworkRetryPolicy::new(attempts, initial_delay, maximum_delay)` builds the backoff policy; the default is four attempts starting at 250 ms and capped at 4 s.

---

## Putting it together

A map centered on Manhattan, with a button that requests permission and then moves the camera to the user:

```rust,ignore
use waterkit_permission::{Permission, request};
use waterui::map::location::PermissionStatus;
use waterui::map::{Coordinate, Location, Map, MapStyle, Region};
use waterui::prelude::*;
use waterui::reactive::binding;

fn located_map() -> impl View {
    let region = binding(Region::new(
        Coordinate::from_degrees(40.7580, -73.9855).expect("valid coordinate"),
        0.030,
        0.050,
    ));
    let user_location: Binding<Option<Location>> = binding(None);
    let status = binding(Str::from("Location not requested"));

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
                            tracing::error!("location request failed: {error}");
                            status.set("Location unavailable".into());
                        }
                    },
                    Ok(other) => status.set(format!("Permission: {other:?}").into()),
                    Err(error) => {
                        tracing::error!("permission request failed: {error}");
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

Next up: [WebView](03-webview.md), where you embed web content in your app -- JavaScript bridges, cookie management, and navigation controls included.

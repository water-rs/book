# Maps and Location

> **In this chapter, you will:**
> - Embed interactive maps with annotations and styles
> - Track and display the user's real-time location
> - Build a complete location-aware feature with reactive state
> - Understand cross-platform differences in map rendering

Picture a ride-sharing app that follows your car in real time, or a travel guide that drops pins on every restaurant nearby. Maps are one of those features that instantly make an app feel polished and professional. WaterUI gives you native map rendering through the `waterui-map` crate and cross-platform location access via `waterkit-location`, all with a declarative, reactive API.

> **Feature flag:** Maps live behind the `map` feature on `waterui`. Enable it in `Cargo.toml` (`waterui = { version = "...", features = ["map"] }`) so the prelude re-export `waterui::map` is available.

## Crate Overview

| Crate | Purpose |
|---|---|
| `waterui-map` | The `Map` view component, `Coordinate`, `Region`, `Annotation`, map styles |
| `waterkit-location` | Cross-platform `Location::get()`, permission handling, coordinate data |

The map crate re-exports the location crate for convenience:

```rust,ignore
use waterui::map::location; // waterkit-location
use waterui::map::Location; // waterkit_location::Location
```

---

## Coordinates and Regions

Before you can display a map, you need to tell it *where* to look. That starts with two types: `Coordinate` and `Region`.

### `Coordinate`

A geographic point on the globe:

```rust,ignore
use waterui::map::Coordinate;

let san_francisco = Coordinate::new(37.7749, -122.4194);
let tokyo = Coordinate::new(35.6762, 139.6503);
```

`Coordinate` has two fields:

| Field | Type | Range |
|---|---|---|
| `latitude` | `f64` | -90.0 to 90.0 |
| `longitude` | `f64` | -180.0 to 180.0 |

You can convert from a `waterkit_location::Location` directly:

```rust,ignore
use waterui::map::Coordinate;
use waterkit_location::Location;

// From a Location reference
let coord = Coordinate::from_location(&location);

// Or via Into
let coord: Coordinate = location.into();
```

### `Region`

A `Region` describes the visible area of the map -- a center coordinate plus
a span in degrees:

```rust,ignore
use waterui::map::{Coordinate, Region};

// Explicit span
let bay_area = Region::new(
    Coordinate::new(37.7749, -122.4194),
    0.5,  // latitude span in degrees
    0.5,  // longitude span in degrees
);

// Default zoom from a coordinate
let zoomed_in = Region::from_coordinate(Coordinate::new(37.7749, -122.4194));
// Uses 0.05 degree span in both directions
```

| Field | Description |
|---|---|
| `center` | The `Coordinate` at the center of the visible region |
| `latitude_delta` | North-to-south span in degrees (smaller = more zoomed in) |
| `longitude_delta` | East-to-west span in degrees |

`Region` implements `From<Coordinate>`, so you can pass a bare coordinate
anywhere a region is expected and get a sensible default zoom.

---

## Displaying a Map

Now let's put a map on screen. You will find it is just as straightforward as placing any other view.

### Basic Map

```rust,ignore
use waterui::map::{Map, Coordinate, Region};

fn city_map() -> impl View {
    let region = Region::new(
        Coordinate::new(48.8566, 2.3522), // Paris
        0.1,
        0.1,
    );
    Map::new(region)
}
```

`Map::new` accepts any `Into<Computed<Region>>`, meaning you can pass:
- A static `Region` value
- A reactive `Computed<Region>` signal that updates the visible area over time

### Centering on a Coordinate

If you only have a coordinate and want the default zoom:

```rust,ignore
use waterui::map::{Map, Coordinate};

fn pin_map() -> impl View {
    Map::centered_on(Coordinate::new(35.6762, 139.6503))
}
```

`Map::centered_on` also accepts `Computed<Coordinate>` for reactive updates.

### Centering on a `Location`

When working directly with the `waterkit-location` crate:

```rust,ignore
use waterui::map::Map;
use waterkit_location::Location;

fn location_map(location: Computed<Location>) -> impl View {
    Map::centered_on_location(location)
}
```

---

## Annotations (Map Markers)

A map without markers is just a pretty picture. Add pins with `Annotation`:

```rust,ignore
use waterui::map::{Map, Coordinate, Region, Annotation};

fn annotated_map() -> impl View {
    let sf = Coordinate::new(37.7749, -122.4194);
    let la = Coordinate::new(34.0522, -118.2437);

    Map::new(Region::new(Coordinate::new(36.0, -120.0), 5.0, 5.0))
        .annotations(vec![
            Annotation::new(sf, "San Francisco"),
            Annotation::new(la, "Los Angeles")
                .subtitle("City of Angels"),
        ])
}
```

### `Annotation` API

| Method | Description |
|---|---|
| `Annotation::new(coordinate, title)` | Create with a position and title |
| `.subtitle(text)` | Add optional subtitle text |

Each annotation has these fields:

| Field | Type | Description |
|---|---|---|
| `coordinate` | `Coordinate` | Where the pin is placed |
| `title` | `Str` | Primary label shown on the annotation |
| `subtitle` | `Option<Str>` | Secondary label (optional) |

### Reactive Annotations

Since `annotations` accepts `Into<Computed<Vec<Annotation>>>`, you can drive
the marker list from a reactive signal. This is perfect for search results, live tracking, or any data that changes over time:

```rust,ignore
use waterui::prelude::*;
use waterui::map::{Map, Coordinate, Region, Annotation};

fn dynamic_markers() -> impl View {
    let markers = Binding::container(vec![
        Annotation::new(Coordinate::new(37.7749, -122.4194), "Start"),
    ]);

    Map::new(Region::default())
        .annotations(markers.into_computed())
}
```

---

## Map Styles

Choose between three display modes to match the feel of your app:

```rust,ignore
use waterui::map::{Map, Region, MapStyle};

fn satellite_view() -> impl View {
    Map::new(Region::default())
        .style(MapStyle::Satellite)
}
```

| Style | Description |
|---|---|
| `MapStyle::Standard` | Road map with labels (default) |
| `MapStyle::Satellite` | Satellite imagery |
| `MapStyle::Hybrid` | Satellite imagery with road overlays |

---

## User Location

### Showing the User's Position

Display the familiar blue dot indicating the user's current location:

```rust,ignore
use waterui::map::{Map, Region};

fn location_enabled_map() -> impl View {
    Map::new(Region::default())
        .shows_user_location(true)
}
```

> **Note:** This requires location permission. Use `Location::ask_permission()`
> or let the platform prompt the user automatically.

### Following the User

`follows_location` both centers the map on a reactive `Location` stream and
enables the user-location indicator. The map moves as the user moves -- great for navigation or fitness tracking:

```rust,ignore
use waterui::map::Map;
use waterkit_location::Location;

fn tracking_map(location: Computed<Location>) -> impl View {
    Map::new(Region::default())
        .follows_location(location)
}
```

This is equivalent to calling `shows_user_location(true)` plus binding the
region to the location signal.

---

## Map Interaction Controls

Fine-tune the map's interactive behavior. For example, you might want a non-interactive overview map in a list cell:

```rust,ignore
use waterui::map::{Map, Region};

fn static_overview() -> impl View {
    Map::new(Region::default())
        .is_interactive(false)  // Disable pan and zoom
        .shows_compass(false)   // Hide the compass
        .shows_scale(true)      // Show the scale bar
}
```

| Method | Default | Description |
|---|---|---|
| `is_interactive(bool)` | `true` | Enable or disable pan/zoom gestures |
| `shows_compass(bool)` | `true` | Show the compass indicator |
| `shows_scale(bool)` | `true` | Show the distance scale bar |

---

## Getting the User's Location

The `waterkit-location` crate provides a cross-platform API for accessing device location:

```rust,ignore
use waterkit_location::{Location, LocationError};

async fn where_am_i() -> Result<(), LocationError> {
    let location = Location::get().await?;

    tracing::info!("Lat: {}", location.latitude());
    tracing::info!("Lon: {}", location.longitude());

    if let Some(alt) = location.altitude() {
        tracing::info!("Altitude: {} meters", alt);
    }

    tracing::info!("Accuracy: {:?} meters", location.horizontal_accuracy());
    tracing::info!("Time: {:?}", location.timestamp());

    Ok(())
}
```

### `Location` Accessors

| Method | Return Type | Description |
|---|---|---|
| `latitude()` | `f64` | Latitude in degrees |
| `longitude()` | `f64` | Longitude in degrees |
| `altitude()` | `Option<f64>` | Altitude in meters above sea level |
| `horizontal_accuracy()` | `Option<f64>` | Horizontal accuracy in meters |
| `vertical_accuracy()` | `Option<f64>` | Vertical accuracy in meters |
| `timestamp()` | `Timestamp` | When the location was recorded |

### Error Handling

`Location::get()` returns `Result<Location, LocationError>`:

| Error | Description |
|---|---|
| `PermissionDenied` | The user denied location access |
| `ServiceDisabled` | Location services are turned off on the device |
| `Timeout` | The location request timed out |
| `NotAvailable` | Location data could not be determined |
| `Unknown(String)` | An unexpected platform error |

### Permissions

`Location::get()` calls `Location::ask_permission()` automatically. If you
want to check permission status before presenting the map, call it explicitly:

```rust,ignore
use waterkit_location::Location;

async fn ensure_location_access() {
    if let Err(e) = Location::ask_permission().await {
        tracing::warn!("Location permission not granted: {e}");
    }
}
```

---

## Convenience Function

A free function `map()` is available as a shorthand for `Map::new()`:

```rust,ignore
use waterui::map::{map, Region};

fn quick_map() -> impl View {
    map(Region::default())
}
```

---

## Complete Example

Here is a complete example that fetches the user's location and displays a
map centered on it with an annotation:

```rust,ignore
use waterui::prelude::*;
use waterui::map::{Annotation, Coordinate, Map, MapStyle, Region};
use waterkit_location::Location;

fn location_app() -> impl View {
    let location_binding: Binding<Option<Location>> = Binding::container(None);

    // Reactive map region and annotations derived from the same source signal.
    let region = location_binding.map(|opt| {
        opt.map(|loc| Region::from_coordinate(Coordinate::from(loc)))
            .unwrap_or_default()
    });

    let annotations = location_binding.map(|opt| {
        opt.map(|loc| vec![
            Annotation::new(Coordinate::from(loc), "You are here"),
        ])
        .unwrap_or_default()
    });

    // Fetch the location once when the map appears; the task is cancelled with the view.
    let loc = location_binding.clone();
    Map::new(region)
        .annotations(annotations)
        .style(MapStyle::Standard)
        .shows_user_location(true)
        .shows_compass(true)
        .task(async move {
            match Location::get().await {
                Ok(location) => loc.set(Some(location)),
                Err(e) => tracing::error!("Location error: {e}"),
            }
        })
}
```

---

## Platform Considerations

| Feature | Apple | Android | Desktop |
|---|---|---|---|
| Map rendering | MKMapView (MapKit) | Platform map view | WIP |
| Standard/Satellite/Hybrid | All supported | All supported | -- |
| User location dot | Native | Native | -- |
| Annotations | Native pins | Native markers | -- |
| Location access | CoreLocation | FusedLocationProvider | GeoClue (Linux), WinRT (Windows) |

The `Map` component uses the `configurable!` macro with `StretchAxis::Both`,
meaning it expands to fill available space in both directions by default. Use
layout modifiers like `.size(width, height)`, `.width(...)`, or `.height(...)`
to constrain its size when needed.

---

## What's Next

You have maps and location covered. Next up: [WebView](03-webview.md), where you will embed web content directly into your app -- complete with JavaScript bridges, cookie management, and navigation controls.

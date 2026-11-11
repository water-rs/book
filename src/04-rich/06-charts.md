# Charts

> **In this chapter, you will:**
> - Pick the right chart view for your data from the thirteen kinds `waterui-chart` ships
> - Feed a chart reactive data and watch it cross-fade between states
> - Add axes, a legend, and tooltips around a chart
> - Read which datum is focused or selected, and scroll a cartesian chart along its x domain
> - Know where `ChartAnimation` fits and what the crate animates for you

Charts are drawn views, like [barcodes](04-barcode.md): WaterUI does not bridge a platform chart framework, and draws every chart itself through the same canvas pipeline on every backend. Each chart is a view whose first argument is a *signal of its data*; change the signal and the chart redraws, cross-fading from the old state to the new one.

> **Dependencies and feature flag.** Charts live in their own crate, `water-rs/chart`. The facade exposes it as `waterui::chart` behind the `chart` feature (`waterui = { version = "...", features = ["chart"] }`). A direct dependency on `waterui-chart` exposes the same types. Every example below imports from `waterui::chart`.

## Quick start

```rust
use waterui::chart::{AxisConfig, ChartExt, DataBounds, DataPoint, LineChart};
use waterui::color::Srgb;
use waterui::prelude::*;

fn revenue() -> impl View {
    let points = vec![
        DataPoint::new(0.0, 10.0),
        DataPoint::new(1.0, 45.0),
        DataPoint::new(2.0, 30.0),
        DataPoint::new(3.0, 70.0),
        DataPoint::new(4.0, 55.0),
    ];
    let bounds = DataBounds::from_points(&points);
    let data: Binding<Vec<DataPoint>> = binding(points);

    LineChart::new(data)
        .color(Srgb::from_hex("#22C55E"))
        .line_width(3.0)
        .axes(bounds)
        .y_axis(AxisConfig::new().tick_count(5).show_grid().label("Revenue"))
        .x_axis(AxisConfig::new().label("Quarter"))
        .size(350.0, 280.0)
}
```

Three things to notice:

- `LineChart::new(data)` takes a `Signal` whose output is `Vec<DataPoint>`. A `Binding<Vec<DataPoint>>` is one; so is a plain `Vec` of the point types, which is a constant signal.
- Chart colors are `Srgb`, not `Color`. The data types that carry their own color (`RadarSeries`, `AreaSeries`, `BubblePoint`, `GaugeRegion`) store it as four normalized `f32` components: you pass `(r, g, b, a)` or a hex string (`color_hex`, `GaugeRegion::hex`).
- `.axes(bounds)` wraps the chart in a `ChartAxes`. Every other chart modifier (`color`, `line_width`, `focused_tooltip`) comes **before** it, because `ChartAxes` is a plain view and no longer has the chart's methods.

Each chart also has a free-function constructor, `line_chart(data)`, `bar_chart(data)`, and so on. They are equivalent to `Type::new`.

## The chart kinds

Every kind is a struct `XxxChart<S: Signal<Output = ...>>` plus a free function `xxx_chart`. The data type each one reads is the part to remember:

| View | Constructor | Signal output | Family |
|---|---|---|---|
| `LineChart` | `line_chart` | `Vec<DataPoint>` | cartesian |
| `BarChart` | `bar_chart` | `Vec<DataPoint>` | cartesian |
| `ScatterChart` | `scatter_chart` | `Vec<DataPoint>` | cartesian |
| `BubbleChart` | `bubble_chart` | `Vec<BubblePoint>` | cartesian |
| `AreaChart` | `area_chart` | `AreaData` | cartesian |
| `PieChart` | `pie_chart` | `Vec<DataPoint>` | polar |
| `RadarChart` | `radar_chart` | `RadarData` | polar |
| `GaugeChart` | `gauge_chart` | `GaugeData` | polar |
| `HeatmapChart` | `heatmap_chart` | `HeatmapData` | grid |
| `ContourChart` | `contour_chart` | `ContourData` | grid |
| `CandlestickChart` | `candlestick_chart` | `Vec<Candle>` | financial |
| `DepthChart` | `depth_chart` | `DepthData` | financial |
| `ChoroplethChart` | `choropleth_chart` | `ChoroplethData` | geographic |

Every data type is plain data with public fields. To change a chart, either `set` a new value on its `Binding` or edit the current one in place with `Binding::with_mut`. That works for the `Vec` kinds and for the composite structs (`AreaData`, `RadarData`, `GaugeData`, `HeatmapData`, `ContourData`, `DepthData`, `ChoroplethData`) alike.

Charts have no intrinsic size. Give each one a frame with `.size(width, height)` from `ViewExt`, as the examples do, or place it in a container that proposes one. See [the layout chapter](../03-ui/02-layout.md).

### Cartesian charts: line, bar, scatter, bubble, area

These plot against x and y axes, so they are the kinds you usually wrap with `.axes(..)`. Together with the two financial charts, they also support x-domain selection and scrolling (below).

```rust
use waterui::chart::{
    AxisConfig, BarChart, BubbleChart, BubblePoint, ChartExt, DataBounds, DataPoint, ScatterChart,
};
use waterui::color::Srgb;
use waterui::prelude::*;

fn sales_bars() -> impl View {
    let points = vec![
        DataPoint::new(0.0, 65.0),
        DataPoint::new(1.0, 85.0),
        DataPoint::new(2.0, 45.0),
        DataPoint::new(3.0, 95.0),
    ];
    let bounds = DataBounds::from_points(&points);
    let data: Binding<Vec<DataPoint>> = binding(points);

    BarChart::new(data)
        .color(Srgb::from_hex("#3B82F6"))
        .axes(bounds)
        .y_axis(AxisConfig::new().show_grid())
        .size(350.0, 280.0)
}

fn measurements() -> impl View {
    let points = vec![
        DataPoint::new(1.0, 2.5),
        DataPoint::new(2.5, 3.0),
        DataPoint::new(4.0, 4.5),
        DataPoint::new(6.0, 7.0),
    ];
    let bounds = DataBounds::from_points(&points);
    let data: Binding<Vec<DataPoint>> = binding(points);

    ScatterChart::new(data)
        .color(Srgb::from_hex("#EF4444"))
        .radius(6.0)
        .axes(bounds)
        .size(350.0, 280.0)
}

fn markets() -> impl View {
    let bubbles: Binding<Vec<BubblePoint>> = binding(vec![
        BubblePoint::with_color(14.0, 1.4, 100.0, 0.94, 0.27, 0.27, 0.7),
        BubblePoint::with_color(21.0, 0.33, 80.0, 0.23, 0.51, 0.96, 0.7),
        BubblePoint::new(3.0, 1.38, 50.0),
    ]);

    BubbleChart::new(bubbles)
        .min_radius(10.0)
        .max_radius(50.0)
        .opacity(0.7)
        .size(400.0, 350.0)
}
```

What each kind adds on top of `color`:

- `LineChart`: `line_width(w)` and `fill(opacity)` for an area under the line.
- `ScatterChart`: `radius(r)`.
- `BubbleChart`: `BubblePoint::new(x, y, size)` uses the chart's `color`, and `BubblePoint::with_color(x, y, size, r, g, b, a)` sets the point's own. `min_radius` and `max_radius` set the pixel range that `size` maps onto, and `opacity` fades every bubble.
- `BarChart`: `color` only; bars sit at each point's x.

`AreaChart` reads an `AreaData`: one shared list of x values, then one `AreaSeries` per band. The bands stack by default.

```rust
use waterui::chart::{AreaChart, AreaData, AreaSeries};
use waterui::prelude::*;

fn revenue_streams() -> impl View {
    let data = AreaData::new(vec![1.0, 2.0, 3.0, 4.0])
        .series(AreaSeries::new("Products", vec![30.0, 45.0, 35.0, 50.0]).color(0.23, 0.51, 0.96, 0.7))
        .series(AreaSeries::new("Services", vec![20.0, 25.0, 30.0, 35.0]).color_hex("#22C55E"));
    let areas: Binding<AreaData> = binding(data);

    AreaChart::new(areas).size(400.0, 350.0)
}
```

Every series in an `AreaData` must have one value per x. Call `.stacked(false)` to overlay the bands instead of stacking them. `AreaData::bounds()` gives you the `DataBounds` for `.axes(..)`, and it accounts for stacking. `color_hex` gives the band an alpha of 0.7; use `color(r, g, b, a)` to choose another one.

### Polar charts: pie, radar, gauge

Polar charts have no axes, so you size them directly.

```rust
use waterui::chart::{
    ArcAngles, DataPoint, GaugeChart, GaugeData, GaugeRadii, GaugeRegion, PieChart, RadarChart,
    RadarData, RadarSeries,
};
use waterui::color::Srgb;
use waterui::prelude::*;

fn share() -> impl View {
    let slices: Binding<Vec<DataPoint>> = binding(vec![
        DataPoint::new(0.0, 35.0),
        DataPoint::new(1.0, 25.0),
        DataPoint::new(2.0, 20.0),
        DataPoint::new(3.0, 15.0),
    ]);

    // `donut` takes the inner radius as a fraction of the outer one.
    PieChart::new(slices).donut(0.5).size(300.0, 300.0)
}

fn skills() -> impl View {
    let data = RadarData::new(5)
        .labels(vec!["Speed", "Power", "Range", "Defense", "Luck"])
        .series(RadarSeries::new("Warrior", vec![70.0, 95.0, 40.0, 90.0, 50.0]).color_hex("#3B82F6"))
        .series(RadarSeries::new("Mage", vec![50.0, 30.0, 80.0, 40.0, 60.0]).color_hex("#9333EA"))
        .max_value(100.0);
    let radar: Binding<RadarData> = binding(data);

    RadarChart::new(radar).ring_count(5).fill_opacity(0.3).size(350.0, 350.0)
}

fn load() -> impl View {
    let data = GaugeData::new(72.0, 0.0, 100.0)
        .region(GaugeRegion::hex(30.0, "#22C55E"))
        .region(GaugeRegion::hex(70.0, "#EAB308"))
        .region(GaugeRegion::hex(100.0, "#EF4444"));
    let gauge: Binding<GaugeData> = binding(data);

    GaugeChart::new(gauge)
        .arc(ArcAngles::from_degrees(-120.0, 120.0))
        .radii(GaugeRadii::new(0.35, 0.45))
        .needle_color(Srgb::WHITE)
        .size(300.0, 300.0)
}
```

- `PieChart`: each slice's size comes from its point's `y`, and negative values count as zero. `colors(Vec<Srgb>)` sets the palette. `donut` accepts `0.0..=0.95`, and `full_pie()` undoes it.
- `RadarChart`: `RadarData::new(axis_count)` needs at least three axes and panics on fewer. Give each `RadarSeries` one value per axis, and give `labels` one entry per axis if you set them. A series raises `max_value` to its largest value, so a `max_value` set after the series replaces that. `ring_count` (at least 1), `line_width`, and `fill_opacity` style the web.
- `GaugeChart`: a `GaugeRegion` colors the zone up to its threshold. `GaugeData::region` keeps regions sorted by threshold, so you can add them in any order. `GaugeData::new(value, min, max)` panics unless `max > min` and all three are finite. The `GaugeRadii::new(inner, outer)` values are fractions of the chart's shorter side and must satisfy `0.0 <= inner < outer <= 0.5`. The default arc runs from -135° to 135°.

### Grid charts: heatmap and contour

Both read a row-major grid of `f32` values.

```rust
use waterui::chart::{ContourChart, ContourData, HeatmapChart, HeatmapData};
use waterui::prelude::*;

fn correlation() -> impl View {
    let size = 8_u32;
    let mut values = Vec::new();
    for row in 0..size {
        for col in 0..size {
            let distance = row.abs_diff(col) as f32 / size as f32;
            values.push(1.0 - distance);
        }
    }
    let heat: Binding<HeatmapData> = binding(HeatmapData::new(size, size, values));

    HeatmapChart::new(heat).size(300.0, 300.0)
}

fn terrain() -> impl View {
    let size = 40_u32;
    let mut values = Vec::new();
    for row in 0..size {
        for col in 0..size {
            let x = col as f32 / size as f32 * 4.0 - 2.0;
            let y = row as f32 / size as f32 * 4.0 - 2.0;
            values.push((-(x * x + y * y)).exp());
        }
    }
    // Eight evenly spaced contour levels between the grid's min and max.
    let contour: Binding<ContourData> = binding(ContourData::new(size, size, values, 8));

    ContourChart::new(contour).line_width(2.0).size(350.0, 350.0)
}
```

`ContourData::with_levels(rows, cols, values, levels)` takes explicit level values instead. `HeatmapData::from_matrix(&[&[f32]])` builds a grid from nested slices.

### Financial charts: candlestick and depth

```rust
use waterui::chart::{
    Candle, CandlestickChart, DepthChart, DepthData, DepthLevel,
};
use waterui::color::Srgb;
use waterui::prelude::*;

fn price_history() -> impl View {
    let candles: Binding<Vec<Candle>> = binding(vec![
        // Candle::new(timestamp, open, high, low, close, volume)
        Candle::new(0.0, 100.0, 110.0, 95.0, 105.0, 1000.0),
        Candle::new(1.0, 105.0, 115.0, 100.0, 98.0, 1200.0),
        Candle::new(2.0, 98.0, 108.0, 92.0, 106.0, 800.0),
    ]);

    CandlestickChart::new(candles)
        .bullish_color(Srgb::from_hex("#22C55E"))
        .bearish_color(Srgb::from_hex("#EF4444"))
        .size(350.0, 280.0)
}

fn order_book() -> impl View {
    let book = DepthData::new(
        vec![DepthLevel::new(100.0, 500.0), DepthLevel::new(99.0, 2000.0)],
        vec![DepthLevel::new(100.5, 400.0), DepthLevel::new(101.5, 1800.0)],
    );
    let depth: Binding<DepthData> = binding(book);

    DepthChart::new(depth)
        .colors(Srgb::from_hex("#22C55E"), Srgb::from_hex("#EF4444"))
        .size(350.0, 280.0)
}
```

`Candle::timestamp` is an `f64`, so Unix seconds fit without precision loss. When the chart computes x positions it rebases them on the first candle (`candle_epoch`) before narrowing to `f32`. If you wrap a candlestick chart in `.axes(..)`, build the bounds with `DataBounds::from_candles(&candles)` so they use the same rebasing. The x tick labels then count seconds from the first candle; use a `TickFormat::Custom` formatter (see [Axes](#axes)) to show dates.

`DepthLevel::new(price, cumulative_volume)` takes the cumulative volume at that price, not the volume of one order. List bids from the highest price down and asks from the lowest price up, as an order book does.

### Geographic charts: choropleth

`ChoroplethChart` colors polygons by value. It draws the polygons you give it in plain x/y space; it is not a map and loads no tiles. For a real map, see [Maps and location](02-maps.md).

```rust
use waterui::chart::{ChoroplethChart, ChoroplethData, ColorScale, GeoPolygon};
use waterui::color::Srgb;
use waterui::prelude::*;

fn regions() -> impl View {
    let data = ChoroplethData::new(vec![
        GeoPolygon::new(1, 12.0, vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]),
        GeoPolygon::new(2, 48.0, vec![[1.0, 0.0], [2.0, 0.0], [2.0, 1.0], [1.0, 1.0]]),
    ])
    .color_scale(ColorScale::sequential(
        Srgb::from_hex("#DBEAFE"),
        Srgb::from_hex("#1E3A8A"),
    ));
    let map: Binding<ChoroplethData> = binding(data);

    ChoroplethChart::new(map)
        .stroke_width(1.5)
        .stroke_color(Srgb::WHITE)
        .size(300.0, 200.0)
}
```

`GeoPolygon::new` triangulates by a fan from the first vertex, which is correct for convex polygons. For concave outlines, tessellate yourself and pass `GeoPolygon::with_indices(id, value, vertices, indices)`. `ColorScale` also has `diverging`, `viridis` (the default), and `heat`; `ChoroplethData::range(min, max)` pins the value range that maps onto the scale.

## The data model

Everything a chart draws is plain data in `waterui::chart`:

| Type | Shape |
|---|---|
| `DataPoint` | `DataPoint::new(x, y)`; the point of line, bar, scatter, and pie |
| `BubblePoint` | `new(x, y, size)` or `with_color(x, y, size, r, g, b, a)` |
| `Candle` | `new(timestamp, open, high, low, close, volume)` |
| `DepthLevel` / `DepthData` | `DepthLevel::new(price, cumulative_volume)`, `DepthData::new(bids, asks)` |
| `HeatmapData` | `new(rows, cols, values)`, `from_matrix(..)`, `get(row, col)` |
| `ContourData` | `new(rows, cols, values, num_levels)`, `with_levels(..)` |
| `AreaData` / `AreaSeries` | `AreaData::new(x_values).series(..).stacked(bool)` |
| `RadarData` / `RadarSeries` | `RadarData::new(axes).labels(..).series(..).max_value(..)` |
| `GaugeData` / `GaugeRegion` | `GaugeData::new(value, min, max).region(..).show_needle(..)` |
| `ChoroplethData` / `GeoPolygon` | see above |
| `DataBounds` | `new(min_x, max_x, min_y, max_y)`; also `from_points`, `from_candles`, and `with_padding(ratio)` |

The crate also has wrappers around the `Vec` kinds: `BarData`, `LineData`, `PieData`, `ScatterData`, `CandlestickData`, and `BubbleData`, each with `From<Vec<_>>`. No chart view takes them. They are useful for their helpers: `bounds()` on all except `PieData`, which has `total()` instead. For example, `BubbleData::from(points).bounds()` gives the bounds for a bubble chart's axes. `DepthData` has `bounds()` and `mid_price()` of its own.

### Constructors that panic, and their fallible twins

Constructors that take a grid or a dimension check their input and panic when it is malformed, which is right for data you wrote in source. When the data comes from a file or a server, use the `try_` variant and handle `ChartDataError`:

```rust
use waterui::chart::data::ChartDataError;
use waterui::chart::{HeatmapChart, HeatmapData};
use waterui::prelude::*;

fn loaded_grid(rows: u32, cols: u32, values: Vec<f32>) -> Result<impl View, ChartDataError> {
    let data = HeatmapData::try_new(rows, cols, values)?;
    let heat: Binding<HeatmapData> = binding(data);
    Ok(HeatmapChart::new(heat).size(300.0, 300.0))
}
```

`HeatmapData::try_new` and `try_from_matrix`, `ContourData::try_new` and `try_with_levels`, and `RadarData::try_new` return `ChartDataError` (`ZeroDimension`, `RaggedMatrix`, `ValueCountMismatch`, `AxisCountTooSmall`, `DimensionTooLarge`). `ChartDataError` is not re-exported at the crate root, so import it from `waterui::chart::data`.

Style parameters follow the same pattern. Some builders accept either a raw `f32` or a validated newtype:

- `line_width`, `radius`, `min_radius`, `max_radius`, and `stroke_width` take `PositiveF32`.
- `fill`, `opacity`, and `fill_opacity` take `UnitInterval`.
- `donut` takes `DonutInnerRadius`.

A raw `f32` that is `NaN`, infinite, or out of range panics. Build the newtype with `try_new` to get a `ChartParamError` instead. The gauge's `arc` and `radii` take only `ArcAngles` and `GaugeRadii`. Their panicking constructors are `ArcAngles::from_degrees` / `from_radians` and `GaugeRadii::new`, and their fallible ones are `try_degrees` / `try_radians` and `GaugeRadii::try_new`. `RadarChart::try_ring_count` covers the ring count. `ChartParamError` is exported at `waterui::chart::ChartParamError`.

### Reactive data

Because the chart takes a signal, you update the data by updating a `Binding`. The chart cross-fades from the old state to the new one; see [Animation](#animation) below.

```rust
use waterui::chart::{AxisConfig, ChartExt, DataBounds, DataPoint, LineChart};
use waterui::prelude::*;

fn live_series() -> impl View {
    let data: Binding<Vec<DataPoint>> = binding(vec![DataPoint::new(0.0, 4.0)]);
    // Bounds accept any signal, so the axis labels follow the data.
    let bounds = data.map(|points| DataBounds::from_points(&points));
    let writer = data.clone();

    vstack((
        LineChart::new(data)
            .axes(bounds)
            .y_axis(AxisConfig::new().show_grid())
            .size(350.0, 240.0),
        button("Add sample").action(move || {
            writer.with_mut(|points| {
                let x = points.len() as f32;
                points.push(DataPoint::new(x, (x * 1.7).sin() * 20.0 + 30.0));
            });
        }),
    ))
}
```

Pass a signal for the bounds whenever the data can change; a constant `DataBounds` would leave the axis labels describing the first state. For the signal basics, see [Reactive state](../02-core/02-reactive.md).

## Axes

`ChartExt::axes(bounds)` is implemented for every view and returns a `ChartAxes`, which draws tick labels and optional grid lines around the chart. Both axes are on by default. Configure each with an `AxisConfig`:

```rust
use std::sync::Arc;

use waterui::chart::{AxisConfig, BarChart, ChartExt, DataBounds, DataPoint, TickFormat};
use waterui::prelude::*;

fn percentages() -> impl View {
    let points = vec![DataPoint::new(0.0, 0.2), DataPoint::new(1.0, 0.65), DataPoint::new(2.0, 0.4)];
    let bounds = DataBounds::from_points(&points);
    let data: Binding<Vec<DataPoint>> = binding(points);

    BarChart::new(data)
        .axes(bounds)
        .y_axis(
            AxisConfig::new()
                .tick_count(5)
                .format(TickFormat::Percent)
                .show_grid()
                .label("Completion"),
        )
        .x_axis(
            AxisConfig::new().format(TickFormat::Custom(Arc::new(|value: f32| {
                Str::from(format!("W{}", value as i32 + 1))
            }))),
        )
        .size(350.0, 280.0)
}
```

| `AxisConfig` method | Effect |
|---|---|
| `tick_count(n)` | Approximate tick count; a "nice numbers" pass may adjust it |
| `format(TickFormat)` | `Auto` (default), `Fixed(decimals)`, `SI` (1K, 1M), `Percent` (value times 100), `Custom(Arc<dyn Fn(f32) -> Str + Send + Sync>)` |
| `show_grid()` | Grid lines at the ticks |
| `label(text)` | Axis title; takes anything `IntoText`, so it localizes like any other text |

`ChartAxes` itself has:

- `no_x_axis()` and `no_y_axis()` to drop an axis.
- `padding(left, bottom)` and `padding_all(left, bottom, top, right)` for the space the labels get. The defaults are 50, 30, 10, and 10 points.
- `plot_padding(fraction)`, default 0.1. It tells the axes how far inside its frame the chart plots its data, so the ticks line up. The cartesian, grid, and financial charts all inset by that fraction, so change it only when you wrap a view of your own.

## Legend

A `Legend` is a view that pins itself to a corner or edge of the space it is given, 8 points in. Layer it over a chart in a `zstack`:

```rust
use waterui::chart::{
    AreaChart, AreaData, AreaSeries, Legend, LegendItem, LegendOrientation, LegendPosition,
};
use waterui::color::Srgb;
use waterui::prelude::*;

fn sales_by_city() -> impl View {
    let data = AreaData::new(vec![1.0, 2.0, 3.0])
        .series(AreaSeries::new("北京", vec![30.0, 45.0, 35.0]).color_hex("#3B82F6"))
        .series(AreaSeries::new("東京", vec![20.0, 25.0, 30.0]).color_hex("#22C55E"))
        .series(AreaSeries::new("서울", vec![15.0, 20.0, 18.0]).color_hex("#9333EA"));
    let areas: Binding<AreaData> = binding(data);

    zstack((
        AreaChart::new(areas),
        Legend::new(vec![
            LegendItem::new("北京", Srgb::from_hex("#3B82F6")),
            LegendItem::new("東京", Srgb::from_hex("#22C55E")),
            LegendItem::new("서울", Srgb::from_hex("#9333EA")),
        ])
        .position(LegendPosition::TopLeft)
        .orientation(LegendOrientation::Horizontal),
    ))
    .size(400.0, 300.0)
}
```

`LegendItem::new(label, color)` takes any `IntoText` label and anything that converts to a `Color`. A chart does not generate its legend: you list the items, so labels and colors stay your decision. `LegendPosition` has `TopRight` (default), `TopLeft`, `BottomRight`, `BottomLeft`, `Top`, and `Bottom`; `LegendOrientation` is `Vertical` (default) or `Horizontal`; `marker_size` and `spacing` tune the entries.

## Tooltips and interaction

Every chart kind tracks two interaction states. **Focused** follows the pointer while it hovers or drags over a datum, and a tap focuses too. **Selected** is set by a tap or click, or where a drag ends. Both resolve to a `HitResult<T>`:

```rust,ignore
pub struct HitResult<T> {
    pub series: usize,       // series index for multi-series charts
    pub index: usize,        // datum index within the series
    pub value: T,            // the datum; its type depends on the chart
    pub anchor: ChartAnchor, // position in the chart's local coordinates, for placing readouts
}
```

`T` is the chart's datum type: `DataPoint` for line, bar, scatter; `BubblePoint`, `Candle`; and dedicated payloads for the rest: `AreaDatum`, `SliceDatum` (pie and gauge), `RadarDatum`, `GridDatum` (heatmap and contour), `DepthDatum` (with `DepthSide`), and `RegionDatum` (choropleth).

### Built-in tooltips

`focused_tooltip` and `selected_tooltip` take a closure from the hit to any view. The chart places the view a few points above the datum's anchor and clamps that position to the chart's frame:

```rust
use waterui::chart::{
    ChartExt, DataBounds, DataPoint, HitResult, LineChart, Tooltip, TooltipContent,
};
use waterui::prelude::*;

fn hover_readout() -> impl View {
    let points = vec![
        DataPoint::new(0.0, 12.0),
        DataPoint::new(1.0, 48.0),
        DataPoint::new(2.0, 31.0),
    ];
    let bounds = DataBounds::from_points(&points);
    let data: Binding<Vec<DataPoint>> = binding(points);

    LineChart::new(data)
        .focused_tooltip(|hit: HitResult<DataPoint>| {
            Tooltip::new(
                TooltipContent::new()
                    .title(format!("Sample {}", hit.index))
                    .value("x", format!("{:.1}", hit.value.x))
                    .value("y", format!("{:.1}", hit.value.y)),
            )
        })
        .axes(bounds)
        .size(350.0, 280.0)
}
```

`Tooltip` is just one view you can return. `TooltipContent` holds an optional title and value lines (`value(label, text)`, or `colored_value(label, text, color)` for a color swatch), and `Tooltip` has `background`, `text_color`, `corner_radius`, and `padding`. Return any other view from the closure to design the readout yourself.

### Reading the state

To act on the hover or tap yourself, pass a binding to `focused(&binding)` or `selected(&binding)`:

```rust
use waterui::chart::{DataPoint, HitResult, LineChart};
use waterui::prelude::*;

fn selection_label() -> impl View {
    let data: Binding<Vec<DataPoint>> = binding(vec![
        DataPoint::new(0.0, 12.0),
        DataPoint::new(1.0, 48.0),
        DataPoint::new(2.0, 31.0),
    ]);
    let selected: Binding<Option<HitResult<DataPoint>>> = binding(None);
    let readout: Computed<String> = selected
        .map(|hit| match hit {
            Some(hit) => format!("y = {:.1}", hit.value.y),
            None => String::from("Nothing selected"),
        })
        .computed();

    vstack((
        LineChart::new(data).selected(&selected).size(350.0, 240.0),
        text!("{readout}"),
    ))
}
```

For overlays that react to the hit, `chart_overlay` and `chart_background` pass a `ChartProxy<T>`. It exposes `focused()`, `selected()`, their `*_anchor()` positions, and the `chart_frame()` and `plot_area_frame()` viewports as signals, so a crosshair or highlight is an ordinary reactive view. Given a `DataBounds`, `ChartViewport` converts between view and data coordinates (`data_to_screen`, `screen_to_data`).

### X selection and scrolling (cartesian charts)

Line, bar, scatter, bubble, area, candlestick, and depth charts add x-domain controls. `chart_x_selection(&Binding<Option<f32>>)` and `chart_x_selection_range(&Binding<Option<RangeInclusive<f32>>>)`, and their `y` twins, track a selected value or a dragged range. One chart cannot track both a value and a range on the same axis; it panics if you ask for both. For a scrolling window, you supply the window length and a position binding together:

```rust
use waterui::chart::{
    AxisConfig, ChartExt, ChartScrollableAxes, DataBounds, DataPoint, LineChart,
};
use waterui::prelude::*;

const WINDOW: f32 = 40.0;

fn scrolling_series() -> impl View {
    let points: Vec<DataPoint> = (0..200)
        .map(|i| DataPoint::new(i as f32, (i as f32 / 8.0).sin() * 40.0 + 50.0))
        .collect();
    let full = DataBounds::from_points(&points);
    let data: Binding<Vec<DataPoint>> = binding(points);
    let position: Binding<f32> = Binding::f32(0.0);
    // The x axis follows the visible window, not the whole series.
    let bounds = position.map(move |start| {
        DataBounds::new(start, start + WINDOW, full.min_y, full.max_y)
    });

    LineChart::new(data)
        .chart_x_visible_domain(WINDOW)
        .chart_x_scroll_position(&position)
        .chart_scrollable_axes(ChartScrollableAxes::horizontal())
        .axes(bounds)
        .y_axis(AxisConfig::new().show_grid())
        .size(350.0, 240.0)
}
```

The position binding holds the x value at the window's leading edge. The chart clamps it to the data and writes it back as you drag. The three viewport settings depend on one another:

- `chart_x_visible_domain` requires `chart_x_scroll_position`, and the reverse.
- `chart_scrollable_axes` for an axis requires that axis's scroll position.
- The `y` versions behave the same way.

A missing partner panics when the chart's body is built, and the message names the missing call.

When you name a chart with `a11y_label`, the name lands on one accessibility element at the chart's bounds. The axis labels and composed layers (tooltips, `chart_overlay`, `chart_background`) do not repeat it. See [the accessibility chapter](../06-advanced/05-accessibility.md).

## Animation

Charts animate for you. When the signal's data changes, the chart keeps the previous frame and cross-fades to the new one over a fixed transition (240 ms, ease-in-out), then drops the old frame. The old frame fades out while the new one fades in; points do not move between positions. A change that only resizes or scrolls the chart redraws at once, without the fade. A chart has no builder to change the duration or easing, so the [reactive example](#reactive-data) that appends a sample already animates without any code.

The chart runs that transition on public types from `waterui::chart`, and you can drive them yourself:

- `EasingType`: `Linear`, `EaseIn`, `EaseOut`, `EaseInOut` (default), `Spring`. `apply(t)` maps progress in `0.0..=1.0` through the curve.
- `ChartAnimator`: a small state machine. `start_transition(now, duration, easing)` begins a run, and `start_entry` begins an entry animation. `update(now)` returns the current `ChartAnimation`, `is_animating()` reports whether a run is in progress, and `finish()` ends it. Time is a `Duration` you supply, so you own the clock.
- `ChartAnimation`: the plain `#[repr(C)]` value `update` returns. Its fields are `time` (seconds since the run started), `progress` (0 to 1, easing applied), `easing` (as `u32`), and `entry_active` (1 or 0).

`waterui::chart::animation::AnimationConfig` pairs a duration with an easing, with constructors such as `AnimationConfig::ease_in_out(duration)`.

```rust
use std::time::Duration;

use waterui::chart::{ChartAnimation, ChartAnimator, EasingType};

fn progress_halfway() -> ChartAnimation {
    let start = Duration::from_millis(0);
    let mut animator = ChartAnimator::new();
    animator.start_transition(start, Duration::from_millis(300), EasingType::EaseOut);
    animator.update(Duration::from_millis(150))
}
```

Reach for these only when you are building your own chart-like view on the same primitives. For an ordinary chart, change the signal and let it cross-fade.

## Where to go next

- [Maps and location](02-maps.md) for real geographic maps; a choropleth chart is data art over an x/y plane, not a map.
- [Barcodes and QR codes](04-barcode.md), another drawn component with the same sizing considerations.
- [Animation](../06-advanced/01-animation.md) for animating your own state around a chart.
- [Reactive state](../02-core/02-reactive.md) for the bindings and computed values every chart reads.

Part V starts with [Canvas](../05-graphics/01-canvas.md), the retained vector drawing view for anything the chart types do not cover.

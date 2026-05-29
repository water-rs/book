# Media: photos, video, and audio

> **In this chapter, you will:**
> - Display images from the network with progressive streaming
> - Play video with native controls or build your own player UI
> - Work with Live Photos and the unified `Media` enum
> - Let users pick media with the platform-native `MediaPicker`
> - Apply GPU-accelerated image filters

Every app eventually needs to show a photo, play a video, or let the user pick something from their library. WaterUI's media stack handles the hard parts for you: async fetching, progressive decoding, and GPU texture management.

## Feature flags and crate layout

The media types live behind cargo features in the `waterui` crate:

```toml
[dependencies]
waterui = { version = "*", features = ["media"] }
# Or for the raw video surface only:
# waterui = { version = "*", features = ["video"] }
```

`media` enables both `waterui-media` and `waterui-video`; the picker pulls in
platform dialogs through the `std` feature, which is on by default.

| Crate | Purpose |
|---|---|
| `waterui-media` | Photos, Live Photos, media picker, unified `Media` enum, GPU `Image` view |
| `waterui-video` | `Video` (raw) and `VideoPlayer` (with controls), aspect ratio, volume |

`waterui-media` re-exports the video types, so a single import covers most
apps:

```rust,ignore
use waterui::prelude::*;
use waterui::media::{Photo, Video, VideoPlayer, LivePhoto, Media};
```

---

## Displaying Images with `Photo`

`Photo` is the primary component for showing images from a URL. It fetches
the image asynchronously, decodes it through a streaming pipeline, and hands the
pixel data to the GPU-backed `Image` view.

### Basic Usage

```rust,ignore
use waterui::media::Photo;

fn avatar() -> impl View {
    Photo::new("https://static.rust-lang.org/logos/rust-logo-512x512.png")
}
```

`Photo::new` accepts anything that implements `Into<Url>`, including string
literals.

### Listening for Events

You can observe loading progress through the `on_event` callback. Because
`waterui::media` re-exports a video `Event` type, alias the photo event to
keep the two clearly separate:

```rust,ignore
use waterui::media::Photo;
use waterui::media::photo::Event as PhotoEvent;

fn profile_photo() -> impl View {
    Photo::new("https://static.rust-lang.org/logos/rust-logo-512x512.png")
        .on_event(|event| match event {
            PhotoEvent::Loaded => tracing::info!("Image loaded successfully"),
            PhotoEvent::Error(msg) => tracing::error!("Failed to load: {msg}"),
        })
}
```

`photo::Event` has two variants:

| Variant | Description |
|---|---|
| `Loaded` | The image finished loading and is being displayed. |
| `Error(String)` | The fetch or decode failed, with a human-readable message. |

### Reactive filters

Filter modifiers from `ViewExt` accept signals, so a `Binding` lets the user
adjust filter values in real time without rebuilding the photo:

```rust,ignore
use waterui::prelude::*;
use waterui::media::Photo;

fn blurry_photo() -> impl View {
    let blur = Binding::f64(0.0);
    let saturation = Binding::f64(1.0);

    vstack((
        Photo::new("https://static.rust-lang.org/logos/rust-logo-512x512.png")
            .blur(blur.clone())
            .saturation(saturation.clone()),
        Slider::new(&blur).range(0.0..=20.0),
        Slider::new(&saturation).range(0.0..=2.0),
    ))
}
```

See [Filters and Visual Effects](../05-graphics/04-filters.md) for the full
filter catalog.

### Streaming / Progressive Decoding

`Photo` uses an `ImageStreamDecoder` internally. As HTTP response chunks
arrive, the decoder attempts intermediate decodes at increasing byte thresholds
(starting at 24 KB, stepping by 96 KB). For formats that support progressive
rendering -- JPEG, PNG, GIF, WebP, BMP, ICO, TIFF -- you may see a
lower-quality preview appear before the final image lands. This is automatic
and requires no configuration.

> **Tip:** Progressive decoding is especially valuable on slower connections. Your users see *something* almost immediately, which makes the app feel faster even before the full image arrives.

### How `Image` Works Under the Hood

The `Image` struct holds raw pixel data (RGBA8 or RGBA16F) that gets uploaded
to a GPU texture on first render. After the texture is created, the CPU-side
pixel buffer is dropped, keeping memory usage lean.

```rust,ignore
use waterui::media::Image;

// Construct directly from pixel data (4 bytes per pixel, RGBA)
let pixels: Vec<u8> = vec![255, 0, 0, 255]; // 1x1 red pixel
let red_dot = Image::new(pixels, 1, 1);
```

For HDR content on Apple and Android platforms, WaterUI automatically selects
the platform image decoder and produces RGBA16F textures. When the output
surface does not support HDR, a tone-mapping shader converts the content to
SDR transparently.

---

## Video Playback

Whether you are building a media gallery, an onboarding flow with background video, or a full-featured player, WaterUI has you covered with two components:

| Component | Controls | Use Case |
|---|---|---|
| `Video` | None (raw surface) | Custom player UI, background videos, decorative clips |
| `VideoPlayer` | Native platform controls | Standard playback with play/pause, seek, fullscreen |

### `VideoPlayer` -- Full-Featured Playback

The quickest way to get video playing is `VideoPlayer`, which comes with platform-native controls out of the box:

```rust,ignore
use waterui::media::{AspectRatio, VideoPlayer};

fn trailer() -> impl View {
    VideoPlayer::new("https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4")
        .show_controls(true)
        .aspect_ratio(AspectRatio::Fit)
}
```

#### Configuration Methods

| Method | Type | Description |
|---|---|---|
| `aspect_ratio` | `AspectRatio` | `Fit` (letterbox), `Fill` (crop), or `Stretch` |
| `show_controls` | `bool` | Whether to display native playback controls |
| `volume` | `&Binding<Volume>` | Reactive volume binding; positive values are audible, negative values mute while preserving level |
| `muted` | `&Binding<bool>` | Reactive mute toggle layered on top of volume |
| `playback_rate` | `&Binding<f32>` | Reactive playback speed (1.0 = normal) |
| `preserve_pitch` | `&Binding<bool>` | Keep audio pitch constant when speed is not 1x |
| `playback_policy` | `PlaybackPolicy` | Buffering and realtime tuning |
| `on_event` | `impl Fn(Event)` | Callback for playback events |

### `Video` -- Raw View

When you need full control over the playback UI -- a custom scrubber, gesture-based controls, or a looping background -- use `Video`:

```rust,ignore
use waterui::prelude::*;
use waterui::media::{AspectRatio, Video};

fn background_video() -> impl View {
    Video::new("https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/ForBiggerJoyrides.mp4")
        .aspect_ratio(AspectRatio::Fill)
        .loops(true)
}
```

### Volume and Mute System

Both video components encode mute state into the volume value itself:

- **Positive** values represent audible volume (e.g. `0.7` = 70%).
- **Negative** values represent muted state while preserving the original level
  (e.g. `-0.7` means "muted, but restore to 70% on unmute").

In practice, use the `muted` method with a `Binding<bool>` and let the
framework handle the encoding:

```rust,ignore
use waterui::prelude::*;
use waterui::media::VideoPlayer;

fn mutable_player() -> impl View {
    let muted = Binding::bool(false);
    VideoPlayer::new("https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/ElephantsDream.mp4")
        .muted(&muted)
}
```

### Video Events

Subscribe to playback lifecycle events to keep your UI in sync. The video
`Event` is re-exported from `waterui::media` as `Event`; the example aliases
it to make the variant matches read clearly:

```rust,ignore
use waterui::media::VideoPlayer;
use waterui::media::Event as VideoEvent;

fn player_with_events() -> impl View {
    VideoPlayer::new("https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/Sintel.mp4")
        .on_event(|event| match event {
            VideoEvent::ReadyToPlay => tracing::info!("ready"),
            VideoEvent::Ended => tracing::info!("playback ended"),
            VideoEvent::Buffering => tracing::info!("buffering"),
            VideoEvent::BufferingEnded => tracing::info!("resumed"),
            VideoEvent::Error { message } => tracing::error!("error: {message}"),
            _ => {}
        })
}
```

| Event | Description |
|---|---|
| `ReadyToPlay` | The video is ready to begin playback. |
| `Ended` | Playback reached the end of the video. |
| `Buffering` | Playback stalled while waiting for more data. |
| `BufferingEnded` | Enough data is available to resume playback. |
| `BufferLevel { buffered_ms }` | Reports buffered duration ahead of the playhead. |
| `PlaybackMetrics { av_drift_ms, dropped_video_frames }` | Periodic playback diagnostics. |
| `PictureInPictureChanged { active }` | PiP entered or exited. |
| `NextRequested` / `PreviousRequested` | The system or player UI asked for the next or previous queue item. |
| `Error { message }` | A load or playback error occurred. |

---

## Live Photos (Apple)

Live Photos combine a still image with a short video clip: press and hold on an iPhone and the photo comes alive. WaterUI models this through `LivePhoto` and `LivePhotoSource`:

```rust,ignore
use waterui::media::{LivePhoto, Url};
use waterui::media::live::LivePhotoSource;

fn my_live_photo() -> impl View {
    let source = LivePhotoSource::new(
        Url::parse("https://static.rust-lang.org/logos/rust-logo-512x512.png").unwrap(),
        Url::parse("https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/ForBiggerFun.mp4").unwrap(),
    );
    LivePhoto::new(source)
}
```

`LivePhoto::new` accepts any `IntoComputed<LivePhotoSource>`, so you can
drive it with a reactive signal that changes the photo dynamically.

> **Note:** Live Photos are an Apple-specific feature. On non-Apple platforms,
> the native backend may fall back to displaying just the still image.

---

## The Unified `Media` Enum

When your data model may contain images, videos, or Live Photos -- imagine a social feed or a chat thread -- use the `Media` enum. It implements `View` and automatically selects the right component:

```rust,ignore
use waterui::media::{Media, Url};
use waterui::media::live::LivePhotoSource;

let items: Vec<Media> = vec![
    Media::Image(Url::parse("https://static.rust-lang.org/logos/rust-logo-512x512.png").unwrap()),
    Media::Video(Url::parse("https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/TearsOfSteel.mp4").unwrap()),
    Media::LivePhoto(LivePhotoSource::new(
        Url::parse("https://static.rust-lang.org/logos/rust-logo-512x512.png").unwrap(),
        Url::parse("https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/SubaruOutbackOnStreetAndDirt.mp4").unwrap(),
    )),
];
```

Each variant renders via its corresponding component:

| Variant | Renders As |
|---|---|
| `Media::Image(url)` | `Photo` |
| `Media::Video(url)` | `VideoPlayer` |
| `Media::LivePhoto(source)` | `LivePhoto` |

---

## Media Picker

Want to let users choose a photo or video from their device? The `MediaPicker` component presents the platform's native media selection dialog. It requires the `std` feature, which is enabled by default.

### Basic Selection

```rust,ignore
use waterui::prelude::*;
use waterui::media::media_picker::{MediaFilter, MediaPicker, Selected};

fn picker_demo() -> impl View {
    let selection: Binding<Option<Selected>> = Binding::container(None);

    MediaPicker::new(&selection)
        .filter(MediaFilter::Image)
}
```

### Media Filters

Control what type of media the user can select:

| Filter | Description |
|---|---|
| `MediaFilter::Image` | Only images |
| `MediaFilter::Video` | Only videos |
| `MediaFilter::LivePhoto` | Only Live Photos |
| `MediaFilter::Any(vec)` | Any of the listed filters |
| `MediaFilter::All(vec)` | All conditions must match |
| `MediaFilter::Not(vec)` | Exclude the listed filters |

### Custom Label

Replace the default "Select Media" button text:

```rust,ignore
use waterui::prelude::*;
use waterui::media::media_picker::{MediaPicker, Selected};

fn custom_picker() -> impl View {
    let selection: Binding<Option<Selected>> = Binding::container(None);

    MediaPicker::new(&selection)
        .label(text("Choose a photo"))
}
```

### Accessing the Result

Once the user selects media, inspect the `Selected` value through its
`media()` accessor:

```rust,ignore
use waterui::media::Media;
use waterui::media::media_picker::Selected;

fn handle_selection(selected: &Selected) {
    match selected.media() {
        Media::Image(url) => tracing::info!("selected image: {url}"),
        Media::Video(url) => tracing::info!("selected video: {url}"),
        Media::LivePhoto(_source) => tracing::info!("selected live photo"),
    }
}
```

---

## Image Filters

Filter modifiers come from `ViewExt` and accept any `IntoSignalF32`, so you can
hand them a literal value or a `Binding<f64>` for live updates:

```rust,ignore
use waterui::prelude::*;
use waterui::media::Photo;

fn vintage_photo() -> impl View {
    Photo::new("https://static.rust-lang.org/logos/rust-logo-512x512.png")
        .saturation(0.6)
        .brightness(-0.05)
        .blur(1.5)
}
```

Common modifiers: `.blur(radius)`, `.brightness(amount)`, `.contrast(amount)`,
`.saturation(amount)`. The full list (and how filters compose into a single
GPU pass) lives in [Filters and Visual Effects](../05-graphics/04-filters.md).

WaterUI also re-exports `filtrate::Filter` as `waterui::media::Filter` if you
need to construct filter pipelines manually.

---

## Platform Considerations

| Feature | Apple | Android | Desktop (Hydrolysis) |
|---|---|---|---|
| Photo (network images) | Full support | Full support | Full support |
| HDR (AVIF/HEIC) | Platform decoder, RGBA16F | Platform decoder, RGBA16F | Software fallback (SDR) |
| VideoPlayer controls | Native (AVPlayerViewController) | WaterUI/Rust controls | WIP |
| Live Photos | Native support | Image-only fallback | Image-only fallback |
| Media Picker | Native photo picker | Native photo picker | File dialog fallback |
| Streaming decode | JPEG, PNG, GIF, WebP, BMP, TIFF | Same | Same |

### Supported Image Formats

The decoding pipeline automatically selects between a platform-native decoder
(Apple/Android) and a software fallback depending on the format and platform:

- **Software path:** JPEG, PNG, GIF, WebP, BMP, ICO, TIFF
- **Platform path:** AVIF, HEIC/HEIF (Apple & Android only), images with
  embedded ICC/cICP color profiles

---

## What's Next

Now that you can display images and play video, the next chapter explores [Maps and Location](02-maps.md) -- embedding interactive maps, dropping pins, and tracking the user's real-time position.

# Media: photos, video, and audio

> **In this chapter, you will:**
> - Display network images with `Photo`, including reactive URLs and progressive decoding
> - Play video in one line with `video()` and `video_player()`
> - Own playback state in a `PlaybackSession` and drive it through a `PlayerController`
> - Build playlists, custom transport controls, and Live Photos
> - Let users pick media with the platform-native `MediaPicker`

The media stack does two jobs. For images it handles async fetching, progressive decoding, and GPU texture upload. For video and audio it gives you a *playback session*: an owned object that holds the playlist, the position, the volume, and every other piece of playback state, separate from the view that displays it.

## Crates and imports

`media` is a default feature of `waterui`, and it turns on `video` with it:

```toml
[dependencies]
waterui = "*"                       # media + video are on by default

# Video types only, without the photo/picker stack:
# waterui = { version = "*", default-features = false, features = ["video"] }
```

| Crate | Path | Contents |
|---|---|---|
| `waterui-media` | `waterui::media` | `Photo`, `LivePhoto`, `Media`, `MediaPicker`, `Image`, plus re-exports of the video types |
| `waterui-video` | `waterui::video` | `Video`, `VideoPlayer`, `PlaybackSession`, `PlayerController`, `Playlist`, `MediaItem` |

`waterui::media` re-exports the video types it needs, so one import covers most
apps:

```rust,ignore
use waterui::prelude::*;
use waterui::media::{LivePhoto, Media, Photo, PlaybackSession, Playlist, Video, VideoPlayer};
```

The prelude also re-exports these names, but spelling them out keeps chapters
and examples greppable.

---

## Displaying images with `Photo`

```rust,ignore
use waterui::media::Photo;

fn avatar() -> impl View {
    Photo::new("https://static.rust-lang.org/logos/rust-logo-512x512.png")
}
```

`Photo::new` takes `impl IntoComputed<Url>`. That covers a string literal, a
`Url`, and — the interesting case — a signal. Passing a signal swaps the image
when the URL changes without rebuilding the view:

```rust,ignore
use waterui::prelude::*;
use waterui::media::{Photo, Url};

fn hero(selected: Computed<Url>) -> impl View {
    Photo::new(selected).resizable()
}
```

`.resizable()` lets the decoded image stretch to the bounds its parent proposes;
without it the image keeps its intrinsic pixel size. For a local file, use
`Photo::from_path("/path/to/image.png")`.

### Load events

`Photo` reports load outcomes through `on_event`. Both `waterui::media` and
`waterui::media::photo` export a type named `Event`, so alias the one you mean:

```rust,ignore
use waterui::media::Photo;
use waterui::media::photo::Event as PhotoEvent;

fn profile_photo() -> impl View {
    Photo::new("https://static.rust-lang.org/logos/rust-logo-512x512.png")
        .on_event(|event: PhotoEvent| match event {
            PhotoEvent::Loaded => tracing::info!("image loaded"),
            PhotoEvent::Error(message) => tracing::error!("image failed: {message}"),
        })
}
```

`PhotoEvent` has exactly two variants: `Loaded` and `Error(String)`.

### Progressive decoding

`Photo` feeds the HTTP response into an `ImageStreamDecoder` as chunks arrive.
The first decode attempt happens at 24 KB, then every 96 KB after that, for up
to ten attempts. When the format supports it — JPEG, PNG, GIF, WebP, BMP, ICO,
TIFF — a low-quality preview appears before the full image lands. There is
nothing to configure.

### Filters take signals

Filter modifiers come from `FilterViewExt` (the `gpu` feature, on by default)
and accept anything convertible to an `f32` signal, so a `Binding` drives them
live:

```rust,ignore
use waterui::prelude::*;
use waterui::component::slider;
use waterui::media::Photo;

fn blurry_photo() -> impl View {
    let blur = Binding::f64(0.0);
    let saturation = Binding::f64(1.0);

    vstack((
        Photo::new("https://static.rust-lang.org/logos/rust-logo-512x512.png")
            .blur(blur.clone())
            .saturation(saturation.clone()),
        slider("Blur radius", &blur).range(0.0..=20.0),
        slider("Saturation", &saturation).range(0.0..=2.0),
    ))
}
```

The full catalog — `blur`, `brightness`, `contrast`, `saturation`, `exposure`,
`gamma`, `vibrance` — and how they collapse into one GPU pass is in
[Filters and Visual Effects](../05-graphics/04-filters.md).

### Pixels you already have

`Image` is the GPU-backed view underneath `Photo`. Build one directly when the
pixels come from somewhere other than a URL:

```rust,ignore
use waterui::media::Image;

let pixels: Vec<u8> = vec![255, 0, 0, 255]; // one red RGBA pixel
let red_dot = Image::new(pixels, 1, 1);
```

HDR sources decode to RGBA16F and are tone-mapped at draw time when the output
surface is SDR.

---

## Playing video

| Component | Controls | Use for |
|---|---|---|
| `Video` | none (raw surface) | custom player UI, background clips |
| `VideoPlayer` | platform-appropriate controls | ordinary playback |

Each has a free-function constructor for the single-item case:

```rust,ignore
use waterui::media::video::{video, video_player};

fn trailer() -> impl View {
    // Paused, with controls. The user starts it.
    video_player("https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4")
}

fn ambient_background() -> impl View {
    // Autoplays, loops, no controls.
    video("https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/ForBiggerJoyrides.mp4")
}
```

`video()` starts playing as soon as the surface is ready; `video_player()` stays
paused. `Video` loops by default (`.loops(false)` to stop at the end), and both
accept `.aspect_ratio(AspectRatio::Fit | Fill | Stretch)`. `VideoPlayer` hides
its controls with `.show_controls(false)`.

---

## Playback sessions

`Video::new` and `VideoPlayer::new` do not take a URL. They take a
`PlaybackSession` — the object that owns the playlist, the position, the volume,
the track selections, and the transport state:

```rust,ignore
use waterui::media::{PlaybackSession, Playlist, Video};

let session = PlaybackSession::new(Playlist::single(
    "https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4",
));
let controller = session.controller();
let view = Video::new(session);
```

This split is the point. The view is a projection of the session, not the owner
of it — so playback state lives at whatever level of your app actually owns it,
and the view that renders it can be rebuilt, moved between containers, or
swapped from `Video` to `VideoPlayer` without resetting the stream. WaterUI has
no hidden per-view state slots to lose; if a value must survive, you hold it, the
same way you hold a `Binding`.

A session is mounted exactly once. `session.controller()` hands out a
`PlayerController` that is cheap to clone, so pass one copy to every control that
needs it.

### Driving your own transport controls

```rust,ignore
use waterui::prelude::*;
use waterui::media::{AspectRatio, PlaybackSession, Playlist, Video};

fn custom_player() -> impl View {
    let session = PlaybackSession::new(Playlist::single(
        "https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4",
    ));
    let controller = session.controller();

    let muted = controller.muted();
    let elapsed = controller.position().map(|position| position.as_secs()).computed();

    let play = controller.clone();
    let pause = controller.clone();
    let rewind = controller;

    vstack((
        Video::new(session).aspect_ratio(AspectRatio::Fit).loops(false),
        hstack((
            button("Play").action(move || play.play()),
            button("Pause").action(move || pause.pause()),
            button("Back 10s").action(move || {
                if let Err(error) = rewind.seek_relative(-10.0) {
                    tracing::warn!("seek rejected: {error}");
                }
            }),
            toggle("Mute", &muted),
            text!("{elapsed}s"),
        ))
        .spacing(12.0),
    ))
}
```

`muted()` returns the session's own `Binding<bool>`, so the toggle and the player
read and write the same state — no synchronization code. `position()` returns a
`Computed<Duration>`, which maps into whatever the label needs.

### The controller surface

Commands that can fail return `Result<_, PlaybackError>`; the rest return `()`.

| Command | Notes |
|---|---|
| `play` / `pause` / `stop` | `stop` pauses and returns to the start of the item |
| `step_forward` / `step_backward` | pause and move one frame |
| `seek(Duration)` | errors outside the duration or live window |
| `seek_relative(f64)` | signed seconds, clamped to the item bounds |
| `seek_to_live_edge()` | errors for finite media |
| `next()` / `previous()` / `seek_to_item(id)` | playlist navigation |
| `set_repeat(RepeatMode)` / `set_shuffle(bool)` | traversal policy |
| `replace_playlist` / `add_item` / `remove_item(id)` / `move_item(id, index)` | playlist editing |

Reactive state comes back as signals you can hand straight to views:

| Getter | Type |
|---|---|
| `phase()` | `Computed<PlaybackPhase>` — `Idle`, `Preparing`, `Ready`, `Playing`, `Paused`, `Buffering`, `Ended`, `Failed` |
| `position()` / `duration()` | `Computed<Duration>` (duration is zero until known) |
| `current_item_id()` / `current_item_index()` | `Computed<MediaItemId>` / `Computed<usize>` |
| `track_catalog()` / `live_window()` | `Computed<TrackCatalog>` / `Computed<Option<LiveWindow>>` |
| `volume()` / `muted()` / `playback_rate()` / `preserve_pitch()` | shared `Binding`s |
| `subtitle_selection()` / `audio_track_selection()` / `video_track_selection()` | shared `Binding`s |
| `repeat_mode()` / `shuffle_enabled()` | shared `Binding`s |

### Volume and mute are separate

`Volume` is a validated linear level in `0.0..=1.0` — constructing one outside
that range panics — and muting is its own boolean binding, so a single value can
never encode two unrelated states:

```rust,ignore
use waterui::media::Volume;

controller.volume().set(Volume::new(0.35));
controller.muted().set(true);       // volume is still 0.35 when unmuted
```

`Volume::SILENT` and `Volume::FULL` are the constants; the default is `0.5`.

### Playlists

`Playlist` is non-empty by construction, so there is no "nothing is playing"
state to defend against:

```rust,ignore
use waterui::media::{PlaybackSession, Playlist, VideoPlayer};
use waterui::video::{Delivery, MediaItem, MediaMetadata};

fn episode_queue() -> impl View {
    let live = MediaItem::new(
        "https://devstreaming-cdn.apple.com/videos/streaming/examples/bipbop_4x3/bipbop_4x3_variant.m3u8",
        Delivery::Hls,
    )
    .metadata(MediaMetadata::new().with_title("Episode 1").with_artist("WaterUI"));

    let playlist = Playlist::new(
        live,
        [MediaItem::from(
            "https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/ElephantsDream.mp4",
        )],
    );

    VideoPlayer::new(PlaybackSession::new(playlist).autoplay())
}
```

`Playlist::new(first, remaining)` takes the first item separately to keep the
invariant in the type system; `Playlist::single(item)` is the one-item case.
`remove_item` refuses the final item with `PlaybackError::CannotRemoveFinalItem`
rather than leaving an empty session.

`MediaItem::from` covers plain URLs with progressive delivery. Use
`MediaItem::new(url, Delivery::Hls | Delivery::Dash)` for adaptive streams, and
`MediaMetadata` (title, artist, album, artwork URL, duration) to populate the
platform's system media session and now-playing UI.

Repeat and shuffle live on the controller: `RepeatMode::{Off, One, All}`, and
`set_shuffle(true)` traverses a stable order derived from item identity without
reordering the playlist itself.

### Buffering policy

`PlaybackSession::new` uses `PlaybackPolicy::vod_default()`. For a live stream,
pass the realtime policy explicitly:

```rust,ignore
use waterui::media::{PlaybackSession, Playlist};
use waterui::video::{Delivery, MediaItem, PlaybackPolicy, Url};

fn live_session(stream: Url) -> PlaybackSession {
    PlaybackSession::with_policy(
        Playlist::single(MediaItem::new(stream, Delivery::Hls)),
        PlaybackPolicy::live_default(),
    )
}
```

`NetworkPlaybackPolicy` under it makes every bound explicit — maximum manifest
and segment bytes, initial bandwidth estimate, buffer thresholds, live catch-up
rate limits — so a hostile manifest cannot make the player allocate without
limit.

---

## Playback events

`Video` and `VideoPlayer` share one `Event` type:

```rust,ignore
use waterui::media::Event as VideoEvent;
use waterui::media::video::video_player;

fn player_with_events() -> impl View {
    video_player("https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/Sintel.mp4")
        .on_event(|event: VideoEvent| match event {
            VideoEvent::ReadyToPlay => tracing::info!("ready"),
            VideoEvent::Buffering => tracing::info!("buffering"),
            VideoEvent::BufferingEnded => tracing::info!("resumed"),
            VideoEvent::Ended => tracing::info!("ended"),
            VideoEvent::Error { message } => tracing::error!("playback error: {message}"),
            _ => {}
        })
}
```

| Event | Meaning |
|---|---|
| `ReadyToPlay` | the current item can start |
| `PlaybackStateChanged { playing }` | media time started or stopped advancing |
| `Buffering` / `BufferingEnded` | playback stalled on data, then resumed |
| `BufferLevel { buffered_ms }` | buffered duration ahead of the playhead |
| `PlaybackMetrics { metrics }` | periodic diagnostics: A/V drift, dropped frames, rebuffer counts |
| `PictureInPictureChanged { active }` / `ExternalPlaybackChanged { active }` | presentation route changed |
| `TimedMetadata { metadata }` | a container metadata event reached its timestamp |
| `NextRequested` / `PreviousRequested` | system or player UI asked to change item |
| `Ended` | the current item reached its end |
| `Error { message }` | load or playback failure |

For state you want to *render*, prefer `controller.phase()` over counting
events: it is a signal, so it updates the exact view that reads it.

---

## Live Photos

`LivePhoto` is composed in Rust from a `Photo`, a muted `Video`, and a long-press
gesture — no per-platform live-photo primitive is involved, so it behaves the
same anywhere photos and video work:

```rust,ignore
use waterui::media::{LivePhoto, Url};
use waterui::media::live::LivePhotoSource;

fn memory() -> impl View {
    LivePhoto::new(LivePhotoSource::new(
        Url::from_file_path_str("beach.heic"),
        Url::from_file_path_str("beach.mov"),
    ))
    .activation_duration_ms(400)
}
```

Press and hold for `activation_duration_ms` (250 ms by default) and the motion
clip plays once over the still image, then the still returns. `LivePhoto::new`
accepts `impl IntoComputed<LivePhotoSource>`, so the pair can come from a signal
— usually one the `MediaPicker` below produced, or files your app ships.

---

## The unified `Media` enum

When your data model can hold any of the three kinds, `Media` implements `View`
and picks the component for you:

```rust,ignore
use waterui::media::{Media, Url};
use waterui::media::live::LivePhotoSource;

let feed = vec![
    Media::Image(Url::from("https://static.rust-lang.org/logos/rust-logo-512x512.png")),
    Media::Video(Url::from(
        "https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/TearsOfSteel.mp4",
    )),
    Media::LivePhoto(LivePhotoSource::new(
        Url::from_file_path_str("beach.heic"),
        Url::from_file_path_str("beach.mov"),
    )),
];
```

| Variant | Renders as |
|---|---|
| `Media::Image(url)` | `Photo` |
| `Media::Video(url)` | `video_player(url)` — paused, with controls |
| `Media::LivePhoto(source)` | `LivePhoto` |

For a feed whose membership changes, render it with `ForEach`/`List` over a
reactive collection so items diff by identity — see
[Lists and collections](../03-ui/05-lists.md).

---

## Audio

There is no audio-only component at this pin. An audio track is a `MediaItem` in
a `PlaybackSession`, driven by the same `PlayerController` — and because a
session only starts once a view mounts it, you still place a `Video` (with your
own controls, and no visible surface to speak of) to host the session. Fill in
`MediaMetadata` so the platform's now-playing UI and lock-screen controls show
the right title, artist, and artwork.

For a live microphone visualization, the `waterui-visualizer` crate provides
`Waveform`. It is not re-exported through `waterui`, so add it as its own
dependency:

```rust,ignore
use waterui_visualizer::{AudioCapture, waveform};

fn microphone_meter() -> impl View {
    waveform(AudioCapture::new()).sensitivity(1.5)
}
```

Constructing an `AudioCapture` has no side effects; recording starts when the
first visualizer using it finishes GPU setup, and clones share that one recorder
and sample buffer. Request `Permission::Microphone` (from `waterkit-permission`)
and confirm it was granted before you show the waveform.

---

## Media picker

`MediaPicker` renders as a button that opens the platform's media selection
dialog. It needs the `std` feature, which is on by default.

```rust,ignore
use waterui::prelude::*;
use waterui::media::media_picker::{MediaFilter, MediaPicker, Selected};

fn picker_demo() -> impl View {
    let selection: Binding<Option<Selected>> = Binding::container(None);

    MediaPicker::new(&selection)
        .filter(MediaFilter::Image)
        .label(text("Choose a photo"))
}
```

`filter` accepts a signal, so the allowed types can change while the view is
live. The default label is "Select Media".

| Filter | Selects |
|---|---|
| `MediaFilter::Image` / `Video` / `LivePhoto` | one kind |
| `MediaFilter::Any(vec)` | any of the listed filters |
| `MediaFilter::All(vec)` | all conditions must match |
| `MediaFilter::Not(vec)` | everything except the listed filters |

The binding fills with a `Selected`. Borrow the payload with `media()`, or take
ownership with `load()`:

```rust,ignore
use waterui::media::Media;
use waterui::media::media_picker::Selected;

fn describe(selected: &Selected) {
    match selected.media() {
        Media::Image(url) => tracing::info!("selected image: {url}"),
        Media::Video(url) => tracing::info!("selected video: {url}"),
        Media::LivePhoto(source) => tracing::info!(?source, "selected live photo"),
    }
}
```

---

## Platform notes

- **Video realization.** Apple platforms bridge AVPlayer/AVKit. Every non-Apple
  target gets WaterUI's own GPU video player, installed by `export!()` at
  compile time. This is a `cfg` decision, not a runtime switch: there is no
  environment variable to flip and no silent fallback between the two.
- **HDR video.** The backend negotiates HDR and tone-maps to SDR when the output
  surface cannot display it. Applications do not configure the pipeline; to
  *observe* it, read `VideoTrackInfo::is_hdr()` from `controller.track_catalog()`.
- **Still images.** Apple platforms decode HEIF and AVIF through the system
  decoder; other platforms use the software decoder, which includes an AVIF path
  on desktop.
- **Live Photos** work everywhere, because they are a Rust-side composition.
  Whether the *picker* can return one depends on the platform dialog.

---

Next: [Maps and Location](02-maps.md) — embedding an interactive map, dropping
annotations, and following the user's position.

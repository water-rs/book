# Media: photos, video, and audio

> **In this chapter, you will:**
> - Enable the `media`, `video`, `video-gpu` and `remote` features and know what each one links
> - Display images with `Photo`, including reactive URLs, load events, progressive decoding, and filters that take signals
> - Play video with `video()` and `video_player()`, and own playback state in a `PlaybackSession` driven through a `PlayerController`
> - Build playlists, Live Photos, and the unified `Media` view, and let users pick media with `MediaPicker`
> - Know which realization draws video on each platform, and what audio support exists today

The media stack does two jobs. For images it fetches, decodes progressively, and uploads pixels to the renderer. For video it gives you a *playback session*: an owned object that holds the playlist, the position, the volume, and every other piece of playback state, separate from the view that displays it.

## Features and imports

Neither media nor video is on by default. An application that shows no pictures should not link a decoder, and one that never names a web URL should not link an HTTP client, so each capability is its own feature of `waterui`:

| Feature | Turns on |
|---|---|
| `media` | `waterui::media`: `Photo`, `Image`, `Media`, `MediaPicker`, and the image kind of `asset!` |
| `video` | `waterui::video` (the playback API) and, together with `media`, `Video`, `VideoPlayer`, `LivePhoto` and the video kinds of `Media` and `MediaPicker` under `waterui::media` |
| `video-gpu` | WaterUI's own GPU video player; implies `video` and `gpu` |
| `remote` | the HTTP client that lets `Photo` (and `Url::fetch`) read web URLs |

```toml
[dependencies]
waterui = { version = "0.5", features = ["media", "video", "remote"] }
```

The examples below log through `waterui::log`, the facade's re-export of `tracing`, so they need no extra dependency.

Add `video-gpu` on the targets that have no native player to bridge (see [Platform notes](#platform-notes)). It is deliberately not implied by `video`: Apple platforms and the web bridge a player of their own, and an application there would otherwise link one it never runs. Enabling it everywhere is safe but not free: the framework never installs the GPU player on Apple platforms or the web, and a `wasm32` build does not link it, but an Apple build still compiles and links it.

Without `remote`, a `Photo` pointed at a web URL reports an error that names the missing feature. Local files and bundled assets need nothing extra. Video does not go through `remote`: each video realization loads its sources through its own player.

| Crate | Path | Contents |
|---|---|---|
| `waterui-media` | `waterui::media` | `Photo`, `LivePhoto`, `Media`, `MediaPicker`, `Image`, `Url`, `Filter`, plus re-exports of the common video types |
| `waterui-video` | `waterui::video` | `Video`, `VideoPlayer`, `PlaybackSession`, `PlayerController`, `Playlist`, `MediaItem`, policies and events |
| `waterui-image` | `waterui::media::Image` | the image view itself. It lives in its own repository and is re-exported here |

With `media` on, the prelude also re-exports the items at the root of `waterui::media` (`Photo`, `Image`, `Media`, `MediaPicker`, `Url`, and with `video` the video types listed above), as well as the `media` and `video` modules. The examples below still import each type by its path, so you can see where it lives.

---

## Displaying images with `Photo`

```rust
use waterui::prelude::*;
use waterui::media::Photo;

fn avatar() -> impl View {
    Photo::new("https://static.rust-lang.org/logos/rust-logo-512x512.png")
}
```

`Photo::new` takes `impl IntoComputed<Url>`. That covers a string literal, a `Url`, and the interesting case, a signal. Passing a signal swaps the image when the URL changes without rebuilding the view:

```rust
use waterui::prelude::*;
use waterui::media::{Photo, Url};

fn hero(selected: Computed<Url>) -> impl View {
    Photo::new(selected).resizable()
}
```

A runtime string is not a `Url`. Parse it with `Url::parse`, which accepts only web URLs and returns an `Option`, and report the failure rather than letting a bad address reach the loader as a local path. For a local file, use `Photo::from_path("/path/to/image.png")`.

By default a photo keeps its natural pixel size and only scales down when offered less. `.resizable()` lets it fill the bounds its parent proposes, stretching each axis independently. Add `.content_mode(...)` to keep the aspect ratio: `ContentMode::Fit` letterboxes, `ContentMode::Fill` covers the bounds and crops the overflow.

```rust
use waterui::prelude::*;
use waterui::media::Photo;

fn cover(url: &'static str) -> impl View {
    Photo::new(url)
        .resizable()
        .content_mode(ContentMode::Fill)
}
```

`ContentMode` here is the layout one from the prelude. Video has its own type of the same name, covered below.

### Load events

`Photo` reports load outcomes through `on_event`. With `video` on, both `waterui::media` and `waterui::media::photo` export a type named `Event`, and the one at the root of `media` is the *video* event, so import the photo one under an alias:

```rust
use waterui::prelude::*;
use waterui::media::Photo;
use waterui::media::photo::Event as PhotoEvent;

fn profile_photo() -> impl View {
    Photo::new("https://static.rust-lang.org/logos/rust-logo-512x512.png")
        .on_event(|event: PhotoEvent| match event {
            PhotoEvent::Loaded => waterui::log::info!("image loaded"),
            PhotoEvent::Error(message) => waterui::log::error!("image failed: {message}"),
            // `Event` is non-exhaustive: later releases may add variants.
            _ => {}
        })
}
```

`Loaded` means the final decoded frame is registered with the renderer and on screen. A frame the renderer refuses reports `Error` instead, never `Loaded`. The handler follows the same extractor pattern as `Button::action`: after the event argument it may take `State<T>` and other environment values.

Attach `on_event` before any filter. It is a method on `Photo`, and a filter wraps the view in a different type.

### Progressive decoding

For a web URL, `Photo` feeds the HTTP response into an `ImageStreamDecoder` as chunks arrive. The first preview decode is tried at 24 KB, then after every further 96 KB, for up to ten attempts, and only when the bytes look like a format that can decode partially. A low-quality preview then appears before the full image lands, and the final frame replaces it. Preview attempts stop once more than 8 MB has arrived, and the final decode still reads the whole response. A local file is read and decoded in one step. There is nothing to configure.

### Filters take signals

Filter modifiers come from `FilterViewExt` (in the prelude with the `effects` feature, which the default `gpu` feature implies) and accept anything convertible to an `f32` signal, so a `Binding` drives them live:

```rust
use waterui::prelude::*;
use waterui::media::Photo;

fn tunable_photo() -> impl View {
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

The full catalog and how filters collapse into one GPU pass is in [Filters and visual effects](../05-graphics/04-filters.md).

### Pixels you already have

`Image` is the view underneath `Photo`. Build one directly when the pixels come from somewhere other than a URL:

```rust
use waterui::prelude::*;
use waterui::media::Image;

fn red_dot() -> impl View {
    // One RGBA8 pixel: `Image::new` asserts `len == width * height * 4`.
    Image::new(vec![255, 0, 0, 255], 1, 1)
        .resizable()
        .content_mode(ContentMode::Fit)
}
```

`Image::from_encoded(&bytes)` decodes encoded bytes (PNG, JPEG, GIF, WebP, BMP, ICO and TIFF everywhere; HEIF, HEIC and AVIF where the platform decoder reads them) into an `Image` and returns `Result<Image, String>`. `Image::new_rgba16f` takes half-float RGBA and tone-maps high-dynamic-range values into the standard range on the way in. `image.status()` is a signal that reads `Pending` until a render target first draws the image, then `Ready` or `Refused` with the reason (the image exceeds the target's limits or image budget). A refused image draws nothing.

Nearest-neighbor sampling for pixel art and icons is `Image::interpolation(Interpolation::Nearest)`. `Interpolation` lives in `waterui-image` and the facade does not re-export it, so an app that needs it adds that crate as a direct dependency.

---

## Playing video

| Component | Controls | Use for |
|---|---|---|
| `Video` | none (raw surface) | custom player UI, background clips |
| `VideoPlayer` | platform-appropriate controls | ordinary playback |

Each has a free-function constructor for the single-item case, taking anything that converts into a `MediaItem` (a `&'static str` or a `Url`):

```rust
use waterui::prelude::*;
use waterui::media::video_player;
use waterui::media::video::video;

fn trailer() -> impl View {
    // Paused, with controls. The user starts it.
    video_player("https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4")
}

fn ambient_background() -> impl View {
    // Autoplays, loops, no controls.
    video("https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/ForBiggerJoyrides.mp4")
}
```

`video()` starts playing as soon as the surface is ready; `video_player()` stays paused. `Video` loops by default (`.loops(false)` to stop at the end). Both take `.content_mode(...)` with the video type of that name, and `.projection(...)` for equirectangular 360-degree sources. `VideoPlayer` hides its controls with `.show_controls(false)`.

```rust
use waterui::prelude::*;
use waterui::media::video::ContentMode as VideoContentMode;
use waterui::media::video_player;

fn cropped_player() -> impl View {
    video_player("https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4")
        .content_mode(VideoContentMode::Fill)
        .show_controls(false)
}
```

`ContentMode::Fit` (the default) keeps the aspect ratio inside the bounds; `Fill` covers them and crops; `Stretch` ignores the ratio. A `Fit` video stretches horizontally only: it takes the width it is offered and the height its source's aspect ratio gives at that width, 16:9 until the source reports its size. `Fill`, `Stretch` and an equirectangular projection take the whole proposal. This is the layout contract of the leaf, so a backend that composes the picture from WaterUI views answers with `fit_video` and `FitVideoLayout` from `waterui::video`; see [Backends](../08-internals/04-backends.md).

---

## Playback sessions

`Video::new` and `VideoPlayer::new` do not take a URL. They take a `PlaybackSession`, the object that owns the playlist, the position, the volume, the track selections, and the transport state:

```rust
use waterui::prelude::*;
use waterui::media::{PlaybackSession, Playlist, Video};

fn session_view() -> impl View {
    let session = PlaybackSession::new(Playlist::single(
        "https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4",
    ));
    Video::new(session)
}
```

This split is the point. The view is a projection of the session, not the owner of it, so playback state lives at whatever level of your app actually owns it. WaterUI has no hidden per-view state slots to lose; if a value must survive, you hold it, the same way you hold a `Binding`. See [Reactive state](../02-core/02-reactive.md) for the ownership model.

A session mounts exactly once, so grab its controller *before* the session moves into a view. `session.controller()` hands out a `PlayerController` that is cheap to clone, so pass one copy to every control that needs it.

### Driving your own transport controls

```rust
use waterui::prelude::*;
use waterui::media::video::ContentMode as VideoContentMode;
use waterui::media::{PlaybackSession, Playlist, Video};

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
        Video::new(session)
            .content_mode(VideoContentMode::Fit)
            .loops(false),
        hstack((
            button("Play").action(move || play.play()),
            button("Pause").action(move || pause.pause()),
            button("Back 10s").action(move || {
                if let Err(error) = rewind.seek_relative(-10.0) {
                    waterui::log::warn!("seek rejected: {error}");
                }
            }),
            toggle("Mute", &muted),
            text!("{elapsed}s"),
        ))
        .spacing(12.0),
    ))
}
```

`muted()` returns the session's own `Binding<bool>`, so the toggle and the player read and write the same state with no synchronization code. `position()` returns a `Computed<Duration>`, which maps into whatever the label needs.

### The controller surface

Commands that can fail return `Result<_, PlaybackError>`; the rest return `()`.

| Command | Notes |
|---|---|
| `play` / `pause` / `stop` | `stop` pauses and returns to the start of the item |
| `step_forward` / `step_backward` | pause and move one frame |
| `seek(Duration)` | errors before the duration is known, and outside the duration or live window |
| `seek_relative(f64)` | signed seconds, clamped to the item or live window; errors before the duration is known; panics on a non-finite value |
| `seek_to_live_edge()` | errors for finite media and before the live window is known |
| `next()` / `previous()` / `seek_to_item(id)` | playlist navigation |
| `set_repeat(RepeatMode)` / `set_shuffle(bool)` | traversal policy |
| `replace_playlist` / `add_item` / `remove_item(id)` / `move_item(id, index)` | playlist editing |

Reactive state comes back as signals you can hand straight to views:

| Getter | Type |
|---|---|
| `phase()` | `Computed<PlaybackPhase>`: `Idle`, `Preparing`, `Ready`, `Playing`, `Paused`, `Buffering`, `Ended`, `Failed` |
| `position()` / `duration()` | `Computed<Duration>` (duration is zero until known) |
| `current_item_id()` / `current_item_index()` | `Computed<MediaItemId>` / `Computed<usize>` |
| `track_catalog()` / `live_window()` | `Computed<TrackCatalog>` / `Computed<Option<LiveWindow>>` |
| `volume()` / `muted()` / `playback_rate()` / `preserve_pitch()` | shared `Binding`s |
| `subtitle_selection()` / `audio_track_selection()` / `video_track_selection()` | shared `Binding`s of `Auto`, `Off` or `Track(index)` (no `Off` for audio and video) |
| `repeat_mode()` / `shuffle_enabled()` | shared `Binding`s |

### Volume and mute are separate

`Volume` is a validated linear level in `0.0..=1.0`, and constructing one outside that range panics. Muting is its own boolean binding, so one value can never encode two unrelated states:

```rust
use waterui::media::{PlayerController, Volume};

fn quiet(controller: &PlayerController) {
    controller.volume().set(Volume::new(0.35));
    controller.muted().set(true); // the volume is still 0.35 when unmuted
}
```

`Volume::SILENT` and `Volume::FULL` are the constants; the default is `0.5`.

### Playlists

`Playlist` is non-empty by construction, so there is no "nothing is playing" state to defend against:

```rust
use waterui::prelude::*;
use waterui::media::{PlaybackSession, Playlist, VideoPlayer};
use waterui::video::{Delivery, MediaItem, MediaMetadata};

fn episode_queue() -> impl View {
    let first = MediaItem::new(
        "https://devstreaming-cdn.apple.com/videos/streaming/examples/bipbop_4x3/bipbop_4x3_variant.m3u8",
        Delivery::Hls,
    )
    .metadata(MediaMetadata::new().with_title("Episode 1").with_artist("WaterUI"));

    let playlist = Playlist::new(
        first,
        [MediaItem::from(
            "https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/ElephantsDream.mp4",
        )],
    );

    VideoPlayer::new(PlaybackSession::new(playlist).autoplay())
}
```

`Playlist::new(first, remaining)` takes the first item separately to keep the invariant in the type system; `Playlist::single(item)` is the one-item case. `remove_item` refuses the final item with `PlaybackError::CannotRemoveFinalItem` rather than leaving an empty session.

`MediaItem::from` covers plain URLs with progressive delivery. Use `MediaItem::new(url, Delivery::Hls)` or `Delivery::Dash` for adaptive streams. Each item has a stable `MediaItemId` (`item.get_id()`) that survives playlist edits, so collect the ids before the items move into a playlist; `seek_to_item` takes them. Further builders on an item are `.subtitle_track(SubtitleTrack::new(url).label("English").language("en"))` for side-loaded subtitles, `.metadata(...)`, and `.drm(DrmConfiguration::new().license_url(...))` for protected content.

`MediaMetadata` (title, artist, album, artwork URL, duration) populates the platform's system media session and now-playing UI. Repeat and shuffle live on the controller: `RepeatMode::{Off, One, All}`, and `set_shuffle(true)` traverses a stable order derived from item identity without reordering the playlist.

### Buffering policy

`PlaybackSession::new` uses `PlaybackPolicy::vod_default()`. For a live stream, pass the realtime policy explicitly:

```rust
use waterui::media::{PlaybackSession, Playlist};
use waterui::video::{Delivery, MediaItem, PlaybackPolicy, Url};

fn live_session(stream: Url) -> PlaybackSession {
    PlaybackSession::with_policy(
        Playlist::single(MediaItem::new(stream, Delivery::Hls)),
        PlaybackPolicy::live_default(),
    )
}
```

Each policy carries a `NetworkPlaybackPolicy` (read it with `policy.network()`) that makes every HLS and DASH bound explicit: maximum manifest and segment bytes, initial bandwidth estimate, buffer thresholds and live catch-up rate limits. A hostile manifest therefore cannot make the player allocate without limit. Both presets use `NetworkPlaybackPolicy::standard()` (4 MB manifests, 64 MB segments, a 5 Mbit/s initial estimate), and `PlaybackPolicy` has no setter to replace it.

`.power(PlaybackPowerPolicy::RequireAudioOffload)` or `.power(PlaybackPowerPolicy::RequireAudioVideoTunneling)` on a policy requires a hardware audio-offload or A/V tunnel path. If the platform cannot provide the required path, preparation fails instead of silently starting on another one. The default, `PlatformManaged`, lets the backend choose.

---

## Playback events

`Video` and `VideoPlayer` share one `Event` type, exported as `waterui::media::Event` and `waterui::video::Event`:

```rust
use waterui::prelude::*;
use waterui::media::Event as VideoEvent;
use waterui::media::video_player;

fn player_with_events() -> impl View {
    video_player("https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/Sintel.mp4")
        .on_event(|event: VideoEvent| match event {
            VideoEvent::ReadyToPlay => waterui::log::info!("ready"),
            VideoEvent::Buffering => waterui::log::info!("buffering"),
            VideoEvent::BufferingEnded => waterui::log::info!("resumed"),
            VideoEvent::Ended => waterui::log::info!("ended"),
            VideoEvent::Error { message } => waterui::log::error!("playback error: {message}"),
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
| `PlaybackOutputPathChanged { path }` | the backend chose its media path (`PlatformManaged`, `DecodedPcmGpu`, `AudioOffload`, `AudioVideoTunneling`) |
| `PictureInPictureChanged { active }` / `ExternalPlaybackChanged { active }` | presentation route changed |
| `TimedMetadata { metadata }` | a container metadata event reached its timestamp |
| `OfflineDrmKeySetChanged { key_set }` | a persistent license was renewed and the stored key set must be replaced |
| `NextRequested` / `PreviousRequested` | system or player UI asked to change item |
| `Ended` | the current item reached its end |
| `Error { message }` | load or playback failure |

The enum is non-exhaustive, so keep a wildcard arm. For state you want to *render*, prefer `controller.phase()` over counting events: it is a signal, so it updates exactly the view that reads it.

---

## Live Photos

`LivePhoto` is composed in Rust from a `Photo`, a muted autoplaying `Video`, and a long-press gesture. No per-platform live-photo primitive is involved, so it behaves the same anywhere photos and video work. It needs the `video` feature.

```rust
use waterui::prelude::*;
use waterui::media::LivePhoto;
use waterui::media::live::{Event as LiveEvent, LivePhotoSource};

fn memory(source: LivePhotoSource) -> impl View {
    LivePhoto::new(source)
        .activation_duration_ms(400)
        .on_event(|event: LiveEvent| match event {
            LiveEvent::MotionStarted => waterui::log::info!("motion started"),
            LiveEvent::MotionEnded => waterui::log::info!("motion ended"),
            LiveEvent::MotionFailed(message) => waterui::log::warn!("motion failed: {message}"),
            _ => {}
        })
}
```

Press and hold for `activation_duration_ms` (250 ms by default) and the motion clip plays once over the still image, then the still returns. `MotionFailed` carries the playback error and also puts the still back, so a handler can tell a completed motion from one that never played. `LivePhoto::new` accepts `impl IntoComputed<LivePhotoSource>`, so the pair can come from a signal, usually one a `MediaPicker` produced. For files your app ships, build it with `LivePhotoSource::new(image_url, video_url)` from the URLs that `ImageAsset::url` and `VideoAsset::url` resolve (see [Assets](05-assets.md)). The still is resizable, so the Live Photo fills the bounds its parent proposes.

---

## The unified `Media` enum

When your data model can hold any of the three kinds, `Media` implements `View` and picks the component for you:

```rust
use waterui::prelude::*;
use waterui::media::live::LivePhotoSource;
use waterui::media::{Media, Url};

fn feed(memory: LivePhotoSource) -> impl View {
    vstack((
        Media::Image(Url::from(
            "https://static.rust-lang.org/logos/rust-logo-512x512.png",
        )),
        Media::Video(Url::from(
            "https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/TearsOfSteel.mp4",
        )),
        Media::LivePhoto(memory),
    ))
}
```

| Variant | Renders as | Needs |
|---|---|---|
| `Media::Image(url)` | `Photo` | `media` |
| `Media::Video(url)` | `video_player(url)`: paused, with controls | `video` |
| `Media::LivePhoto(source)` | `LivePhoto` | `video` |

`Media` is non-exhaustive, so a `match` over it needs a wildcard arm. For a feed whose membership changes, render it from a reactive collection so items diff by identity; see [Lists](../03-ui/05-lists.md).

---

## Media picker

`MediaPicker` renders as a button that opens the platform's media selection dialog. It needs the `std` feature of `waterui-media`, which the facade leaves on.

```rust
use waterui::prelude::*;
use waterui::media::media_picker::{MediaFilter, MediaPicker, Selected};
use waterui::media::Media;

fn picker_demo() -> impl View {
    let selection: Binding<Option<Selected>> = Binding::default();
    let picked: Binding<Option<Media>> = Binding::default();

    vstack((
        MediaPicker::new(&selection)
            .filter(MediaFilter::Image)
            .label(text("Choose a photo"))
            .on_change(
                &selection,
                |selected: Option<Selected>, State(picked): State<Binding<Option<Media>>>| {
                    picked.set(selected.map(Selected::load));
                },
            )
            .state(&picked),
        Dynamic::watch(picked, |media| match media {
            Some(media) => AnyView::new(media),
            None => AnyView::new(text("Nothing chosen yet")),
        }),
    ))
}
```

`filter` accepts a signal, so the allowed kind can change while the view is live. The default filter is `Image` and the default label is "Select Media".

| Filter | Selects | Needs |
|---|---|---|
| `MediaFilter::Image` | a still image | `media` |
| `MediaFilter::Video` | a video | `video` |
| `MediaFilter::LivePhoto` | a paired still and motion clip | `video` |

The binding fills with a `Selected`. Borrow the payload with `media()`, or take ownership with `load()`; both give a `Media` whose URLs are local file paths:

```rust
use waterui::media::Media;
use waterui::media::media_picker::Selected;

fn describe(selected: &Selected) {
    match selected.media() {
        Media::Image(url) => waterui::log::info!("selected image: {url}"),
        Media::Video(url) => waterui::log::info!("selected video: {url}"),
        Media::LivePhoto(source) => waterui::log::info!(?source, "selected live photo"),
        _ => waterui::log::info!("selected media of a newer kind"),
    }
}
```

A dialog that fails to open, or a selection that fails to load, is logged through `tracing` and leaves the binding unchanged. Whether the dialog can return a Live Photo depends on the platform.

---

## Audio

There is no audio-only component. What exists today:

- `AudioAsset` (from the `assets` feature, not on wasm32) is a handle to an audio file in your asset bundle. `audio.url(&resources)` resolves it to a `Url` and `audio.path(&resources)` to a filesystem path. See [Assets](05-assets.md).
- A `MediaItem` is built from any `Url`, and its `MediaMetadata` feeds the platform's now-playing UI, so an audio file can be described as an item and queued in a `Playlist`.
- Playback goes through the same `PlaybackSession` and `PlayerController` as video, hosted by a mounted `Video` or `VideoPlayer`.

The gap is a view and a backend contract designed for audio on its own: no audio-only player view, no waveform or level meter, and no microphone capture in the facade. Whether each backend plays an audio-only item through `Video` is not specified by the playback API, so test it on the platforms you ship to rather than assume it.

---

## Platform notes

Realization follows the framework's design principles: an Apple backend bridges the platform's own controls, and the self-drawn backends draw.

- **Apple platforms** (macOS, iOS) bridge AVPlayer and AVKit for `Video` and `VideoPlayer`, so they need no `video-gpu`. Picture in picture and external playback arrive as the matching events.
- **The web** uses the browser's `<video>` element, and `video-gpu` links nothing on `wasm32`. The element plays what the browser plays: progressive sources everywhere and HLS where the browser reports support, while DASH and DRM-protected items fail with a playback error. The browser owns buffering, so the policy's buffering values have no effect. It cannot step by frame or select embedded audio and video tracks, and GPU filters do not apply to its pixels. Side-loaded subtitles become `<track>` children.
- **Every other platform** (Android through Hydrolysis, Windows, Linux) uses WaterUI's own GPU video player. Enable the `video-gpu` feature there. It decodes video for GPU rendering and audio to PCM, and may use hardware audio offload or a tunnel when the policy asks for one. ExoPlayer and Media3 are not used.
- **The output path** a backend chose arrives as `PlaybackOutputPathChanged`. It is never a silent fallback: a required power path that the platform cannot satisfy fails preparation.
- **HDR.** To observe HDR sources, read `VideoTrackInfo::is_hdr()` from `controller.track_catalog()`.
- **Still images** go through the platform decoder on Apple platforms and Android for HEIF, HEIC and AVIF, and for PNG or JPEG files that carry a color profile. Every other image, and every image on other platforms, goes through the software decoder, and a failed platform decode is retried there. `Image::from_encoded_with_path` also returns the `DecodePath` it took.
- **Live Photos** work wherever photos and video do, because they are a Rust-side composition. Whether the *picker* can return one depends on the platform dialog.

Platform support tiers are in [Platforms](../01-getting-started/05-platforms.md).

---

Next: [Maps and Location](02-maps.md), which embeds an interactive map, drops annotations, and follows the user's position.

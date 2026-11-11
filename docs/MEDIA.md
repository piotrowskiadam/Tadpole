# Store banners and demo recordings

## Snapcraft featured banner

The logo banner uses Tadpole's existing green mascot and the headline “Explore your site. Find what matters.” The supplied Snapcraft form specifies PNG/JPEG, a 3:1 aspect ratio, 720×240 minimum, 4320×1440 maximum, and a 2 MB limit. The finished upload asset is [snapcraft-featured-banner.png](media/snapcraft-featured-banner.png): 2172×724 pixels, approximately 1.3 MB. It was generated with AI using the supplied logo reference; the exact generation prompt is saved alongside it. Upload it in the store's Featured banner field; it does not make the app automatically eligible for featuring.

## Offline media mode

Build the current source, then explicitly start an isolated media session:

```sh
cargo build --locked
TADPOLE_MEDIA_MODE=1 TADPOLE_MEDIA_DIR="$PWD/tadpole-media" target/debug/tadpole
```

On Windows, set the same environment variables before starting `tadpole.exe`. This mode has its own application ID, uses fictional `demo.example` pages, and makes no startup crawl or AI calls. Normal startup is unchanged. The initial crawl controls are disabled. Other interactive tools remain real: if you deliberately load a project or initiate an API action, review its contents before capturing.

A separate **Tadpole media tools** window contains:

- A target selector for the crawl window or the site visualizer. Open the visualizer from the crawl window before selecting it.
- **Save screenshot**: exports the selected app window's GTK render tree as a PNG, without the desktop, cursor or recorder controls.
- **Start recording (2 fps)** / **Stop recording**: writes numbered PNG frames to a new recording directory. Recording is capped at 120 frames (60 seconds), stops on errors, and ends when the tools window closes. A start/stop control remains visible while recording.

Keep the selected window open at a fixed size during capture. This is a low-frame-rate product walkthrough recorder, suitable for demonstrating filters, detail panels and visualization modes; it does not capture audio, the pointer, external file dialogs or other applications. PNG capture and encoding happen on the GTK thread, so use it for short demos, not performance benchmarking. Use the desktop's normal screen recorder for smooth cursor movement or narration.

Convert a recording with FFmpeg installed:

```sh
scripts/media-export.sh tadpole-media/recording-TIMESTAMP walkthrough.gif
scripts/media-export.sh tadpole-media/recording-TIMESTAMP walkthrough.mp4
```

The script uses the recording's fixed 2 fps, refuses to overwrite an existing output, and requires FFmpeg with libx264 for MP4. The raw PNGs remain available. Resizing a window midway can break video conversion; record each window size separately.

For an automated native-renderer screenshot smoke test:

```sh
TADPOLE_MEDIA_MODE=1 TADPOLE_MEDIA_AUTOSHOT=1 \
  TADPOLE_MEDIA_DIR="$PWD/tadpole-media" target/debug/tadpole
```

This saves `demo-crawl.png` and `media-tools.png` after three seconds and exits. Add `TADPOLE_MEDIA_AUTORECORD=1` to also run a three-second recording smoke test before exit. Run it in a graphical test session. GTK Broadway's renderer may not support render-to-texture correctly; use a native Wayland/X11 renderer for exported PNGs. Review captures for correct colors and complete content before publishing. Demo data is illustrative, not measured results from a real website.

# Ghostty demo

[Download the 18-second MP4](assets/lsa-in-ghostty.mp4?raw=true) · [Back to the README](../README.md)

Recorded on 2026-09-30 from a real Ghostty window on Debian 13.6 (x86-64),
using the released lsa 0.5.0 binary. Ghostty was built from its verified 1.3.1
release source; without Git metadata it reports `1.3.1-dev+0000000`. This was a
direct X11 desktop session, with Mesa llvmpipe software rendering (OpenGL 4.5),
without SSH or a multiplexer. The sample gallery contains eight generated PNG, JPEG and WebP images;
it does not contain personal photos or files. The terminal output is a screen
capture, not a mockup.

## Commands shown

```sh
lsa -l --header gallery
lsa --thumbnail-size=4 gallery
lsa gallery | head -4
```

Replace `gallery` with a directory of your own images to try the same commands
in a direct Ghostty session. The first command shows details and one-row
miniatures; the second selects the automatic gallery layout; the last shows
plain filenames passing through a pipe.

## What this verifies

The recording shows readable long-view columns, miniatures, an eight-image
gallery and plain piped output in this Linux Ghostty session. It is a limited
visual check, not the full v0.5.0 terminal validation pass. Link clicks, retained
images after scrolling back, large galleries, light themes, macOS ImageIO
formats and other terminals still need the checks in
[compatibility](compatibility.md).

## Media

- `assets/lsa-in-ghostty.mp4`: original 18-second capture, 1188×800, H.264,
  30 fps, no audio.
- `assets/lsa-in-ghostty.gif`: 960-pixel-wide, 8 fps derivative of that capture
  for inline GitHub README playback. The MP4 preserves smoother motion and
  more color detail.

Both links are repository-relative so they follow the branch being viewed.
The README embeds the GIF and links to the MP4 instead of relying on an HTML
video element in GitHub's Markdown renderer.

Regenerate the GIF from the MP4 from the repository root with FFmpeg:

```sh
ffmpeg -i docs/assets/lsa-in-ghostty.mp4 \
  -filter_complex 'fps=8,scale=960:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=256:stats_mode=full[p];[b][p]paletteuse=dither=none:diff_mode=rectangle' \
  -loop 0 docs/assets/lsa-in-ghostty.gif
```

# Roadmap

Updated 2026-09-28, after v0.5.0. lsa prints into scrollback, returns to the
shell and keeps one mixed ordering with complete names. History lives in the
[changelog](CHANGELOG.md); evidence in [compatibility](docs/compatibility.md)
and [measurements](benchmarks/README.md).

## Status

[v0.5.0](https://github.com/yungibly/lsa/releases/tag/v0.5.0) is published and
the Homebrew tap points to it. CI and the release workflow passed on four native
targets plus Rust 1.88 ([evidence](docs/compatibility.md)). The text layer was
also checked in Ghostty's terminal core (libghostty-vt). No real Ghostty visual
pass has been recorded since v0.2.0.

## Next

1. **Ghostty pass for v0.5.0.** Record Ghostty version, geometry and transport,
   and check:
   - HEIC/AVIF/TIFF/RAW thumbnails;
   - progressive rows in a large photo gallery;
   - compressed images, including older placements after scrolling back;
   - `-C` versus `-x`;
   - red dangling links and `-` directory sizes;
   - plain files on a light theme;
   - the `--grid` notices.

   The steps are in [compatibility](docs/compatibility.md).
2. **README screenshot** from a real Ghostty session showing a mixed long view
   and a gallery.
3. **Carried over:** the v0.4.0 link click and scrollback check, and the v0.3.0
   default-details and AM/PM appearance.

## Open questions

- **Automatic grid threshold.** Grids start when at least half the entries are
  images. Now that long view carries miniatures, a higher threshold might suit
  mixed folders better. Decide from daily use.
- **Linux JPEG speed.** Linux decodes full-size JPEGs (parallel since v0.5.0).
  Reduced-size decoding (DCT scaling) or EXIF thumbnails would help photo
  folders there, and would lift the 16 MP limit for JPEG.
- **Other terminals.** WezTerm and Konsole implement Kitty graphics; iTerm2 has
  its own protocol; Sixel is widespread. Add any of them only with real-terminal
  evidence.
- **Video frames.** WebM/MP4 thumbnails remain deferred: they need a codec stack
  or an external process.

## Not planned

Recursive trees, Git status, file operations, a configuration file, a pager or
browser, a plugin system, or background services.

# Roadmap

Updated 2026-11-06 for v0.7.0 terminal-independent icons. lsa prints into scrollback,
returns to the shell and keeps one mixed ordering with complete names. History lives in the
[changelog](CHANGELOG.md); evidence in [compatibility](docs/compatibility.md)
and [measurements](benchmarks/README.md).

## Status

v0.7.0 removes the Ghostty-only choice of Nerd Font glyphs. Normal TTY output
uses Nerd Font icons across terminal brands, multiplexers and SSH sessions;
`--icon-style=portable` selects basic Unicode symbols. Visibility and graphics
detection remain independent. Released as
[v0.7.0](https://github.com/yungibly/lsa/releases/tag/v0.7.0), with the Homebrew
tap updated. Formatting, Clippy, 111 local Rust tests, 115 local layout/style
checks, four-platform CI, public-archive checks and Homebrew installs pass.

v0.6.0 implements practical `ls` replacement coverage: ordinary format/option
precedence, operand sorting, distinct `-a`/`-A`, recursive sections, explicit
symlink policies, and fuller long metadata. Natural ordering, readable sizes,
safe filename text and automatic previews remain product defaults. Recursion
shares the invocation's existing preview limits and detects ancestor cycles.
See [usage](docs/usage.md) for supported behavior and intentional differences.

Released as [v0.6.0](https://github.com/yungibly/lsa/releases/tag/v0.6.0), with
the Homebrew tap updated. Local tests, headless terminal checks, Ghostty's text
core, all four platform builds and published-package/Homebrew checks pass.
Evidence is recorded in
[compatibility](docs/compatibility.md) and application cost in
[measurements](benchmarks/README.md). The [v0.5.0 Ghostty demo](docs/demo.md)
remains the latest recorded image rendering evidence. On 2026-11-06 the user
reported completing the v0.6.0 checks successfully, then reported missing Nerd
Font icons in Rex. Exact version, geometry and transport were not supplied.

## Next

1. **Rex icon follow-up for v0.7.0.** Verify the default glyphs and the portable
   alternative in the user's font, and record terminal version and transport.
2. **Detailed terminal evidence.** The user reports the v0.6.0 checks work well;
   record version, geometry and transport when repeating or documenting:
   - fuller `-l` columns and totals beside miniatures, including narrow widths;
   - recursive sections and budget fallback, with earlier images in scrollback;
   - `-H`/`-L`/`-P`, `-a`/`-A`, and optional ACL/xattr rows;
   - HEIC/AVIF/TIFF/RAW thumbnails;
   - progressive rows in a large photo gallery;
   - compressed images, including older placements after scrolling back;
   - `-C` versus `-x`;
   - red dangling links and `-` directory sizes;
   - plain files on a light theme;
   - the `--grid` notices.

   The steps are in [compatibility](docs/compatibility.md).
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

Interactive trees, Git status, file operations, a configuration file, a pager or
browser, a plugin system, or background services.

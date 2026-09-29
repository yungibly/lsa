# Changelog

User-facing changes per release. The release workflow publishes each version's
section as its GitHub release notes. Measurements and their conditions are in
[benchmarks/README.md](benchmarks/README.md).

## v0.5.0 — 2026-09-28

**Faster, leaner galleries; iPhone photos on macOS; familiar `ls` flags.**

Previews

- Galleries decode on up to four worker threads while output keeps listing
  order: 64 photos take 0.38 s instead of 2.75 s.
- Fixed memory growing with every decoded JPEG above 4 MiB (about 8 MiB kept
  per photo on macOS): a 64-photo gallery peaked at 534 MiB and now uses 53 MiB.
- On macOS, **HEIC/HEIF, AVIF, TIFF, JPEG XL, Photoshop and camera RAW** files
  preview through the system's ImageIO decoders, loaded only when previewing.
  JPEGs decode directly at thumbnail size, so photos above 16 MP preview instead
  of showing error artwork. Linux keeps the bundled Rust decoders.
- Images are sent zlib-compressed (Kitty `o=z`) when smaller. Built-in folder and
  file artwork is about 30× smaller (a 256-folder grid sends 0.3 MB, not 7.9 MB).
- Dangling symlinks get broken-link artwork and no longer spend a preview attempt.
- Automatic graphics stay off in SSH sessions, like clickable links;
  `--protocol=kitty` still forces them.
- An explicit `--grid` that the terminal cannot show now prints why.

Listings

- `-C` fills columns top to bottom, like `ls`; the new `-x` fills rows across.
- New flags: `-p`, `-v`, `-i`, `-s`, `-o`, `-g`, `-c` and `-u`. `-R` explains
  that lsa lists one directory level.
- Ordinary files keep the terminal's default foreground instead of palette
  white, which was nearly invisible on light themes.
- Dangling symlinks are red, honoring LS_COLORS `or` and `mi`.
- Directory sizes show `-`; with `-S`, directories follow sized entries.
- LS_COLORS accepts up to 4,096 rules (previously 128, which dropped the later
  rules of generated palettes), with a case-insensitive suffix fallback.
- Clickable links no longer contain `/./` when listing the current directory.
- Plain names in a pipe are about 6% faster with a third less memory; long
  listings in a terminal are about 11% faster with 13% less output.

Notes

- The opt-in thumbnail cache moves to `lsa-thumbnails-v3`; older namespaces are
  left alone.
- `--diagnose` reports decode workers and the system decoder.
- Automated checks cover four native targets. The text layer was also checked
  in Ghostty's terminal core (libghostty-vt). A real Ghostty visual pass is
  pending.

## v0.4.0 — 2026-09-22

**Clickable filenames by default.**

- Local terminal listings link filenames with OSC 8, including details, compact
  columns and wrapped thumbnail labels. Pipes, redirected files, dumb or unset
  `TERM` and detected SSH sessions stay plain.
- `--hyperlink=auto|always|never` controls links; bare `--hyperlink` forces them
  and `--no-hyperlink` disables them. `NO_COLOR` affects color only.
- File URLs include the local hostname and percent-encode raw filename bytes;
  symlinks keep their own paths.

## v0.3.1 — 2026-09-12

**Less formatting and artwork work.**

- Long listings reuse formatting storage and borrow cached account names; labels
  stream without assembling a combined string.
- Artwork rasterizes polygon spans per row with pixel-identical output.
- About 4–7% less CPU for styled and long listings, 4.8–7.5% less for artwork.

## v0.3.0 — 2026-09-10

**Everyday details by default.**

- Terminal text defaults to long form: permissions, readable size, owner and local
  modification time. Image-heavy and small mixed listings still use grids; pipes
  print plain names.
- `-C` / `--columns` selects compact columns; `--12-hour` shows AM/PM times.
- Smaller metadata records and one local-time conversion per entry.

## v0.2.0 — 2026-09-07

**Larger galleries, adjustable thumbnails, SVG and ICO previews.**

- Up to 256 previews by default; `--preview-limit=0..4096`.
- `--thumbnail-size=1..12` sets grid height in terminal rows.
- Bounded SVG vector previews (shapes, paths, gradients, clipping) and ICO.
- Explanations when options override `--grid`; fewer preview copies; JPEG
  rotation after thumbnailing. The opt-in cache moved to its v2 namespace.

## v0.1.0 — 2026-09-06

First public release: colored, icon-decorated listings with bounded inline Kitty
thumbnails in direct Ghostty/Kitty sessions, natural sorting, an opt-in bounded
thumbnail cache, native builds for macOS and Linux, and a Homebrew tap.

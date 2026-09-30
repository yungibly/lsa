# Listings, thumbnails and limits

[Back to the README](../README.md) · [Installation](install.md) · [Compatibility](compatibility.md)

## Listings

- Terminals default to long details: permissions, readable size, owner and local
  modification time. Directories show `-` for size. `-C`/`-x` give compact
  columns, `-1` plain lines; `--header`, `--fields=LIST`, `--12-hour`, `--bytes`
  adjust details.
- Familiar flags work: `-a -A -l -C -x -1 -d -F -p -h -n -o -g -i -s -t -S -r -U
  -v -c -u`, plus `--sort`, `--dirs-first` and `--`. Recursive listing (`-R`) is
  deliberately not supported.
- Names sort naturally (`photo2` before `photo10`), ignoring ASCII case.
- Colors use your terminal's palette; ordinary files keep its default
  foreground. `LS_COLORS` type keys (including `or`/`mi`) and `*suffix` rules
  apply, with a case-insensitive fallback. Dangling symlinks are red. `NO_COLOR`
  and `--color=auto|always|never` are honored.
- Icons: Nerd Font glyphs in Ghostty, portable symbols elsewhere (`--icons`,
  `--no-icons`).
- Names are clickable (OSC 8) on local terminals; `--hyperlink=auto|always|never`.
- Pipes get complete plain names, one per line. Control characters and invalid
  UTF-8 are escaped everywhere; no name is shortened.

## Thumbnails

Images, files, folders and links share one ordering: a thumbnail represents its
entry, and a failed preview never hides a name.

- In a direct Ghostty or Kitty session, listings where at least half the entries
  are images, or that fit one row, become a thumbnail grid. Other listings show
  one-row miniatures beside names in the long view. Multiplexers, SSH sessions
  and unknown terminals use text unless `--protocol=kitty` forces graphics.
- Every tile has pixels: a thumbnail, or built-in folder, file, media, link,
  broken-link or error artwork.
- Formats: PNG, JPEG (with EXIF orientation), GIF (first frame), WebP, BMP, ICO
  and a restricted, self-contained SVG subset. On macOS, HEIC/HEIF, AVIF, TIFF,
  JPEG XL, Photoshop and camera RAW files also preview, using the system's
  ImageIO decoders (loaded only when previewing). Video frames are not decoded.
- Up to four decoders work ahead in parallel; output keeps listing order.
- `--thumbnail-size=1..12` sets grid height in rows (default 3).
  `--preview-limit=0..4096` sets attempts across all paths (default 256).
- Tall galleries print into scrollback. Once a budget is spent, remaining names
  continue as compact text.
- An opt-in thumbnail cache (`--cache-dir=PATH`) is bounded to 64 records and
  about 19 MiB; see [cache design](cache.md). `--diagnose PATH` explains the
  chosen layout, limits and decoders without decoding anything.

| Resource | Bound |
| --- | --- |
| Preview attempts | 256 by default, 4,096 at most; failures and cache hits count |
| Image commands | 128 MiB (counted uncompressed) and 4,096 placements per invocation |
| Decoding | Up to four sources at once, at most eight results ahead of output |
| Source file | 32 MiB for the Rust decoders; 256 MiB for macOS ImageIO, which reads only what it needs |
| Decoded image | 16 million pixels at full size, 64 MiB decoder allocations. JPEG, HEIF and AVIF decode at reduced resolution on macOS, up to 64× larger; bigger TIFF/PSD/JPEG XL/RAW use embedded previews |
| SVG | 256 KiB, 4,096 XML nodes, 32 levels; no text, external or embedded images, filters, masks, patterns, markers or `use` |
| Thumbnail | Grid at most 320×240 pixels; long view at most 96×64 |
| Cache | Opt-in; 64 records, under 20 MiB of file contents |

Exit codes: 0 success (including closed pipes), 1 listing, output or cache-clear
error, 2 invalid options.

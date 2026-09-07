# lsa

**ls, augmented.**

An everyday `ls` replacement with colors, icons, readable details, and inline image
thumbnails. Print the directory, get the prompt back, keep the output in scrollback.
Rust, Unix, one binary. No pager, browser, configuration file, or background process.

Install the latest release from the Homebrew tap:

```sh
brew install yungibly/tap/lsa
```

Or download a checksummed binary from [GitHub Releases](https://github.com/yungibly/lsa/releases).
Packages cover Apple Silicon/Intel macOS 14+ and ARM64/x86-64 Linux.
See [installation and releases](docs/install.md) for details. To build locally:

```sh
export CARGO_HOME="$PWD/.cargo-home"
cargo build --release --locked
./target/release/lsa                # compact, styled listing
./target/release/lsa -la            # hidden files and readable details
./target/release/lsa img-test       # automatic inline thumbnails
./target/release/lsa --no-images .  # compact text
./target/release/lsa -1 . | head    # plain names; closed pipes succeed
```

To try it as `ls` in the current shell, from this checkout:

```sh
alias ls="$PWD/target/release/lsa"
```

Version 0.2.0 includes larger preview budgets, adjustable grid size and SVG/ICO
previews. The user verified the changes in Ghostty on 2026-09-07; terminal version,
geometry and transport were not resupplied. Earlier inline image output was
user-verified in Ghostty 1.3.1. See [compatibility](docs/compatibility.md) and
[installation](docs/install.md).

## Everyday behavior

- Terminal output uses compact columns, colors from the terminal palette, and icons.
  Ghostty gets Nerd Font icons; other terminals get portable Unicode symbols.
  `--icons=always` selects Nerd Font icons elsewhere; `--no-icons` disables them.
- `--color=auto|always|never` controls color. Auto respects nonempty `NO_COLOR`.
  Common `LS_COLORS` file-type and literal `*suffix` rules customize the palette;
  values are bounded and validated as SGR codes. `TERM=dumb` disables auto styling.
- `-l` shows permissions, human-readable size, owner and local modification time.
  On graphics terminals, images get one-row miniatures beside their names without
  making entries taller. `--no-images` keeps ordinary icons. `--header` labels the columns. `--bytes` uses exact sizes; `-n` adds numeric uid/gid
  and link count. `--fields=mode,size,modified` chooses a smaller set of details.
- Names sort naturally: `photo2` before `photo10`, ignoring ASCII case with raw-byte
  tie breaks. Non-ASCII names retain byte ordering; this is not locale collation.
  `-t` / `-S` sort newest/largest first, `-r` reverses, `--dirs-first` groups actual
  directories, and `-U` keeps filesystem order.
- `-a` / `-A` include hidden names without `.` or `..`. `-d` lists directory operands
  themselves. A directory-symlink operand lists its contents, except with `-l` or
  `-d`; links inside a listing retain link identity. `-F` adds type markers.
- Multiple operands keep argument order; consecutive file operands share a layout.
  `--hyperlink` makes names clickable with OSC 8. Use `--` before dash-prefixed paths.
- Pipes default to complete plain names, one per line, with metadata only when
  requested. Explicit color/icons/hyperlink flags can decorate redirected text.
  Previews never go to a pipe. Text stays escaped for safe terminal display,
  including control characters and invalid UTF-8. No filename is shortened.

Full option reference: `lsa --help`. This is a human-oriented tool; it deliberately
omits recursive trees, Git queries, file operations, and exact GNU/BSD scripting parity.

## Images that fit the task

Images, files, folders, and links share one ordering. A thumbnail is a representation
of an entry, and a failed preview never removes its name.

On a direct Ghostty/Kitty terminal, previews are automatic when at least half the
entries are image candidates, or when a mixed listing fits a single thumbnail row.
A single image also previews automatically. `--grid` requests previews in a sparse
mixed directory, including ones containing only folders or ordinary files.
`-1`, `--no-images`, and `--protocol=none` keep text.

Grid frames default to **14 columns × 3 rows** at most. Set
`--thumbnail-size=N` to choose any height from **1 to 12 terminal rows**;
width and spacing follow the size, and frames shrink to fit short/narrow terminals.
Labels wrap completely and have no duplicate font icon. For example:

```sh
./target/release/lsa --grid --thumbnail-size=1 img-test  # dense gallery
./target/release/lsa --grid --thumbnail-size=6 img-test  # larger previews
./target/release/lsa --preview-limit=1024 img-test       # more source previews
```

Both size and preview limit accept `=N` or a separate argument. `--header`,
`--fields`, `-l` and `-n` select long output; combining these with `--grid` now
prints a brief explanation to stderr. Explicit text/image-disabling options also
take precedence over grid, regardless of order. Thumbnail size affects grids;
long output retains its one-row miniatures and reports an ignored size request.

Every tile has a thumbnail or
built-in folder, file, media, link or error drawing. GIFs share the image category
with PNG/JPEG; unsupported image formats get image artwork, and unknown extensions
get a generic file icon/color. Artwork is generated from code and needs no font.

Long view uses **3-column × 1-row** miniatures in a fixed gutter beside filenames.
Ordinary entries retain their icons, and failures get error artwork. When the
terminal is too narrow/short, output is redirected, or images are disabled, long
view stays text. Long filenames can still wrap normally. Small thumbnails have
smaller output payloads, but still require decoding the source image.

Tall galleries print into scrollback. At most **256 previews are attempted by default**,
shared across every operand. `--preview-limit=N` adjusts that from 0 to **4,096**. Once the
attempt, byte or placement budget is spent, the remaining entries print as compact
text in the same order. A partially previewed grid row uses artwork for its remaining
tiles; the next row becomes compact text. In long view, remaining entries keep the
same aligned name gutter. Small terminals, unsupported
terminals and multiplexers fall back to text; `--protocol=kitty` is an explicit
protocol override, not a compatibility guarantee. No terminal queries or input reads.

Static PNG/JPEG/GIF/WebP/BMP/ICO are supported, including JPEG EXIF orientation, aspect
ratio preservation, transparency checkerboarding, and GIF's first canvas frame.
SVG previews support self-contained shapes, paths, gradients and clipping, rasterized
directly at thumbnail size. Fonts/text, embedded or external images, filters, masks,
patterns, markers and `use` expansion currently fall back to error artwork for the
whole preview. SVG input is limited to 256 KiB, 4,096 XML nodes and 32 nesting levels;
DTDs/entities are rejected. No external resources or fonts are loaded.

Image symlinks can preview after verifying a bounded regular-file target. HEIC/AVIF
retain image artwork. WebM and other videos retain media artwork; video frame
decoding is deferred to avoid a native codec stack or external process dependency.

| Resource | Bound |
| --- | --- |
| Preview attempts | 256 default, 4,096 maximum; failures and cache hits count |
| Image commands | 128 MiB and 4,096 placements per invocation, including built-in artwork |
| Raster source | 32 MiB, 16 million pixels, 16,384 pixels per axis |
| Raster decoded image / decoder allocation | 64 MiB; decoder allocation bound is best effort |
| SVG source | 256 KiB, 4,096 XML nodes, 32 nesting levels; restricted features as above |
| Thumbnail | Grid at most 320×240 pixels; long view at most 96×64 pixels |
| Work in flight | One sequential decoder and thumbnail; no worker or queue |
| Optional cache | 64 slots, under 20 MiB of file contents; no scans or hit writes |

Text output and the entry vector scale with directory size. Filesystem, account-name
lookup, decoder and terminal writes have no hard timeout. Styling reads regular-file
mode bits for executable icons/colors but retains no full metadata per entry. Plain
name-only output avoids those stats. Long output caches bounded account names and
exact timestamps. [Measurements](benchmarks/README.md) distinguish application cost
from terminal rendering and warm from empty thumbnail caches.

Raster sources use a bounded buffered reader, avoiding an application copy of the
entire compressed file (some codecs still buffer internally). JPEG orientation is
applied to the thumbnail, avoiding full-resolution rotation. Kitty encoding uses
4 KiB of reusable scratch space. Raising the output allowance does not preallocate
it; bytes stream as each entry finishes. The allowance fits all 256 default source
previews even at maximum thumbnail resolution, with room for artwork in the last row.

Caching stays **off by default**. Enable explicitly with `--cache-dir=PATH`; use
`--no-cache` to bypass it or `--clear-cache --cache-dir=PATH` to clear known records.
`--cache-stats` reports counters. Text, diagnostics, and zero budgets never open the
cache. See [cache design](docs/cache.md). `--diagnose PATH` explains layout and limits
without decoding or accessing cache storage.

Exit codes: 0 success (including closed pipes), 1 listing/output/cache-clear error,
2 invalid options. Preview/cache failures are quiet unless cache statistics are requested.

## Development

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --release --locked --examples --bins
./target/release/examples/fixtures  # creates ignored fixtures once
python3 tests/check_pty.py
python3 tests/check_layout.py
python3 tests/check_cache.py
python3 tests/check_thumbnails.py
python3 tests/check_gallery.py
python3 benchmarks/inline.py
./target/release/examples/gallery_fixtures  # once: SVG/ICO/EXIF visual fixtures
python3 benchmarks/gallery.py --before target/lsa-before-gallery-options
./target/release/examples/preview_sheet  # offline artwork QA PNG under target/
python3 scripts/package.py
python3 -m unittest discover -s tests -p 'test_release.py' -v
```

Keep Cargo storage and test artifacts in this repo. The user handles real-terminal
visual verification; PTY tests model bytes and cursor positions, not a renderer.
See [roadmap](ROADMAP.md), [decisions](docs/decisions.md), and [handoff](HANDOFF.md).

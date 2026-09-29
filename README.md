# lsa

**ls, augmented.** An everyday `ls` replacement with colors, icons, readable
details, clickable names and inline image thumbnails. It prints into your
terminal's scrollback and returns to the shell: no pager, browser, configuration
file or background process. Rust, one binary, macOS and Linux.

## Install

```sh
brew install yungibly/tap/lsa
```

Or download a checksummed binary from [GitHub Releases](https://github.com/yungibly/lsa/releases)
for Apple Silicon/Intel macOS 14+ or ARM64/x86-64 Linux. See
[installation](docs/install.md). To use it as `ls`, add `alias ls=lsa` to your
shell configuration.

## Everyday use

```sh
lsa                        # details, or a thumbnail grid where images dominate
lsa -la                    # include hidden files
lsa -C                     # compact columns filled downward (-x: across)
lsa -lt                    # newest first
lsa photos                 # thumbnails in Ghostty or Kitty
lsa --grid --thumbnail-size=6 photos   # larger thumbnails
lsa | head                 # plain names in pipes
```

`lsa --help` lists every option.

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
  about 19 MiB; see [cache design](docs/cache.md). `--diagnose PATH` explains the
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

## Development

```sh
export CARGO_HOME="$PWD/.cargo-home"
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo build --release --locked --examples --bins
./target/release/examples/fixtures          # once: creates img-test/generated
for check in pty layout cache thumbnails gallery; do python3 tests/check_$check.py; done
python3 -m unittest discover -s tests -p 'test_release.py'
python3 scripts/package.py
```

`tests/check_ghostty_vt.py` optionally checks the text layer in Ghostty's own
terminal core. It uses the [tui-test](https://github.com/microsoft/tui-test)
release binary placed at `target/tools/tui-test/tui-test`. Headless checks model
terminal bytes and cursor movement; images still need a real-terminal look.

Keep Cargo storage and test artifacts inside the repository. More detail:
[changelog](CHANGELOG.md), [roadmap](ROADMAP.md), [decisions](docs/decisions.md),
[compatibility](docs/compatibility.md), [measurements](benchmarks/README.md).

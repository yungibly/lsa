# lsa

A fast, human-oriented directory listing with inline image thumbnails. An explicit
browser can follow. Kitty graphics first; Sixel is an optional future addition.

**Status:** Rust CLI with compact text and automatic inline Kitty grids. The
original inline renderer is user-verified in Ghostty 1.3.1; the new default layouts
pass automated checks and await a visual check.
See [ROADMAP.md](ROADMAP.md) and [compatibility](docs/compatibility.md).

## Direction

- One sorted collection of images, files, folders, links, and special files.
  A preview changes an entry's representation, never its membership or ordering.
- Inline output returns to the shell and remains useful in history. Browsing is
  explicit; automatic paging, if added, is opt-in.
- Names and metadata are terminal text. Unsupported images and preview failures
  retain a readable entry. Piped output is complete plain text.
- Make the ordinary text path cheap: no content reads, graphics queries, eager
  metadata, cache initialization, or background work unless needed.
- Bound thumbnail input, dimensions, allocations, work, and transmitted bytes.
  Listing every name is more important than previewing every image.
- Preserve aspect ratio, orientation, and transparency. No required icon font.

The docs are working design notes, not fixed requirements. Prefer a measured,
useful slice over speculative interfaces. Exact BSD/GNU `ls` parity, file mutation,
indexing services, plugins, animation, and broad preview providers are outside the
initial scope.

## Run

```sh
# Keep Cargo downloads and build outputs inside this repository.
export CARGO_HOME="$PWD/.cargo-home"
cargo build --release --locked
./target/release/lsa
./target/release/lsa -lah img-test
./target/release/lsa --grid img-test
./target/release/lsa --grid --protocol=kitty img-test
./target/release/lsa --diagnose
```

Rust 1.88+ declared; built/tested with 1.98.0. Unix only; Linux is not yet tested.
No install step or external image program is needed. The release binary is about
1.2 MiB on this Mac. Full option semantics: `lsa --help`.

Supported: `-a`/`-A`, `-l`, `-h`, `-1`, `-t`, `-S`, `-r`, multiple operands, `--`,
`--grid`, `--no-images`, `--protocol=auto|kitty|none`, `--preview-limit=0..256`,
`--diagnose`. Text flags override grid regardless of option order. Graphics
selection uses Ghostty/Kitty environment hints; unknown terminals/multiplexers
fall back to text. Non-TTY output always stays text, including with an explicit
override. `--diagnose PATH` reports the chosen layout, reason, candidate count,
and estimated grid height without decoding images.
No terminal queries, stdin reads, paging, color, configuration, or cache yet.

Deliberate `ls` differences: bytewise name order; `-a` and `-A` both omit `.`/`..`;
directory/link/FIFO/socket suffixes; numeric uid/gid and local minute timestamps in
long output; directory symlink operands remain links. Time/size sorts use names to
break ties. Reverse applies to the full order; operands retain argument order.
Exit codes: 0 success (including broken pipe), 1 listing/output error, 2 bad options.

## Default layout

Each operand chooses its layout once, after hidden filtering and sorting:

- On a TTY, use aligned text columns, reading across each row. Widths include
  Unicode and escaped controls. Keep a two-space gap and leave the rightmost cell
  free; fall back to one column when names are too wide. Names are never shortened.
- Automatically use a grid only with Kitty enabled, at least four preview
  candidates, at least half the entries eligible, and the full grid fitting within
  terminal height minus two rows. Wrapped labels count toward that height. Check
  candidates against the configured attempt and byte caps before decoding.
- `--grid` bypasses those heuristics. `--no-images` and `--protocol=none` force
  compact text; `-1` forces one entry per line; `-l` forces long text. Pipes use one
  entry per line (long metadata remains available with `-l`).

The four user images plus `generated/` choose a grid at 122×40. The 40-image fixture
chooses text; use `--grid` to preview it. These thresholds are provisional. Candidate
extensions/types are cheap hints: decode failures never change the chosen layout.
The preview budget is shared across operands, so later grids may reach the limit.
Text planning retains only widths, with a search capped at 64 columns.

## Previews and limits

Static PNG/JPEG/GIF/WebP/BMP, JPEG EXIF orientation, aspect-preserving letterboxing,
and a neutral checker under transparency. GIF uses the first frame on its logical
canvas. Symlinks to regular images can preview while retaining their link marker.
Names wrap at grapheme boundaries; missing previews keep their tiles and labels.

One synchronous decode at a time; no worker queue or disk cache. Per invocation:
64 preview attempts by default (including failures), hard maximum 256, and 8 MiB
of image commands. Per source: 32 MiB input, 16 million pixels, 16,384 pixels per
axis, 64 MiB decoded output, and a **best-effort** 64 MiB decoder allocation limit.
Thumbnails fit within 320×240 pixels. These are starting caps, not performance
claims or a hard process-memory/time sandbox. Preview work can delay a row; there
is no timeout for slow filesystems or in-progress decodes. Names are never omitted
after preview limits; stderr reports unavailable/limited previews.

Grid requires at least 12 columns × 8 rows. Cell pixels come from the terminal's
window size; if missing, 8×16 is estimated and aspect may be imperfect. Geometry is
chosen once per invocation; resize during output is not handled. Detailed resize,
theme, and retention trials are not yet recorded. Inline images use anonymous placements and are left in terminal
history; no global image deletion. Terminal storage may evict old previews.

## Check

```sh
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --release --locked --examples
./target/release/examples/fixtures  # creates img-test/generated once
python3 tests/check_pty.py
python3 tests/check_layout.py
python3 benchmarks/measure.py
```

Tests keep scratch files under `target/`; fixtures and measurements are gitignored.
The socket integration test needs local Unix-socket creation permission. Invalid
UTF-8 filesystem names are tested on Linux; APFS rejects them, so macOS uses the
byte-level display tests. See [decisions](docs/decisions.md) and
[measurements](benchmarks/README.md) for evidence and remaining limits.

## Later slices

1. Visually check the new defaults in Ghostty and tune them from daily use.
2. Improve latency from actual directories: thumbnail cache and bounded parallel
   work only where measurements justify them.
3. Add an alternate-screen browser with viewport-driven previews, stable
   selection, stale-job rejection, and session-only image cleanup. Share entries
   and decoding with inline output, not output lifetime assumptions.
4. Package tested macOS/Linux targets; expand formats and terminals from demand.

See [AGENTS.md](AGENTS.md) for development practice.

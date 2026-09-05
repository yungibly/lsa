# lsa

A fast, human-oriented directory listing with inline image thumbnails. An explicit
browser can follow. Kitty graphics first; Sixel is an optional future addition.

**Status:** Rust CLI with compact text and automatic inline Kitty grids, both
user-verified in Ghostty 1.3.1. Directories-first sorting, configurable long
metadata, and opt-in cache commands also received a user pass at `7050349`.
Improved cache replacement has automated tests and working-set measurements.
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
./target/release/lsa -lah --dirs-first img-test
./target/release/lsa --fields=size,modified -h img-test
./target/release/lsa --grid img-test
./target/release/lsa --grid --protocol=kitty img-test
./target/release/lsa --diagnose
```

Rust 1.88+ declared; built/tested with 1.98.0. Unix only; Linux is not yet tested.
No install step or external image program is needed. The release binary is about
1.2 MiB on this Mac. Full option semantics: `lsa --help`.

Supported: `-a`/`-A`, `-l`/`--long`, `-h`, `-1`, `-t`, `-S`, `-r`,
`--dirs-first`, `--fields=LIST`, multiple operands, `--`,
`--grid`, `--no-images`, `--protocol=auto|kitty|none`, `--preview-limit=0..256`,
`--diagnose`. Text flags override grid regardless of option order. Graphics
selection uses Ghostty/Kitty environment hints; unknown terminals/multiplexers
fall back to text. Non-TTY output always stays text, including with an explicit
override. `--diagnose PATH` reports the chosen layout, reason, candidate count,
and estimated grid height without decoding images.
Caching is opt-in via `--cache-dir=PATH`; `--no-cache` disables it and
`--clear-cache` clears the selected cache. No terminal queries, stdin reads,
paging, color, or configuration file yet.

Deliberate `ls` differences: bytewise name order; `-a` and `-A` both omit `.`/`..`;
directory/link/FIFO/socket suffixes; numeric uid/gid and local minute timestamps in
long output; directory symlink operands remain links. Time/size sorts use names to
break ties. Reverse applies to the full order unless directories-first is enabled,
when it reverses within each group. Operands retain argument order.
Exit codes: 0 success (including broken pipe), 1 listing/output/cache-clear error,
2 bad options. Ordinary cache I/O failures do not fail a listing.

## Sorting and metadata

`--dirs-first` places actual directories first in every layout, without adding
metadata reads. `-r`, `-t`, and `-S` operate within each group. Symlinks, including
links to directories, stay in the non-directory group; grouping does not resolve
their targets. This remains one ordered listing with no separate image section.

`-l` / `--long` show mode, link count, numeric uid/gid, size, and local modification
time. `--fields=size,modified` chooses metadata columns and their order, and implies
long text output. Valid fields: `mode`, `links`, `uid`, `gid`, `size`, `modified`.
Duplicates, empty lists, and unknown fields are errors. Names always follow the
chosen columns; symlinks retain their target text. `-h` formats sizes in binary units.

Long columns align to the displayed values rather than fixed minimum widths.
Missing metadata shows `?` without dropping the entry. Only long output or time/size
sorting requests per-entry metadata; default text and directories-first remain
free of those extra reads. Extended attributes/ACLs, account-name lookup, and
metadata beneath image tiles are not implemented.

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
  compact text; `-1` forces one entry per line; `-l` / `--fields` force long text. Pipes use one
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

One synchronous decode at a time; no worker queue. Optional cache hits skip decoding.
Per invocation: 64 preview attempts by default (including failures), hard maximum
256, and 8 MiB of image commands. Per source: 32 MiB input, 16 million pixels,
16,384 pixels per axis, 64 MiB decoded output, and a **best-effort** 64 MiB decoder allocation limit.
Thumbnails fit within 320×240 pixels. These are starting caps, not performance
claims or a hard process-memory/time sandbox. Preview work can delay a row; there
is no timeout for slow filesystems or in-progress decodes. Names are never omitted
after preview limits; stderr reports unavailable/limited previews.

Grid requires at least 12 columns × 8 rows. Cell pixels come from the terminal's
window size; if missing, 8×16 is estimated and aspect may be imperfect. Geometry is
chosen once per invocation; resize during output is not handled. Detailed resize,
theme, and retention trials are not yet recorded. Inline images use anonymous
placements and are left in terminal history; no global image deletion. Terminal storage may evict old previews.

## Optional thumbnail cache

The experiment is **off by default** and has no implicit storage location. Try it
inside this repository; run the first command twice to compare misses and hits:

```sh
./target/release/lsa --cache-dir=benchmarks/local/thumbnails --cache-stats img-test
./target/release/lsa --cache-dir=benchmarks/local/thumbnails --no-cache img-test
./target/release/lsa --cache-dir=benchmarks/local/thumbnails --clear-cache
```

The cache stores thumbnail pixels in 64 slots, with eight candidate slots per key,
versioned source identity/timestamp/size/geometry keys, checksums, atomic writes,
and less than 20 MiB of file contents including staging. Full groups randomly
evict a slot; storage errors fall back to decoding. Text output, diagnostics, and exhausted preview budgets
never open storage. Hits still consume preview and output budgets.

On this Mac, the four user images took **4.60 ms warm versus 71.76 ms uncached**
through a drained PTY, with identical output and ~2 MiB versus ~31 MiB application
RSS. These measure application work and PTY transport, not visible terminal rendering.
See [cache behavior and limits](docs/cache.md) and [measurements](benchmarks/README.md).
The larger-working-set follow-up improved a repeated 32-source listing from
216 ms to 16 ms by reducing collisions, with the same storage and output limits.

## Check

```sh
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --release --locked --examples --bins
./target/release/examples/fixtures  # creates img-test/generated once
python3 tests/check_pty.py
python3 tests/check_layout.py
python3 tests/check_cache.py
python3 benchmarks/measure.py
python3 benchmarks/cache.py
python3 benchmarks/cache_working_set.py
```

Tests keep scratch files under `target/`; fixtures and measurements are gitignored.
The socket integration test needs local Unix-socket creation permission. Invalid
UTF-8 filesystem names are tested on Linux; APFS rejects them, so macOS uses the
byte-level display tests. See [decisions](docs/decisions.md) and
[measurements](benchmarks/README.md) for evidence and remaining limits.

## Later slices

1. Tune layouts and metadata controls from daily use.
2. Evaluate the opt-in cache from daily use before choosing a default policy;
   add bounded parallel work only where measurements justify it.
3. Add an alternate-screen browser with viewport-driven previews, stable
   selection, stale-job rejection, and session-only image cleanup. Share entries
   and decoding with inline output, not output lifetime assumptions.
4. Package tested macOS/Linux targets; expand formats and terminals from demand.

See [AGENTS.md](AGENTS.md) for development practice.

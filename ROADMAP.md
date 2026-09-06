# Roadmap

Updated 2026-09-05. Working sequence; change it as evidence arrives.

## Current state

Rust CLI on arm64 macOS 26.6.2, built with Rust/Cargo 1.98.0. Inline Kitty output
was user-verified in Ghostty 1.3.1 at 122×40 cells, 8×17 pixels per cell. Compact
columns and automatic grid selection also received a clear user visual pass at
`364976f`. The user also passed all supplied metadata/grouping and cache commands
at `7050349`, in the previously reported Ghostty context. Cache replacement now
uses eight candidates per key after independent-source/geometry measurements;
that storage change has automated output-equivalence and compatibility coverage.
Four user images and generated fixtures remain gitignored under `img-test/`.
The user passed the `--browse` text slice at `1ea10e5`, in the previously reported
Ghostty context. The subsequent image-browser review was negative: poor spatial
controls and visual design, and scope beyond an `ls` replacement. The user chose
to replace browsing with a simple image pager. `--page` is implemented; its visual
check remains pending. Historical browser measurements are retained separately.

## 1. Useful inline prototype — complete for the tested Ghostty commands

- [x] Inspect workspace/toolchain; narrow graphics scope to Kitty.
- [x] Implement paths, hidden filtering, deterministic ordering, basic long
  output, safe filename display, partial errors, and normal closed-pipe behavior.
- [x] Keep redirected/text output free of graphics and content reads.
- [x] Build one mixed-entry grid, bounded static decoding, quiet chunked Kitty
  transmission, and visible placeholders for failures or exhausted budgets.
- [x] Test CLI behavior, special files, invalid-byte display, image limits, protocol framing,
  and headless terminal output. Generate a reproducible mixed fixture.
- [x] Record application timing, memory, payload size, and dependency choices.
- [x] User verified all supplied Ghostty commands: mixed/full/limited grids,
  diagnostic output, and pipes. Exact version and geometry recorded. Detailed
  resize/theme/transport trials remain follow-up compatibility work.

Evidence: [compatibility report](docs/compatibility.md), original application
[baseline](benchmarks/README.md), generated fixtures, and protocol/CLI tests.

## 2. Daily inline use — layouts user-verified; metadata and opt-in cache implemented

- [x] Compact row-wise text columns, Unicode display widths, complete filenames,
  narrow-window fallback, and stable ordering.
- [x] Conservative automatic grid selection from count/proportion, shared grid
  geometry, wrapped label height, and configured preview budgets; no content reads
  to choose a layout. Text and explicit grid overrides remain available.
- [x] Per-path diagnostics explain layout selection without decoding images.
- [x] 21 unit + 6 CLI tests, 16 original PTY + 22 new layout checks; fmt/clippy clean.
- [x] Benchmark default paths at 122×40; preserve original baseline for comparison.
  Piped 10,000-name listing remains ~6.7 ms; compact PTY output ~9.8 ms / 3.69 MiB.
- [x] User reported all new default commands look good and visual testing is a
  clear pass. Computer Use cannot access terminal emulators; user checks suffice.
- [x] Add `--dirs-first`, preserving groups under reverse/time/size sorting.
  Directory symlinks keep link identity; grouping adds no per-entry metadata reads.
- [x] Add `--long` and `--fields=LIST`, aligned metadata columns, complete names,
  and placeholders for unavailable metadata. Long output remains text-only.
- [x] 23 unit + 8 CLI tests, 16 original PTY + 34 layout scenarios pass; release
  build, fmt, and clippy pass. Metadata commands subsequently received a user pass
  with the `7050349` checklist.

[Default measurements](benchmarks/defaults.json) separate sink, explicit grid,
and automatic TTY output. [Metadata measurements](benchmarks/metadata.json) compare
against `364976f`: plain 10,000-name output stays ~6.8 ms; dynamic long alignment
costs ~24.9 ms versus ~21.6 ms for the previous fixed-width output, at ~5.3 MiB RSS.
No cache, concurrency, or browser was added in the metadata chunk.

### Opt-in thumbnail cache — implemented and measured

- [x] `--cache-dir=PATH`, `--no-cache`, `--clear-cache`, and `--cache-stats`;
  no default storage path. Text, diagnose, and exhausted preview budgets never
  access storage. Preview attempts/output bytes still bound hits.
- [x] Versioned device/inode/size/mtime/ctime/pixel-geometry keys, exact key checks,
  RGBA records with checksums, and atomic writes. Validate source access/type/size
  before lookup and recheck fd metadata before accepting hits or inserting.
- [x] 64 replaceable slots, one staging file, and a permanent lock. Under 20 MiB
  of file contents, no directory scans or hit writes. Nonblocking locks bound
  concurrent storage work; cache failures fall back to decoding.
- [x] Exercise metadata invalidation, replacement/retargeting, corruption,
  geometry, read-only storage, concurrent writes, contention, eviction, stale
  staging, safe clear, and existing preview limits.
- [x] 30 unit + 8 CLI tests, 16 original PTY + 34 layout + 23 cache scenarios;
  release build, fmt, and clippy pass. The user subsequently passed all supplied
  Ghostty cache commands at `7050349`.
- [x] [Saved cache measurements](benchmarks/cache.json): four user images at
  122×40 take 71.76 ms uncached, 71.82 ms with empty cache, and 4.60 ms warm
  (~15.6× faster), with 31.36 → 2.03 MiB application RSS and identical output.
  Invalidation returns to 72.34 ms. OS cache warm; no terminal renderer measured.
  Plain 10,000-name output stays ~7.2–7.3 ms in a same-session before/after check.

### Cache working sets — measured; replacement improved

- [x] Compare 4/16/32/48/64/96 independent sources with repeated and alternating
  176×85/176×80 thumbnail geometry. Keep original images and fixture identities
  unchanged across direct/four-candidate/eight-candidate binaries.
- [x] [Saved comparison](benchmarks/cache-working-set.json): 32 sources at fixed
  geometry improve from 56.25% hits / 216.42 ms to 100% / 15.97 ms; 16 sources
  alternating sizes improve from 62.50% / 98.74 ms to 100% / 9.01 ms. Four warm
  sources remain ~4.76 ms. All output digests match; no cache I/O errors measured.
- [x] Eight candidate slots per key, 64 slots total. Missing slots are filled
  before replacing entries; a randomly seeded key hash chooses a victim in full
  groups. Bounded header reads, no hit writes, same storage/locking/pixel format.
- [x] Existing v1 records remain safe to reuse; unchanged namespace/lock/slot names
  keep storage bounded while old placements become normal misses. No migration scan.
- [x] 32 unit + 8 CLI tests, 16 protocol + 34 layout + 23 cache scenarios, release,
  fmt, and clippy pass. New coverage includes coexistence/full-group eviction,
  duplicate prevention, malformed candidates before hits, and prior slot mappings.

The experiment stays opt-in. A 96-source alternating-size listing needs 192 keys
and sees little benefit from 64 slots. Random replacement and unequal decode costs
make individual latency/hit rates vary; eight candidates are not fastest in every
overloaded case. [Cache design](docs/cache.md) records limits and tradeoffs. No
worker concurrency or browser was added in the cache chunks.

## 3. Explicit image pager — replaces the browser experiment

The browser's text slice at `1ea10e5` received a user pass; image work at `59f5cec`
had automated coverage and application measurements. The user's visual review
rejected its presentation and directory-navigation scope. Neither the old graphics
nor the replacement pager has a positive visual pass.

- [x] Replace `--browse` with `--page [DIRECTORY]`; the old option gives a migration
  error. Read one sorted, hidden-filtered mixed listing once. Remove directory
  navigation, symlink traversal actions, history, refresh, and revision state.
- [x] Spatial arrows/hjkl: left/right stay within a row; up/down move one grid row.
  Space/PgDn and b/PgUp page; Home/End or g/G jump. Enter inspects any entry's full
  escaped name/path; Esc/Backspace closes inspection; q quits.
- [x] Consistent tile-width filename selection, centered text placeholders, a
  selected-name status line, and shorter pager controls. Keep names terminal text.
- [x] Retain viewport previews, one lazy decoder, one outstanding job/completion,
  stale-work rejection, at most 32 placements and 34 records, and opt-in cache.
- [x] Renew attempt/output budgets only when the viewport changes. Count retained
  records against the new attempt allowance and reserve resident-image cleanup.
  A long listing can preview beyond the former whole-session 64-attempt/8-MiB caps;
  each viewport remains bounded. Selection-only redraws cannot reset the caps.
- [x] Retain separate inline/pager lifetimes, targeted Kitty cleanup, resize,
  raw-mode restoration, quit/signals/suspend, text fallback, and idle blocking.
- [x] 39 unit + 8 CLI tests, 25 text-pager + 23 image-pager scenarios, and the
  existing 16 protocol + 34 layout + 23 cache scenarios pass. fmt/clippy/release
  pass. Coverage includes all 100 previews beyond the old lifetime caps, bounded
  output within an oversized viewport, and selection not renewing budgets.
- [ ] User verifies the [pager in Ghostty](docs/compatibility.md#pager-checklist).

There is no search, directory navigation, file launching, copying, or rich-TUI
roadmap. Initial directory enumeration and terminal output remain synchronous;
a codec or filesystem call cannot be cancelled, although quit never joins it.
Snapshot refers to listing membership/order; image contents are read when needed.
Preview failures never hide entries. Ordinary inline output is unchanged.

## 4. Package and expand — pending

Verify installation on selected macOS/Linux versions. Publish exact local/SSH/
multiplexer compatibility, format limits, and cold/warm measurements. Consider
SVG, TIFF, AVIF, HEIC, Sixel, or other protocols only from concrete demand.

## Evidence standards

- Automated: empty/mixed/large directories, all entry types, hidden/Unicode/invalid
  byte/control names, operands and `--`, sorting, partial failures, pipes, corrupt
  images, excessive dimensions, byte/attempt limits, and protocol framing.
- Terminal: terminal/version, OS, protocol, mode, transport, reproduction command,
  visible outcome. Upstream support is not lsa validation.
- Performance: separate application from terminal cost and OS cache from thumbnail
  cache. Record fixture, build, geometry, elapsed time, peak memory, and bytes.
  No GUI superiority claims from sink benchmarks.

## Next task

Review the simple pager in Ghostty using a large image directory and a mixed one.
Check whether page scrolling and spatial selection serve the original need,
along with name inspection, resize, cleanup, and prior inline-image retention.
Fix concrete issues within that scope; do not resume the browser/search roadmap.
Return to everyday inline use and packaging after that review. Keep the single
decoder and opt-in cache unless measurements or usage justify a change. Linux,
other terminals/transports, and the declared minimum Rust version remain unverified.

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
Ghostty context. Viewport-driven browser previews now have automated coverage and
application/PTY measurements; their Ghostty visual check is pending.

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

## 3. Explicit browser — text user-verified; images implemented, visual check pending

- [x] `--browse` for one directory, alternate screen, mixed-entry selection,
  arrows/j/k, paging, first/last, directory navigation, and explicit refresh.
  Share existing entries, hidden filtering, sorting, and suffixes.
- [x] Preserve raw-name selection on refresh/return and index fallback on removal.
  Resolve directory links only on navigation; retain up to 64 return bookmarks.
- [x] Full escaped name/path inspection with grapheme wrapping and scrolling;
  selection survives resize, including temporarily unusable terminal dimensions.
- [x] Separate browser terminal guard: raw input, cursor and alternate-screen
  restoration, Ctrl-C and exit signals, Ctrl-Z/continue, and foreground TTY checks.
  Reject pipes before consuming input; inline output never enters this lifecycle.
- [x] Blocking `pselect` while idle, bounded input/escape buffers and redraw geometry,
  no background refresh or new dependencies. The initial text slice did no image
  or cache work; it remains available through `--no-images`.
- [x] Unit and controlling-PTY checks cover interaction, restoration, screen bounds,
  full names, safe input, failures, and idle CPU. See [browser design](docs/browser.md).
  36 unit + 8 CLI tests, 25 browser + 16 protocol + 34 layout + 23 cache scenarios,
  fmt, clippy, and release build pass. One idle trial emitted no bytes over one
  second and used 1.87 ms child CPU including startup/quit; no renderer measured.
- [x] User reported all supplied text-browser commands worked well at `1ea10e5`.
  Version/geometry/transport were not separately resupplied.
- [x] Mixed Kitty grid for browser directories with preview candidates, with names
  first, complete-name inspection, and the same sort/filter/navigation behavior.
  Unknown/multiplexed terminals and explicit text flags retain the text browser.
- [x] One lazy decoder worker; one outstanding request/completion; selected source
  first, visible entries next, then one entry on each side. Retain at most 34
  records and place at most 32 images. No parallel decodes or new dependency.
- [x] Viewport/revision/geometry generations discard stale work; overlapping images
  move without re-upload. Session-wide attempt and 8 MiB command budgets include
  cache hits/prefetch/stale attempts and reserve image cleanup bytes.
- [x] Randomized Kitty image numbers, one placement each, individually freed on
  release/exit/suspend. Raw mode/signal ownership remains separate from inline
  output. Exit never joins an in-progress decode; the worker ends with the process.
- [x] 39 unit + 8 CLI tests; 25 text-browser + 21 image-browser scenarios and all
  existing 16 protocol + 34 layout + 23 cache scenarios; fmt/clippy/release pass.
  Controlled slow-worker tests prove stale pixels are discarded and quit does not
  join. [Browser measurements](benchmarks/browser.json) record first-name/preview
  readiness, input response, child CPU/RSS, and opt-in cold/warm cache behavior.
- [ ] User verifies [browser images and cleanup in Ghostty](docs/compatibility.md#browser-image-checklist).
- [ ] Search and any explicit open/copy actions remain later work from daily use.

Inline metadata/layout flags cannot combine with browsing. Directory reads and
terminal writes remain synchronous and can delay input/signals. Decodes cannot be
interrupted in progress; stale work may delay newer previews, but not selection.
Automatic tests do not prove visible Ghostty graphics, resize, or inline retention.

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

Obtain the user's Ghostty pass for progressive browser images, scrolling/navigation,
resize, detail view, quit/signals/suspend, cache reuse, and retention of prior inline
images. Fix any reported issues before claiming visual compatibility. Then tune
browser interaction and per-session limits from daily use; consider search if it
solves a concrete need. Keep the single decoder and opt-in cache policy unless
measurements or actual usage justify changing them. Other terminals/platforms and
packaging remain unverified follow-up work.

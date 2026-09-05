# Roadmap

Updated 2026-09-05. Working sequence; change it as evidence arrives.

## Current state

Rust CLI on arm64 macOS 26.6.2, built with Rust/Cargo 1.98.0. Inline Kitty output
was user-verified in Ghostty 1.3.1 at 122×40 cells, 8×17 pixels per cell. Compact
columns and automatic grid selection also received a clear user visual pass at
`364976f`. Directories-first, custom long fields, and an opt-in thumbnail cache
now have automated coverage. Warm cache output matches uncached PTY bytes; cache
visual behavior has not yet received a new user pass.
Four user images and generated fixtures remain gitignored under `img-test/`.

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
  build, fmt, and clippy pass. New metadata controls await ordinary user feedback.

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
  release build, fmt, and clippy pass. PTY payload equivalence is automated;
  Ghostty cache checks remain unverified.
- [x] [Saved cache measurements](benchmarks/cache.json): four user images at
  122×40 take 71.76 ms uncached, 71.82 ms with empty cache, and 4.60 ms warm
  (~15.6× faster), with 31.36 → 2.03 MiB application RSS and identical output.
  Invalidation returns to 72.34 ms. OS cache warm; no terminal renderer measured.
  Plain 10,000-name output stays ~7.2–7.3 ms in a same-session before/after check.

The experiment remains off by default. [Cache design and limits](docs/cache.md)
record the direct-mapping collision tradeoff, local locking assumptions, storage
accounting, and recovery behavior. No worker concurrency or browser was added.

## 3. Explicit browser — pending

Alternate screen; keyboard selection, scrolling, navigation, search, quit, resize,
and Ctrl-C restoration. Names first; decode visible entries plus a small margin.
Bound jobs, bytes, and terminal residency; ignore stale completions. Idle browsing
should not spin. Open/copy actions require explicit user input. Validate image
cleanup separately from inline persistence.

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

Evaluate cache usefulness beyond the four-image working set: measure hit rate and
latency on independent sources near/above the 64-slot capacity, plus alternating
thumbnail geometry, before changing its replacement or default policy. Gather the
user's Ghostty empty/warm/disabled comparison and daily-use feedback using the
[compatibility checklist](docs/compatibility.md). Keep caching opt-in and all
experiment data inside the repo. Detailed resize/theme, SSH/multiplexer behavior,
worker concurrency, and browsing remain separate follow-ups.

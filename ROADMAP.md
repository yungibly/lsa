# Roadmap

Updated 2026-09-05. Working sequence; change it as evidence arrives.

## Current state

Rust CLI on arm64 macOS 26.6.2, built with Rust/Cargo 1.98.0. Inline Kitty output
was user-verified in Ghostty 1.3.1 at 122×40 cells, 8×17 pixels per cell. Compact
columns and automatic grid selection also received a clear user visual pass at
`364976f`. Directories-first and custom long fields now have automated coverage.
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

## 2. Daily inline use — default layouts user-verified; metadata controls implemented

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
No cache, concurrency, or browser added in this chunk.
Use daily feedback to decide whether those additions are justified. If caching
comes next, require versioned keys, bounded storage, atomic writes, graceful
corruption handling, and disable/clear controls.

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

Build a bounded thumbnail-cache experiment against the existing cold-decode
baseline (~71 ms for the four user images). Keep the default text path untouched;
measure warm hits and invalidation before enabling caching by default. Use
versioned keys, atomic writes, a storage cap, corruption recovery, and disable/clear
controls. Keep experiment data inside the repo while developing. Detailed resize,
theme, SSH/multiplexer behavior and browsing remain separate follow-ups.

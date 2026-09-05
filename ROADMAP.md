# Roadmap

Updated 2026-09-05. Working sequence; change it as evidence arrives.

## Current state

Documentation-only Git repository at session start. Rust 1.98.0 and Cargo 1.98.0
are available on arm64 macOS 26.6.2. The execution environment reports `TERM=dumb`;
Ghostty is the user's visual test terminal, unavailable through this shell's PATH.
The user supplied four JPEG/GIF/WebP images during implementation; synthetic
fixtures were added in gitignored `img-test/generated/`.

## 1. Useful inline prototype — implemented, visual check pending

- [x] Inspect workspace/toolchain; narrow graphics scope to Kitty.
- [x] Implement paths, hidden filtering, deterministic ordering, basic long
  output, safe filename display, partial errors, and normal closed-pipe behavior.
- [x] Keep redirected/text output free of graphics and content reads.
- [x] Build one mixed-entry grid, bounded static decoding, quiet chunked Kitty
  transmission, and visible placeholders for failures or exhausted budgets.
- [x] Test CLI behavior, special files, invalid-byte display, image limits, protocol framing,
  and headless terminal output. Generate a reproducible mixed fixture.
- [x] Record application timing, memory, payload size, and dependency choices.
- [ ] Verify visually in Ghostty: multiple rows, bottom edge, repeated output,
  prompt, scrollback, resize, light/dark backgrounds. Record version and transport.

Evidence: 14 unit tests, 6 CLI integration tests, 16 PTY scenarios, clean fmt/clippy,
release build, and [application baseline](benchmarks/README.md). Protocol checks
include all four user images (separate local run), generated static formats,
orientation, first-frame GIF offsets, corruption, bounds, and special files.
[Compatibility](docs/compatibility.md) separates automated and visual evidence.
The terminal acceptance gate remains open.

## 2. Daily inline use — pending

Use the prototype before fixing thresholds or adding infrastructure. Improve
compact listings, auto-layout, image quality/orientation, responsiveness, and
diagnostics where needed. Measure large ordinary and image-heavy directories.
If caching is justified, use versioned metadata keys, bounded storage, atomic
writes, graceful corruption handling, and disable/clear controls. Keep names
complete after preview limits; avoid revisiting inline rows that have scrolled away.

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

Run the [Ghostty checklist](docs/compatibility.md#ghostty-checklist) and record
terminal version, local/SSH/multiplexer conditions, placement/scrolling, prompt,
resize, and transparency results. Fix observed inline issues before auto-layout,
caching, or browsing. Application output is ready; visual compatibility is not
yet established.

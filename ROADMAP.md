# Roadmap

Updated 2026-09-05. The product reset supersedes the pager direction.

## Current product

A familiar `ls` replacement: compact, colored, icon-decorated terminal text with
readable details and automatic inline image previews. Always print and exit;
leave scrollback and input to the terminal/shell. Keep one mixed ordering, complete
names, graceful fallback, and bounded preview work.

## Implemented

- Removed the pager, selection/inspection UI, input handling, terminal lifecycle,
  worker/viewport scheduling, pager tests and its active documentation.
- Added automatic palette colors, bounded safe LS_COLORS rules, Nerd icons in
  Ghostty and portable symbols elsewhere, overrides, and opt-in OSC 8 hyperlinks.
- Added natural filename sorting, unsorted mode, familiar -d/-F/-n controls,
  readable sizes/owners by default, optional headings and selected metadata.
- Made directory symlink operands familiar to ls users; grouped consecutive file
  operands so globs share a layout and long columns align.
- Preview single images, small mixed lists and image-heavy directories automatically.
  Tall galleries print inline; 16 default attempts and 8 MiB image commands across
  operands, then complete compact text. Failures are quiet; no hidden entries.
- Retained the bounded decoder, anonymous inline Kitty framing, and opt-in fixed-slot
  cache. No new dependencies, workers, implicit storage or config system.
- Kept the plain text fast path free of decoration stats; styled listings retain
  only executable bits. Added bounded account-name and exact timestamp caches.
- Rewrote product/installation/decision/handoff docs and added paired application
  measurements for the new defaults. Historical reports are labeled separately.

## Verified and next task

The Rust/CLI, protocol, foreground-TTY, inline layout/style and cache suites pass on
arm64 macOS. Formatting, strict clippy, release build and local package smoke checks
pass. See [compatibility](docs/compatibility.md) for exact scope and
[measurements](benchmarks/README.md) for performance conditions.

**Next:** use the new defaults in Ghostty and act on concrete feedback about icon
alignment, readability, thumbnail density, scrollback placement and ordinary alias
usage. The user handles visual checks. Earlier inline passes do not verify this
new presentation. Then verify Linux builds/packages and the declared Rust 1.88 minimum
in suitable environments. No public release is implied by the host archive.

Keep cache policy opt-in until daily use justifies a change. Add formats or decoder
concurrency only for demonstrated needs. No browsing, file operations, Git scans,
recursive trees, services or plugin system are planned.

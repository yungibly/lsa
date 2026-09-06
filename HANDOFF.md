# Session handoff

2026-09-06. Recheck Git state. Started clean at `4a5269b`; the user has authorized
commits. No push, publication, installation or personal-shell changes requested.
Work/access stays in this repository; the user handles Ghostty visual checks.

## Latest direction

The user prefers the inline-only overhaul. Their screenshot shows an empty-looking
folder tile among four apple images and they request meaningful artwork for every
entry/failure, smaller grid previews, removal of redundant grid glyphs, and possible
tiny long-view previews. Preserve ls simplicity, complete names and performance.

## Implementation

- 14×3-cell maximum grid frames; keep wider label columns, center/wrap full names,
  and suppress redundant glyphs only in thumbnail labels.
- Code-generated folder/file/media/link/error drawings. Every tile has pixels;
  failed previews show error artwork. No added font/dependency/asset requirement.
- One-row 3-cell long-view previews replace the name icon in a fixed gutter. Text
  overrides and narrow/short terminal fallback remain; later entries stay aligned
  after budgets. Piped metadata stays plain. Full names may wrap normally.
- One case-insensitive filename classifier for style and preview eligibility.
  Recognized image files keep image styling even with executable mode bits;
  unsupported/unknown types get image/generic artwork respectively.
- All graphics, including artwork, share 8 MiB and 256 placement limits. Source
  attempts remain 16 by default; failures/cache hits consume attempts. Completed
  grid rows fall back to compact text when budgets run out. Cache stays opt-in.
- Existing cache keys already separate thumbnail geometry; long miniatures cannot
  reuse a grid-sized record. Built-in artwork never opens source files or storage.

The prior release binary is saved as `target/lsa-before-tile-polish`. Original
img-test files remain unchanged. `examples/preview_sheet.rs` produces an offline
light/dark artwork sheet under target/visual-checks; it was inspected without GUI
control. `tests/check_thumbnails.py` replays terminal rows and checks actual pixels.
The PTY helper now drains through EOF after child exit to avoid losing a final write.

## Verification and next work

38 unit + 12 CLI tests, strict clippy and release build pass. The PTY suite covers
16 protocol + 48 layout/style + 23 cache + 21 thumbnail UX scenarios. Package checks
and paired measurements are recorded in compatibility/benchmarks.
Use README's test/build commands. Unix socket CLI fixtures require normal sandbox
escalation. The target remains Ghostty; current art/layout needs a user terminal pass.
The latest screenshot does not resupply version/geometry/transport. Historical
context: Ghostty 1.3.1, arm64 macOS 26.6.2, 122×40 cells, 8×17 pixels. Linux and the
Rust 1.88 minimum remain unverified. No terminal queries, input handling or paging.

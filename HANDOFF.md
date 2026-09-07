# Session handoff

## 2026-09-07 — verified Ghostty and v0.2.0 release

The user tested everything successfully in Ghostty and explicitly authorized a
final efficiency pass, commit, tag, push, and watching CI until green. Their terminal
version/geometry/transport were not resupplied. Recheck Git state before continuing.

The final pass limits artwork polygon rasterization to vertex bounds and paints
integer rectangles directly. No caching, memory policy, image pixels or terminal
behavior changes. The full artwork sheet and paired end-to-end outputs match
exactly. Folder grids use about 16% less CPU; mixed artwork grids about 22% less;
raster previews are unchanged. Evidence: benchmarks/artwork.json and its harness.
Before binary: target/lsa-before-final-efficiency. Version chosen: v0.2.0.

Commit `ba4a882` and annotated tag `v0.2.0` are pushed. All five jobs passed in
CI: https://github.com/yungibly/lsa/actions/runs/34106546519 (four native targets
plus Rust 1.88). Local full checks and extracted v0.2.0 package also passed.
Release workflow: https://github.com/yungibly/lsa/actions/runs/34107070950.
All eleven release jobs passed, including native builds, Rust 1.88, publication,
Homebrew installations on both macOS architectures and x86-64 Linux, and the tap
update. Public release: https://github.com/yungibly/lsa/releases/tag/v0.2.0.
Reviewed docs/releases/v0.2.0.md notes are applied. The public formula independently
checks as v0.2.0 with all four architecture URLs. The published Apple Silicon
archive passed SHA-256, extraction, version and mixed-listing checks locally.
Progress/results and downloaded files stay under ignored target/automation/.
No local Homebrew installation, shell changes or source-image modifications.

The final documentation follow-up records these results with [skip ci]; source
and the release tag remain exactly the successfully tested ba4a882 commit.
Next: normal personal use and concrete regressions; no pending publication work.
The prior "not requested" release restriction below is superseded by the user's
latest authorization.

## 2026-09-07 — gallery defaults, sizing, UX and formats

The user's personal use found the 16-image default frustrating. Work is local and
uncommitted on main; recheck Git state. The original images remain unchanged, all
work/artifacts are in this repository, and no computer use was performed. The
published Homebrew v0.1.0 release has not been changed by this session.

- Default source attempts: 256; `--preview-limit=0..4096`, with equals or space.
  Graphics: 128 MiB and 4,096 placements across all operands. Budget exhaustion
  retains compact text; partially completed grid rows retain artwork.
- `--thumbnail-size=1..12`: terminal-row increments, default 3, proportional width/
  spacing, terminal clamping, unchanged 320×240 maximum pixels. Long miniatures
  stay one row. Cache keys continue to separate pixel geometry.
- Help now has examples, says --header/--fields imply -l, and explains precedence.
  An explicit ignored grid/size request gets one notice per invocation, including
  --grid --header. Familiar text overrides remain order independent.
- SVG shapes/paths/gradients/clipping via resvg 0.48.1 with default features off;
  bounded XML preflight, disabled image resolvers, no fonts/resources. Whole-preview
  fallback for text, embedded/external images, filters, masks, patterns, markers,
  use expansion; 256 KiB, 4,096 nodes, 32 nesting levels. ICO uses existing image
  codecs. WebM is deliberately deferred to avoid codec/process complexity, as the
  user allowed. No pager: existing scrollback model remains.
- Bounded buffered raster reads remove one compressed-source copy. JPEG EXIF
  rotation follows thumbnailing; 4 KiB base64 scratch replaces whole-image encoding
  allocation. Slight fractional resize-edge changes require cache v2; old cache
  namespace is untouched and clear acts on the new one only.

Checks: 46 unit + 12 macOS CLI + one artwork-example + three release-tooling tests;
183 PTY scenarios including 75 new gallery checks; formatting, strict clippy,
release build, extracted host-package smoke checks. Unix socket fixtures needed
normal sandbox escalation. CI includes the new gallery script, but hosted Linux
and Rust 1.88 have not yet checked this change; prior passes below are historical.

The before binary is `target/lsa-before-gallery-options`. Paired reports and
methodology are in benchmarks/README.md. Equal-work galleries and text timings are
essentially unchanged; 6.4 MP rotated JPEGs use substantially less time/memory.
PTY timing is not renderer timing. The extra fixtures are at
`target/visual-checks/gallery-files`, created by examples/gallery_fixtures.rs.

**Next:** user Ghostty check of large personal directories, sizes 1/3/6/12, SVG/ICO/
EXIF fixtures, --grid --header notice, long alignment, and prior placements in
scrollback. Record version, geometry and transport. See docs/compatibility.md for
commands. A new release/push was not requested; run hosted CI when preparing it.

## Previous session — distribution

2026-09-06. Recheck Git state. Distribution work started at `8f71819` with the user's
`.gitignore` addition for `.env`. The user authorized GitHub builds, release and
Homebrew tap setup, including pushing the needed changes and publication.
Work/access stays in this repository; the user handles Ghostty visual checks.

## Distribution verified

The chosen name is `lsa`, tagline `ls, augmented`. The remote is
`https://github.com/yungibly/lsa.git`; the user made it public during this session.
The normal GitHub CLI session has write access. `.env` contains BREWTAP_TOKEN for
`yungibly/homebrew-tap`; it must stay ignored and its value must never be printed.

Release `v0.1.0` tags `ded6e1c`, following setup commit `33bf7ee`. Both commits are
pushed. BREWTAP_TOKEN is configured as a repository Actions secret. Native CI and
the full release pipeline passed:

- CI: https://github.com/yungibly/lsa/actions/runs/34011569797
- Release: https://github.com/yungibly/lsa/actions/runs/34011671355
- Public binaries: https://github.com/yungibly/lsa/releases/tag/v0.1.0
- Formula: https://github.com/yungibly/homebrew-tap/blob/main/Formula/lsa.rb

Four architectures passed native builds, strict lint, all Rust and 108 headless
terminal scenarios, and extracted-package smoke tests. CI uses Rust 1.98.1; Rust
1.88.0 passed separately on GNU x86-64 Linux. Homebrew install/test passed on
arm64 macOS 14.8.9, Intel macOS 15.7.9 and x86-64 Ubuntu 24.04.4 before the automatic
tap commit. ARM64 Linux binaries passed native tests on Ubuntu 24.04.4; Homebrew
installation there is not yet tested. The published Apple Silicon archive passed
checksum, extraction, version and mixed-listing checks on this machine as well.

The first Linux lint run exposed libc type differences; metadata keeps portable
mode casts and infers localtime_r's time type. Concurrent local cache tests also
exposed a PTY capture tail loss: keep the slave open until buffered master bytes
are drained after child exit. Ten repeated stress runs (160 concurrent captures)
passed after the fix. No listing/preview behavior was intentionally changed.

Install with `brew install yungibly/tap/lsa`. Future matching stable version tags
publish checksummed binaries and update the formula automatically. See
`docs/install.md` for release/recovery instructions. Local temporary tooling,
logs, tap README staging, and the downloaded package stay under ignored
`target/automation/`; no local Homebrew installation or shell changes were made.

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
context: Ghostty 1.3.1, arm64 macOS 26.6.2, 122×40 cells, 8×17 pixels. Linux packages
and Rust 1.88 are now verified by the hosted jobs above. No terminal queries, input
handling or paging.

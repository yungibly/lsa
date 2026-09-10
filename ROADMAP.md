# Roadmap

Updated 2026-09-10. Print into scrollback, return to the shell, keep one mixed
ordering and complete names. No pager or file browser.

## Current change — everyday defaults and listing efficiency

Terminal text now defaults to long details; image-heavy and small mixed listings
still switch automatically to grids. `-C` / `--columns` selects compact text,
`-1` keeps lines, and pipes still default to plain names. Explicit `-l`, `-n`,
`--header` and `--fields` retain their precedence. Default directory-symlink
operands continue to list contents; explicit `-l` / `-d` still show the link itself.

`--12-hour` uses zero-padded 12-hour local times with AM/PM. The default remains
24-hour, and the flag alone does not add metadata to pipes. Dates, timezone/DST
conversion and complete alignment are preserved.

Listing metadata retains only required stat fields. Layout selection precedes
metadata reads, so automatic grids and plain pipes do not acquire long-form cost.
Local timestamps convert once per entry across alignment/output; compact civil
components supplement the existing bounded lookup cache only for the modified
column. Safe filenames avoid an escaped copy and permission characters no longer
allocate individually. No new dependencies, workers, cache policy or image changes.

Local validation: 47 unit tests, 13 macOS CLI tests, one artwork-example test,
three release-tooling tests, and 195 headless terminal scenarios. Formatting,
strict clippy, release build and extracted-package smoke checks pass. Paired
measurements show materially less time for distinct-timestamp long listings and
less metadata memory; image decoding/output is unchanged. See
[measurements](benchmarks/README.md). These are application/PTY measurements,
not Ghostty rendering measurements.

Version 0.3.0 is prepared for the user-authorized commit, tag, push and CI/release
watch. **Next:** finish publication and record hosted results; then normal use
and a user Ghostty pass for default details and AM/PM alignment. No new real-terminal
visual pass has been claimed. Keep cache opt-in and WebM deferred.

## Previous change — larger galleries and simpler preview controls

Personal use showed that the 16-preview cutoff hurts the experience. The local
implementation now defaults to 256 source attempts, with `--preview-limit=0..4096`.
The shared graphics limits increase to 128 MiB / 4,096 placements, enough for 256
maximum-resolution thumbnails. Output still streams sequentially to scrollback;
every name survives exhaustion or failure. No pager was added.

`--thumbnail-size=1..12` chooses grid height in single terminal-row steps, default
3. Width/spacing follow, frames shrink to fit, pixel dimensions remain capped at
320×240, and labels wrap completely. Long previews stay one row. `--header` and
`--fields` are documented as long-implying options, help has examples, and explicit
grid/size requests overridden by other flags get one explanatory stderr notice.

SVG vector artwork and ICO have content previews. SVG rendering is restricted to
self-contained shapes, paths, gradients and clipping, with source/node/depth bounds
and no fonts/external resources. Text, embedded images, filters and expansion-heavy
features fall back as a whole preview. WebM frame decoding is deferred because it
would add a video codec stack or external process management.

Efficiency changes: buffered bounded source reads, fixed 4 KiB base64 scratch,
JPEG rotation after thumbnailing, and artwork rasterization restricted to each
shape's bounds. The last pass preserves exact pixels while reducing artwork CPU.
Cache namespace/magic advance to v2 because
fractional resize-edge pixels can change; storage stays opt-in and bounded.

## Previous change — GitHub builds and Homebrew distribution

The name remains **lsa — ls, augmented**. CI and tagged-release workflows now
target native Apple Silicon/Intel macOS and ARM64/x86-64 Linux. Every target runs
Rust tests, headless terminal checks and extracted-package smoke tests. Linux
packages use musl; a separate job checks the declared Rust 1.88 minimum.

Stable version tags must match Cargo.toml. The release job verifies checksums and
generates the binary Homebrew formula; installation tests on both macOS
architectures and x86-64 Linux gate the automatic tap commit. The tap token is an
Actions secret, never a tracked file. [v0.1.0](https://github.com/yungibly/lsa/releases/tag/v0.1.0)
is published and the tap installs it with `brew install yungibly/tap/lsa`.
The complete [release workflow](https://github.com/yungibly/lsa/actions/runs/34011671355)
passed, including every native build, Rust 1.88, Homebrew installations on both
macOS architectures and x86-64 Linux, and the automatic tap update. ARM64 Linux
has native package tests; a Homebrew installation check there remains future work.

## Previous change — thumbnail presentation

The user passed the inline-only direction and supplied a Ghostty screenshot showing
oversized image tiles, nearly empty folder tiles and redundant/unhelpful filename
icons. This change addresses that feedback:

- Grid image frames shrink from 22×5 to at most 14×3 cells at the reported geometry.
  Wider label columns remain; names center beneath frames and wrap completely.
- Every grid entry gets pixels: source thumbnail or built-in folder/file/media/link/
  error artwork. Grid labels no longer repeat font icons. Unknown types get a generic
  file icon/color; case-insensitive image classification is shared with the decoder.
- Long view uses one-row, 3-cell-wide miniatures in an aligned name gutter. Ordinary
  entries retain icons, failures get error artwork, and exhausted budgets retain the
  same alignment. Narrow/short terminals, pipes, -1 and --no-images use text.
- Built-in drawings are code-native and font-independent. All graphics count against
  8 MiB and 256 placements across operands; source attempts still default to 16.
  Artwork never uses decoder/cache work. All names survive every fallback.
- Tests cover visible artwork, tiny preview pixels, actual cursor/row replay, mixed
  alignment, GIF/unknown types, budget sharing and cache geometry. Offline light/dark
  artwork inspection supplements byte tests; it is not a real-terminal pass.

## Previous v0.2.0 verification

The gallery change has 46 unit tests, 12 local macOS CLI tests, one artwork-example
test, three release-tooling tests, and 183 headless terminal scenarios (75 new),
including all sizes at four geometries, 256/300-image galleries, 4,096 placements,
full names, SVG/ICO pixels, special-file fallback, shared budgets and cache geometry.
Formatting, strict clippy, release build and local extracted-package smoke checks
pass. The [v0.2.0 commit's CI](https://github.com/yungibly/lsa/actions/runs/34106546519)
also passed every native macOS/Linux job and Rust 1.88 compatibility.

Paired measurements separate equal work from more previews and application cost
from rendering. Plain output/cost and equal-work gallery speed are essentially
unchanged; source buffering lowers memory and thumbnail-first rotation saves both
time and memory. See [benchmarks](benchmarks/README.md),
[compatibility](docs/compatibility.md) and [handoff](HANDOFF.md).

The user reported all requested Ghostty checks working perfectly on 2026-09-07.
Version, geometry and transport were not resupplied. The final artwork optimization
is byte-identical in the full artwork sheet and paired end-to-end galleries.

Commit `ba4a882` and tag `v0.2.0` are pushed; the
[release workflow](https://github.com/yungibly/lsa/actions/runs/34107070950) passed
all eleven jobs, including every native build, Rust 1.88, publication, Homebrew
installations on both macOS architectures and x86-64 Linux, and the tap update.
[v0.2.0](https://github.com/yungibly/lsa/releases/tag/v0.2.0) is published with
release notes and checksummed binaries. The public Apple Silicon archive also
passed checksum, extraction, version and mixed-listing checks locally.

**At v0.2.0:** normal personal use and any concrete regressions. WebM remains deferred;
no additional visual check is needed for the identical artwork output. No pending
release work. Use `brew update && brew upgrade lsa` for an existing installation.

Keep cache policy opt-in. No recursive trees, Git scans, file operations, services or
plugin system are planned. Add formats/concurrency only from demonstrated needs.

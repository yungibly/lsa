# Roadmap

Updated 2026-09-06. Print into scrollback, return to the shell, keep one mixed
ordering and complete names. No pager or file browser.

## Current change — GitHub builds and Homebrew distribution

The name remains **lsa — ls, augmented**. CI and tagged-release workflows now
target native Apple Silicon/Intel macOS and ARM64/x86-64 Linux. Every target runs
Rust tests, headless terminal checks and extracted-package smoke tests. Linux
packages use musl; a separate job checks the declared Rust 1.88 minimum.

Stable version tags must match Cargo.toml. The release job verifies checksums and
generates the binary Homebrew formula; installation tests on both macOS
architectures and x86-64 Linux gate the automatic tap commit. The tap token is an
Actions secret, never a tracked file. The first live workflow/release verification
is in progress; final hosted outcomes will be recorded here after execution.

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

## Verification and next task

38 unit and 12 CLI tests, 108 PTY scenarios, strict clippy, formatting, release build
and extracted-package smoke checks pass. Paired measurements show 52.2% fewer
graphics bytes for the mixed grid and unchanged plain pipe output/cost; uncached
long-view miniatures still incur source decoding. See
[compatibility](docs/compatibility.md), [benchmarks](benchmarks/README.md) and
[handoff](HANDOFF.md). The user handles Ghostty visual verification.

**Next:** finish the first hosted builds, release and Homebrew tap verification.
Then check the smaller gallery and long-view miniatures in Ghostty, particularly
folder/error recognizability, filenames wrapping, scrollback and long-row alignment.
The new CI jobs will establish Linux package and Rust 1.88 verification status.
Earlier terminal passes do not verify this new placement/presentation.

Keep cache policy opt-in. No recursive trees, Git scans, file operations, services or
plugin system are planned. Add formats/concurrency only from demonstrated needs.

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

## Verification and next task

38 unit tests, 12 macOS / 13 Linux CLI tests, one artwork-example test, three
release-tooling tests, 108 PTY scenarios, strict clippy, formatting, native release
builds and extracted-package smoke checks pass. The published Apple Silicon
archive also passed checksum and executable checks locally. Paired measurements show 52.2% fewer
graphics bytes for the mixed grid and unchanged plain pipe output/cost; uncached
long-view miniatures still incur source decoding. See
[compatibility](docs/compatibility.md), [benchmarks](benchmarks/README.md) and
[handoff](HANDOFF.md). The user handles Ghostty visual verification.

**Next:** check the installed smaller gallery and long-view miniatures in Ghostty, particularly
folder/error recognizability, filenames wrapping, scrollback and long-row alignment.
Linux package and Rust 1.88 verification is now covered by the passing CI jobs.
Earlier terminal passes do not verify this new placement/presentation.

Keep cache policy opt-in. No recursive trees, Git scans, file operations, services or
plugin system are planned. Add formats/concurrency only from demonstrated needs.

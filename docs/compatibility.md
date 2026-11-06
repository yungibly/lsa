# Compatibility and verification

What has actually been verified, where, and by whom. Protocol support in a
terminal or library is not lsa validation; record real terminal evidence here.

## Evidence

| Environment | Text, colors, links | Kitty grids and miniatures | Notes |
| --- | --- | --- | --- |
| Ghostty 1.3.1, arm64 macOS 26.6.2, direct | User pass through v0.2.0 | User pass for v0.2.0 galleries, sizes 1-12, SVG/ICO, miniatures, scrollback (2026-09-07) | v0.3.0 details/AM-PM, v0.4.0 link clicks and all v0.5.0 changes are pending a user pass |
| Ghostty 1.3.1 release-source build, x86-64 Debian 13.6, direct X11 | v0.5.0 long-view columns and plain pipe output visually checked (2026-09-30); link clicks untested | Miniatures and an eight-image PNG/JPEG/WebP gallery visually checked | [Recorded demo](demo.md); Mesa llvmpipe software rendering, 1188×800 capture. Limited pass; macOS/system formats and broader scrollback checks remain pending |
| Ghostty terminal core (libghostty-vt) via tui-test 0.1.0-beta.5, headless | v0.5.0 checked: name alignment beside miniatures and icons, colors, parsed OSC 8 targets, `-C`/`-x`, wrapped grid labels, scrollback, budget fallback, no leaked payload bytes | Parsed, not drawn | `tests/check_ghostty_vt.py`; text layer only |
| Headless PTY, 4 native CI targets (arm64/x86-64 macOS, arm64/x86-64 musl Linux) | Byte and cursor models | Protocol bytes, decoded RGBA payloads, placements and budgets | 248 scenarios for v0.5.0, plus unit and CLI tests |
| macOS ImageIO, arm64 macOS (Darwin 27) | — | HEIC/HEIF, AVIF, TIFF (alpha), PSD, 18-48 MP JPEG/HEIC/AVIF, EXIF orientations checked by pixels | Fixtures made with `sips`; real camera RAW and iPhone HEIC files not yet tried |
| Kitty | Unverified | Unverified | Auto-enabled by `TERM=xterm-kitty` or `TERM_PROGRAM=kitty` |
| tmux, screen, zellij | Unverified | Text by default | `--protocol=kitty` forcing is unverified |
| SSH | Links off by default | Text by default since v0.5.0 | Forced graphics and remote file URIs unverified |
| Other terminals | Unverified | Text | Unknown terminals never get graphics automatically |

### v0.6.0 local verification (2026-11-06)

- Formatting and Clippy with warnings denied pass; 110 Rust tests pass on
  arm64 macOS, including real xattrs, macOS ACLs/file flags, symlink policies,
  deep recursion, cycle/error continuation and dot-entry metadata.
- 257 headless PTY scenarios pass. Nine new recursive cases verify one shared
  preview budget across directory sections, complete names after exhaustion,
  cycle handling, cursor limits and unchanged terminal modes.
- Nine Ghostty terminal-core checks pass via tui-test 0.1.0-beta.5:
  long-column alignment, colors, links, wrapping, scrollback and fallback.
  This backend parses but does not draw Kitty images.
- Release packaging validates the archive checksum and runs the extracted
  binary's version and mixed-listing smoke tests. Release-tool unit tests pass.
- Some local system-format fixture conversions (HEIC, AVIF, PSD) were unavailable
  to `sips`; the gallery checks reported those skips. This pass does not add
  new visual evidence for those formats. Hosted platform results are recorded
  after CI and release completion.
- The first hosted attempt could not start its ARM Mac job because GitHub
  retired the macOS 14 image on November 2. Build and Homebrew jobs now use
  macOS 15; `MACOSX_DEPLOYMENT_TARGET=14.0` still sets the binary's minimum.

The latest real-renderer evidence remains the v0.5.0 Ghostty demo above.
v0.6.0's changed long columns, optional metadata rows and recursive output
still need a human visual pass. Application measurements are in
[benchmarks](../benchmarks/README.md).

### v0.6.0 hosted results (2026-11-06)

- [PR #2](https://github.com/yungibly/lsa/pull/2) was merged after
  [all five CI jobs](https://github.com/yungibly/lsa/actions/runs/42304434761)
  passed: both native macOS architectures, both native musl Linux architectures,
  and Rust 1.88 plus release tooling. The release tag points to merge commit
  `5823fe1`, whose tree is identical to the checked PR head.
- The [release workflow](https://github.com/yungibly/lsa/actions/runs/42305020841)
  first stopped at the ARM Mac gallery-memory assertion: one 8-versus-24-preview
  sample measured 40.44 versus 67.98 MiB (27.55 MiB difference, above the
  unchanged 24 MiB guard). All other platform jobs passed.
- Investigation found no decoder/pool/ImageIO changes from v0.5.0. Seven local
  paired trials of each version stayed within 1.10 MiB of growth; three further
  trials each at 8, 24 and 96 previews plateaued around 50 MiB in both versions.
  These local samples do not reproduce the hosted VM's native allocator state.
  The failed ARM job was retried without changing code or the memory threshold.
- The unchanged retry passed. All eleven release jobs completed successfully,
  including four platform archives, Homebrew installs on Apple Silicon, Intel
  Mac and Linux, and the automatic tap update.
- [v0.6.0](https://github.com/yungibly/lsa/releases/tag/v0.6.0) is public, with
  four archives, individual checksums and `SHA256SUMS`. The downloaded Apple
  Silicon archive passed checksum, version, plain-output, format precedence,
  dot-entry, symlink, recursion and metadata-option smoke checks. Its SHA-256 is
  `87ed9e0d2e105ae0a7528de9f44ed14557535f97075cd40494cc96243600c31f`, matching
  the [0.6.0 tap formula](https://github.com/yungibly/homebrew-tap/blob/main/Formula/lsa.rb).

### v0.5.0 hosted results (2026-09-28/29)

- [CI at eadbdc0](https://github.com/yungibly/lsa/actions/runs/36502914797)
  passed all five jobs.
- The [release workflow](https://github.com/yungibly/lsa/actions/runs/36503290801)
  passed all eleven jobs without retries: native builds, Rust 1.88,
  publication, Homebrew installs on arm64/Intel macOS and x86-64 Linux, and the
  tap update.
- The public Apple Silicon archive passed checksum, version, plain pipe output,
  six ImageIO previews, SSH text fallback, links and `--diagnose` checks.
- On the macOS runners, sips could not write AVIF, so AVIF decoding was checked
  locally only. HEIC, TIFF, PSD and 18 MP JPEG decoding passed there, including
  on Intel.
- The [first CI run](https://github.com/yungibly/lsa/actions/runs/36502342540)
  failed on the arm64 macOS runner. That VM lacks Apple Silicon's hardware
  scaler, and ImageIO printed "IOServiceMatchingfailed for:
  AppleM2ScalerCSCDriver" to stderr. Since eadbdc0, stderr is silenced while
  ImageIO workers run (lsa writes nothing there meanwhile).

Earlier versions' results are linked from their tags' workflow runs on GitHub.

## Ghostty check for v0.6.0

Run from this repository in Ghostty and record Ghostty version, OS, cell/pixel
geometry and transport (direct, multiplexer, SSH):

```sh
./target/release/lsa img-test                         # parallel decode, JPEG via ImageIO
./target/release/lsa --grid --thumbnail-size=6 img-test
./target/release/lsa -l img-test/generated            # miniatures, red dangling link, "-" sizes
./target/release/lsa -C img-test/generated            # columns filled downward
./target/release/lsa -x img-test/generated            # rows filled across
./target/release/lsa -la img-test/generated           # dots, full details and total
./target/release/lsa -R --preview-limit=3 img-test/generated  # shared recursive budget
./target/release/lsa -le@ src                         # optional metadata rows
TERM=xterm-256color ./target/release/lsa --grid img-test         # prints a --grid notice
```

System formats on macOS, from the existing fixtures:

```sh
mkdir -p target/visual-checks/system-formats
for format in heic avif tiff psd; do sips -s format $format img-test/generated/landscape.png --out target/visual-checks/system-formats/landscape.$format; done
sips -s format heic img-test/shutterstock_1798373137.jpg --out target/visual-checks/system-formats/photo.heic
sips -s format tiff img-test/generated/transparent.png --out target/visual-checks/system-formats/transparent.tiff
./target/release/lsa --grid target/visual-checks/system-formats
```

Look for:
- thumbnails that match their sources (upright, colors, transparency checker);
- rows appearing progressively in a large gallery;
- earlier images still intact after scrolling back past several listings;
- ordinary filenames readable on a light theme;
- Command-click opening files and folders.

Any HEIC/HEIF, AVIF or camera RAW files from a real camera or phone are the
most useful additional samples.

## Headless checks

PTY tests replay emitted bytes against a small cursor model and decode every
image payload, inflating `o=z` data. They establish bytes, placements and
budgets, not visible rendering. `tests/check_ghostty_vt.py` adds Ghostty's own
parser for the text layer. Neither measures terminal rendering time or image
memory.

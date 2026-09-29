# Compatibility and verification

What has actually been verified, where, and by whom. Protocol support in a
terminal or library is not lsa validation; record real terminal evidence here.

## Evidence

| Environment | Text, colors, links | Kitty grids and miniatures | Notes |
| --- | --- | --- | --- |
| Ghostty 1.3.1, arm64 macOS 26.6.2, direct | User pass through v0.2.0 | User pass for v0.2.0 galleries, sizes 1-12, SVG/ICO, miniatures, scrollback (2026-09-07) | v0.3.0 details/AM-PM, v0.4.0 link clicks and all v0.5.0 changes are pending a user pass |
| Ghostty terminal core (libghostty-vt) via tui-test 0.1.0-beta.5, headless | v0.5.0 checked: name alignment beside miniatures and icons, colors, parsed OSC 8 targets, `-C`/`-x`, wrapped grid labels, scrollback, budget fallback, no leaked payload bytes | Parsed, not drawn | `tests/check_ghostty_vt.py`; text layer only |
| Headless PTY, 4 native CI targets (arm64/x86-64 macOS, arm64/x86-64 musl Linux) | Byte and cursor models | Protocol bytes, decoded RGBA payloads, placements and budgets | 248 scenarios for v0.5.0, plus unit and CLI tests |
| macOS ImageIO, arm64 macOS (Darwin 27) | — | HEIC/HEIF, AVIF, TIFF (alpha), PSD, 18-48 MP JPEG/HEIC/AVIF, EXIF orientations checked by pixels | Fixtures made with `sips`; real camera RAW and iPhone HEIC files not yet tried |
| Kitty | Unverified | Unverified | Auto-enabled by `TERM=xterm-kitty` or `TERM_PROGRAM=kitty` |
| tmux, screen, zellij | Unverified | Text by default | `--protocol=kitty` forcing is unverified |
| SSH | Links off by default | Text by default since v0.5.0 | Forced graphics and remote file URIs unverified |
| Other terminals | Unverified | Text | Unknown terminals never get graphics automatically |

Hosted CI and release results for each version are linked from its tag's
workflow runs on GitHub.

## Ghostty check for v0.5.0

Run from this repository in Ghostty and record Ghostty version, OS, cell/pixel
geometry and transport (direct, multiplexer, SSH):

```sh
./target/release/lsa img-test                         # parallel decode, JPEG via ImageIO
./target/release/lsa --grid --thumbnail-size=6 img-test
./target/release/lsa -l img-test/generated            # miniatures, red dangling link, "-" sizes
./target/release/lsa -C img-test/generated            # columns filled downward
./target/release/lsa -x img-test/generated            # rows filled across
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

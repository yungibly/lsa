# Compatibility and verification

## Evidence

| Environment | Result |
| --- | --- |
| Ghostty 1.3.1, arm64 macOS 26.6.2, 122×40 cells / 8×17 pixels | Earlier anonymous inline Kitty previews and compact text received user passes. Metadata/cache commands also received a pass at `7050349`; transport/version were not resupplied for that pass. |
| User's latest Ghostty screenshot | Inline-only direction approved; requested smaller previews, visible folder/error artwork, simpler grid labels and tiny long-view previews. Version/geometry/transport were not resupplied. |
| Current arm64 macOS, Rust 1.98.0 | Rust unit/CLI tests, strict clippy, release build, PTY byte/cursor checks, foreground-TTY completion and extracted host package smoke checks. |
| Offline artwork sheet | Inspected on light and dark backgrounds at 112×51 and 24×17 pixels. Every built-in folder/file/media/error icon is visible without a font. This is not a terminal renderer. |
| Current smaller grid and long-view miniatures | Implemented and automated-tested; actual Ghostty appearance/scrollback placement await the user's check. |
| Linux, Rust 1.88 minimum, other terminals, multiplexers, SSH | Unverified. Protocol hints or upstream support are not lsa validation. |

Automated suite: 38 unit and 12 CLI tests, 16 protocol/cursor scenarios,
48 inline layout/style scenarios, 23 cache scenarios, and 21 focused thumbnail
scenarios. The new tests inspect actual RGBA payloads and replay text rows to verify
nonempty artwork, one-row miniature geometry, consistent name columns, complete
symlink targets, no extra rows, limits across operands and distinct cache sizes.

Both stdin-disconnected and same-foreground-TTY overflow exit normally without a
key press. Non-controlling PTYs verify unchanged termios. On macOS, controlling
PTY attributes can become unreadable after the session leader exits; those trials
check completion and absence of interactive terminal commands instead. The helper
drains through EOF after child exit so fast final writes cannot be dropped.
No PTY test renders the pixels or establishes a visual pass.

## Ghostty check

Run from this repository in Ghostty:

```sh
./target/release/lsa img-test
./target/release/lsa --grid img-test/generated
./target/release/lsa -l img-test
./target/release/lsa --header img-test/generated
./target/release/lsa --preview-limit=2 img-test/generated/many
./target/release/lsa -l --preview-limit=2 img-test/generated
./target/release/lsa -l --no-images img-test
NO_COLOR=1 ./target/release/lsa --grid img-test/generated
./target/release/lsa -l img-test | cat
```

Grid frames should be noticeably smaller, with a folder drawing instead of an empty
space and error drawings for broken previews. Full filenames should center below
the frames without redundant font icons. GIF previews use the first frame; unsupported
formats get image artwork and unknown files get a document drawing.

Long-view images should be tiny, one-row previews beside the filename. Ordinary
icons, images and later entries after the budget should share a name column. Metadata
and symlink targets remain text. Each entry should take the same height it needs for
its text; a miniature should add no extra row. Check narrow windows and long names.

Scroll up after subsequent commands to check old placements; lsa never deletes them.
Repeat after resizing between invocations. Record terminal/version, OS, cell/pixel
geometry, Kitty protocol, inline mode and direct/multiplexer/SSH transport.
`--diagnose PATH` supplies hints/geometry and budgets, not renderer validation.

To regenerate the offline drawing sheet:

```sh
cargo build --release --locked --example preview_sheet
./target/release/examples/preview_sheet
```

It writes `target/visual-checks/artwork.png`. Its rows show folder, document, image,
video, audio, archive, code, settings, link, special-file and error drawings.

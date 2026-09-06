# Compatibility and verification

## Evidence

| Environment | Result |
| --- | --- |
| Ghostty 1.3.1, arm64 macOS 26.6.2, 122×40 cells / 8×17 pixels | Earlier anonymous inline Kitty previews and compact text received user passes. Metadata/cache commands also received a pass at `7050349`; transport/version were not resupplied for that pass. |
| Current arm64 macOS, Rust 1.98.0 | Rust unit/CLI tests, strict clippy, release build, PTY byte/cursor checks, foreground-TTY completion, and extracted host package smoke checks. |
| Current Ghostty styling and bounded gallery defaults | Implemented and automated-tested; real appearance and scrollback behavior await the user's check. |
| Linux, Rust 1.88 minimum, other terminals, multiplexers, SSH | Unverified. Protocol hints or upstream support are not lsa validation. |

Automated suite: 36 unit and 12 CLI tests, 16 protocol/cursor scenarios,
48 inline layout/style scenarios, and 23 cache scenarios. Tests verify complete
names, ordering, option precedence, plain pipes, explicit decoration, safe SGR/OSC,
wide/combining names, image frames, limits, lazy cache behavior and fallback.

Both stdin-disconnected and same-foreground-TTY overflow exit normally without a
key press. Non-controlling PTYs verify unchanged termios. On macOS, controlling
PTY attributes can become unreadable after the session leader exits; those trials
check completion and absence of interactive terminal commands instead. No test here
renders the pixels or establishes a visual pass.

## Ghostty check

Run from this repository in Ghostty:

```sh
./target/release/lsa
./target/release/lsa -la --header
./target/release/lsa img-test
./target/release/lsa img-test/generated/many
./target/release/lsa --preview-limit=2 img-test/generated/many
./target/release/lsa --grid img-test/generated
NO_COLOR=1 ./target/release/lsa img-test
./target/release/lsa --no-icons --no-images .
./target/release/lsa --hyperlink -l .
./target/release/lsa img-test/generated/many | cat
./target/release/lsa --diagnose img-test
```

Check icon alignment, light/dark palette readability, full names, symlink labels,
image placement, and that every command returns directly to the prompt. The large
gallery should show up to 16 thumbnails followed by compact names. With a limit of
two, both previews should remain in scrollback and all 40 names should appear once.
Scroll up after subsequent commands to check old previews; lsa never deletes them.
Repeat narrow/wide and after resizing between invocations. No live resize UI exists.

Record terminal/version, OS, cell and pixel geometry, Kitty protocol, inline mode,
and direct/multiplexer/SSH transport for each report. `--diagnose` supplies hints and
measured geometry, not renderer validation. Unknown terminals and multiplexers default
to text; use `--protocol=kitty` only as an explicit experiment.

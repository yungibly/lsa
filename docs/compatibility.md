# Compatibility

2026-09-05. **No real terminal has been visually validated yet.**

| Environment | Protocol / mode / transport | Evidence | Status |
| --- | --- | --- | --- |
| macOS 26.6.2 (25G83), arm64; Rust 1.98.0 | Plain text, stdout pipe | CLI tests, error/closed-pipe tests, benchmarks | Automated pass |
| Same host, Python 3.14.7 PTY; injected Ghostty environment | Kitty inline bytes; local pseudo-terminal | 16 scenarios; 40 placements at 80×24, 80×8, 12×8, 200×50; cursor model starts at top/bottom; modes unchanged | Automated pass; no renderer |
| Same PTY, four user JPEG/GIF/WebP files | Kitty inline bytes | Four decoded images transmitted successfully | Local automated pass; no renderer |
| Ghostty, version not supplied | Kitty inline, direct local terminal | User verification pending | Unverified |
| Kitty terminal; other terminals; SSH | Kitty inline | No terminal trials | Unverified |
| tmux / screen / Zellij | Text in auto mode | Environment fallback tested for tmux; no passthrough implementation | Graphics unverified |
| Any terminal | Interactive browser | Not implemented | — |
| Any terminal | Sixel | Deferred, not implemented | — |

PTY tests check quiet/chunked RGBA framing, decoded lengths, total image bytes,
attempt limits, complete ASCII fixture labels, cursor bounds, text overrides,
narrow/short/unknown-terminal fallback, corrupt/oversized files, and image-named
FIFOs/symlinks. They do not prove image visibility, scrollback, clipping, resize,
prompt interaction, or cleanup in Ghostty. Environment detection is a hint; it
does not read a graphics query response.

## Ghostty checklist

From this repository, in a direct Ghostty shell:

```sh
./target/release/lsa --diagnose
./target/release/lsa --grid img-test
./target/release/lsa --grid img-test/generated
./target/release/lsa --grid img-test/generated/many
./target/release/lsa --grid --preview-limit=2 img-test/generated/many
./target/release/lsa --grid img-test | cat
```

Record the Ghostty version from About or `ghostty --version` if available, OS,
`--diagnose` output, and whether local, SSH, or behind a multiplexer. If detection
selects text in a direct graphics-capable session, try `--protocol=kitty` and record
both outcomes. Pixel dimensions may be estimated; note this when judging aspect.

- All entry types stay in name order. White image borders are intact; portrait
  and landscape shapes are correct; filenames are selectable terminal text.
- Transparency shows a checker. Broken images/links keep readable placeholders.
  The limited run lists all 40 names with only two previews. Piped output is text.
- Run the grid twice, including from the bottom of the terminal. Each listing
  remains intact; the prompt appears below it. Scroll up/down through the output.
- Resize narrower/wider after output and try light/dark backgrounds. Note any
  clipping, reflow, image/text separation, or retention limits rather than assuming
  indefinite scrollback support. Resize during output is not yet handled.

The example fixture generator refuses to overwrite an existing generated directory.
No computer-use/GUI automation was used during implementation.

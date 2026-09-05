# Compatibility

2026-09-05. **Original inline commands are user-verified in Ghostty 1.3.1.**
Automatic layouts and compact columns also received a clear user visual pass at
`364976f`. New directories-first, metadata controls, and cache behavior are
automated-tested; their visual verification remains separate.

| Environment | Protocol / mode / transport | Evidence | Status |
| --- | --- | --- | --- |
| macOS 26.6.2 (25G83), arm64; Rust 1.98.0 | Plain text, stdout pipe | CLI tests, error/closed-pipe tests, benchmarks | Automated pass |
| Same host, Python 3.14.7 PTY; injected Ghostty environment | Kitty inline bytes; local pseudo-terminal | 16 scenarios; 40 placements at 80×24, 80×8, 12×8, 200×50; cursor model starts at top/bottom; modes unchanged | Automated pass; no renderer |
| Same PTY, four user JPEG/GIF/WebP files | Kitty inline bytes | Four decoded images transmitted successfully | Local automated pass; no renderer |
| Ghostty 1.3.1 stable, macOS/CoreText/Metal build; 122×40 cells, 8×17 pixels/cell | Kitty inline; user session, transport not separately stated | User reported all original checklist commands worked as expected at `f079a23` | User-verified commands |
| Ghostty 1.3.1, same reported geometry | Automatic grid and row-wise text columns | User reported all new commands look good; visual testing is a clear pass at `364976f` | User-verified commands |
| Local PTY, 122×40 / 8×17 and boundary cases | Layouts, directories-first, custom long fields | 34 layout scenarios plus the original 16 protocol scenarios | Automated pass |
| Local PTY, 80×24 and 122×40 / 8×17 benchmark geometry | Opt-in cached Kitty inline output | 23 cache PTY/CLI scenarios; empty/warm/disabled output matches exactly; concurrency, failures, and budgets checked | Automated pass; no new Ghostty visual pass |
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

## User report

The user supplied `TERM_PROGRAM=ghostty`, version 1.3.1, `stdout_tty=true`, geometry
122×40, measured cell pixels 8×17, and `kitty=true`. They reported that all original
commands below worked exactly as expected. `layout=text` in the original diagnostic
was expected without `--grid`; the updated diagnostic now examines each operand.
The subsequent user report explicitly passed all new default commands visually.
Directories-first and custom metadata were added after that report.

Build details supplied: stable channel, Zig 0.15.2, ReleaseFast, app runtime `.none`,
CoreText font engine, generic Metal renderer, kqueue libxev. This establishes the
reported session, not every Ghostty build or transport. Specific resize/theme trials
and SSH/multiplexer conditions were not separately logged.

During the default-layout chunk, Computer Use rejected access to
`com.mitchellh.ghostty` for safety reasons. No new window was opened and the user's
existing window was untouched. This tool restriction is not an lsa rendering failure.

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

Default commands, now user-verified:

```sh
./target/release/lsa                         # compact source listing
./target/release/lsa img-test                # automatic grid at 122×40
./target/release/lsa img-test/generated/many # compact text: grid would be too tall
./target/release/lsa --no-images img-test    # compact text override
./target/release/lsa --diagnose img-test     # explain the selected layout
```

New metadata/grouping examples:

```sh
./target/release/lsa --dirs-first img-test
./target/release/lsa --dirs-first -r img-test
./target/release/lsa -lh --dirs-first img-test
./target/release/lsa --fields=size,modified -h img-test
```

These have automated text/order checks; no new visual pass is claimed yet.
Text reads across each row, like the grid. Verify column alignment, complete names,
and ordering. Automatic grids should look like `--grid` for the same directory.

Cache comparison, still awaiting a user visual check:

```sh
./target/release/lsa --cache-dir=benchmarks/local/thumbnails --clear-cache
./target/release/lsa --cache-dir=benchmarks/local/thumbnails --cache-stats img-test
./target/release/lsa --cache-dir=benchmarks/local/thumbnails --cache-stats img-test
./target/release/lsa --cache-dir=benchmarks/local/thumbnails --no-cache img-test
```

Compare images, orientation/checker, filenames, order, and prompt placement. At
the recorded 122×40 geometry, these select the same automatic grid. Warm runs
should look identical and report hits; the cache holds pixels only and uses the
existing anonymous placement/history behavior. A slot collision may cause misses.

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
No Ghostty UI automation was performed.

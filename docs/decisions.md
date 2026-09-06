# Product and implementation decisions

Updated 2026-09-05 after the product reset. The priority is an `ls` alias that feels
right immediately, with useful previews and bounded work.

| Decision | Reason |
| --- | --- |
| Always print and exit | A listing belongs in shell scrollback. Removed automatic/explicit paging, selection, inspection, terminal mode handling, worker queues and image residency management. |
| One mixed ordering | A preview represents an entry. Ordinary files, folders and links remain alongside images; no separate gallery section or hidden failures. |
| Compact text; bounded inline galleries | Image-heavy and single-row mixed listings preview automatically. Tall galleries continue inline, with compact text after the shared budget is spent. |
| 16 default attempts, 8 MiB image commands, 256 placements | Useful initial thumbnails without decoding an entire photo directory. User can raise attempts to 256. Cache hits and failures count; all artwork consumes command bytes and placements. |
| Smaller frames and complete tile artwork | 14×3-cell grid frames with wider, centered text labels. Built-in folder/file/media/error drawings fill every tile; no duplicate font glyph under previews. |
| One-row long-view miniatures | 3×1-cell previews replace the filename icon in a fixed gutter. Preserve metadata/name alignment, full names, source limits, text overrides and narrow-terminal fallback. |
| Shared filename classification | One case-insensitive classification supplies decoder eligibility and file appearance. Unknown types get a generic icon; executable bits do not turn a recognized GIF into a terminal glyph. |
| Automatic color and icons | Terminal palette, optional validated LS_COLORS rules, Ghostty's built-in Nerd glyphs and portable symbols elsewhere. Explicit overrides and plain pipe defaults. |
| Accurate executable styling | Read regular-file mode bits when styling/classifying; retain only a boolean. Plain names skip per-entry metadata. Extra TTY cost is measured explicitly. |
| Natural ASCII-case-insensitive sorting | Numbered photos/source files read naturally, without allocations or integer overflow during comparisons. Non-ASCII and invalid bytes remain deterministic. |
| Readable long output | Size, owner, permissions and local time cover daily needs; numeric ownership, bytes and chosen fields are explicit controls. Account/timestamp caches are bounded and invocation-local. |
| Conservative protocol detection | Direct Ghostty/Kitty hints enable Kitty. Multiplexers/unknown terminals use text. Hints are not real-terminal validation. |
| Anonymous inline Kitty placements | Preserve prior output and avoid collisions. No global deletes, alternate screen, terminal queries, raw mode or stdin handling. |
| Sequential decoding and opt-in cache | Reuse the proven decoder and bounded 64-slot cache; no async runtime, new dependency, implicit storage or service. |
| Widths before decoration | Escape/wrap complete names at grapheme boundaries, then add SGR/OSC framing. The column planner retains widths rather than formatted duplicate names. |

The command vocabulary keeps common `ls` flags and a small set of image/appearance
controls. `--page` and `--inline` were removed because output now has one lifetime.
`-F` is opt-in; default redirected names have no synthetic suffixes. File operands
share a layout in argument order, and directory symlink operands follow familiar
`ls` behavior unless `-l`/`-d` asks for the link itself.

Colors/icons and readable details take inspiration from [eza](https://eza.rocks/).
Ghostty documents its [built-in Nerd Font support](https://ghostty.org/docs/config).
Those sources inform the design, not a claim that lsa's current appearance has
been visually checked. New terminal evidence belongs in [compatibility](compatibility.md).

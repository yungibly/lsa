# Product and implementation decisions

Updated 2026-09-07 after personal-use feedback. The priority is an `ls` alias that feels
right immediately, with useful previews and bounded work.

| Decision | Reason |
| --- | --- |
| Always print and exit | A listing belongs in shell scrollback. Removed automatic/explicit paging, selection, inspection, terminal mode handling, worker queues and image residency management. |
| One mixed ordering | A preview represents an entry. Ordinary files, folders and links remain alongside images; no separate gallery section or hidden failures. |
| Compact text; bounded inline galleries | Image-heavy and single-row mixed listings preview automatically. Tall galleries continue inline, with compact text after the shared budget is spent. |
| 256 default attempts; up to 4,096 explicitly | The 16-preview cutoff hurt real directory browsing. Raise it and retain shared finite budgets. Failures and cache hits count. |
| 128 MiB image commands, 4,096 placements | Enough for 256 maximum-resolution source thumbnails, including artwork to finish the last row. Stream sequentially; the cap is not an allocation. All graphics count and overflow retains every filename. |
| Adjustable grid size | Keep 14×3 cells by default; `--thumbnail-size=1..12` changes height one terminal row at a time. Width, spacing and column count follow; clamp to the terminal and retain 320×240 pixel bounds. Long miniatures stay one row. |
| Explain overridden layout flags | Preserve order-independent text/long precedence but issue one notice for ignored explicit grid/size requests. Help now includes examples and marks long-implying options. |
| Complete tile artwork | Built-in folder/file/media/error drawings fill every tile; no duplicate font glyph under previews. |
| Bounded SVG and ICO | ICO uses the existing image/PNG stack. [resvg 0.48.1](https://github.com/linebender/resvg/tree/v0.48.1) is built without text, system fonts, SVGZ or raster-image features. This release retains a Rust 1.85 minimum and fixes non-finite geometry/bounding-box failures. XML preflight bounds size/nodes/depth and rejects costly resource/expansion features; image resolvers are disabled. This intentionally supports vector artwork rather than every SVG feature. |
| Defer WebM frames and keep scrollback | Video demuxing/decoding would introduce substantial native dependencies or external process/timeout management. The user permits deferral. Measured gallery application cost does not justify bringing back a pager; actual terminal costs still need a user check. |
| Reduce preview copies | Bounded buffered source reads, 4 KiB base64 scratch, and thumbnail-before-EXIF rotation reduce memory without workers. Rotation can slightly change fractional resize-edge pixels; cache namespace/magic advance to v2. |
| Bound artwork raster work to each shape | Fill integer rectangles directly and test polygon pixels only within vertex bounds. Exact output matches before/after; less CPU with no retained cache or additional memory. |
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

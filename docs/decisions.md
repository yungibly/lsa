# Product and implementation decisions

Updated 2026-09-28 for v0.5.0. The priority is an `ls` alias that feels right
immediately, with useful previews and bounded work.

| Decision | Reason |
| --- | --- |
| Always print and exit | A listing belongs in shell scrollback. Removed paging, selection, terminal-mode handling and image residency management. |
| One mixed ordering | A preview represents an entry. Files, folders and links stay beside images; no separate gallery section, no hidden failures. |
| Long terminal default; automatic grids | Details suit everyday directories. Image-heavy and single-row listings still switch to grids (threshold: half the entries are images; open question in the roadmap). Pipes print plain names. |
| Familiar flags where cheap | `-C` fills columns downward like `ls`; `-x` fills rows (and continues exhausted galleries). `-p -v -i -s -o -g -c -u` exist because muscle memory hit exit code 2. `-i`/`-s` imply the long view rather than prefixing every layout. `-o` follows POSIX (no group), not macOS BSD (file flags); `-v` means natural order. `-R` is refused with an explanation. |
| Readable details | Size, owner, permissions and local time. Directory sizes show `-`: the byte count describes the directory index. With `-S`, directories follow sized entries. |
| Compact metadata | 64 bytes of listing and sorting fields per record on 64-bit targets (now including inode and allocated blocks), plus civil-time state only when times are shown. One local-time conversion per entry. |
| Name-only entries, classified once | Entries keep their name; paths are rebuilt from the listing directory when a file is opened or stat'ed. Filename classification happens once per entry, and not at all for plain pipe output. |
| Default foreground for ordinary files | Palette white (37) was nearly invisible on light themes. Categories and kinds keep palette colors. |
| Visible dangling links | One extra stat per symlink, only when output is styled or previewable. They are red (LS_COLORS `or`/`mi`), get broken-link artwork and spend no preview attempt. |
| Bounded LS_COLORS | Up to 4,096 rules and 64-byte keys/values, SGR digits only. Suffixes are hash-indexed; the last matching rule wins; case-insensitive only when no exact match. |
| Hyperlinks without `.` components | `/cwd/./name` looked odd and some consumers may not normalize it. `..` and symlink identity are preserved; nothing is canonicalized. |
| Conservative protocol detection | Direct Ghostty/Kitty hints enable Kitty graphics. Multiplexers, SSH sessions and unknown terminals use text. SSH matches the link policy, because a forwarded `TERM` could stream the whole image budget unasked. An explicit `--grid` that cannot apply prints one notice. |
| Anonymous inline Kitty placements | Image IDs could collide with other programs and change earlier scrollback images; anonymous placements preserve history. No terminal queries, raw mode or stdin handling. |
| Compressed payloads (`o=z`) | zlib level 1 (miniz_oxide, already built for PNG) when it shortens the command: artwork shrinks ~30×, photos ~15-20%. Budgets count the uncompressed length, so limits stay exact. Ghostty decodes `o=z` for chunked direct transmission. |
| Artwork rendered once | Each icon, size and color is rasterized and compressed once per invocation. |
| Bounded parallel decoding | Up to four workers, at most eight results ahead of output, consumed in listing order. Budget decisions are settled before decoding, so output equals sequential drawing. Four workers gave 4.7× on a 4-performance-core Mac; eight gained ~20% more for twice the memory. Worker panics become error artwork. |
| Whole-file reads reserve once | The JPEG decoder reads its whole source. Through the bounded reader, std could not see the length, and repeated reallocation left ~8 MiB per photo resident on macOS. The reader now reserves the validated length. |
| macOS ImageIO for system formats | HEIF, AVIF, TIFF, JPEG XL, PSD and camera RAW (embedded previews), plus JPEG for reduced-resolution decoding. Loaded with dlopen on first use (linking cost 0.7 ms per launch). Reads through pread callbacks on the already validated descriptor; content types are allow-listed, with Rust fallback for anything else. Full-size decodes stay within 16 MP; JPEG/HEIF/AVIF decode at reduced resolution up to 64× larger; larger TIFF/PSD/JXL/RAW use embedded previews. |
| Rust decoders elsewhere | PNG, GIF, WebP, BMP, ICO and SVG keep the bundled decoders on every platform, as does everything on Linux. Deterministic pixels, no system dependency. |
| Bounded SVG | resvg without text, fonts, raster images or SVGZ. XML preflight bounds size, nodes and depth and rejects expansion-heavy features; unsupported documents fail whole. |
| Defer video frames | Demuxing and decoding video needs a codec stack or external process and timeout management. |
| Opt-in cache | Bounded 64-record cache, no scans or hit writes. Records are versioned per platform because pixels depend on the decoder. Worker threads share one lock-protected handle; locks never cover decoding. |
| Widths before decoration | Escape and wrap complete names at grapheme boundaries, then add SGR/OSC framing. Column plans keep widths, not formatted copies. |
| Natural ASCII-case-insensitive sorting | Numbered files read naturally, without allocation or integer overflow. Non-ASCII bytes keep a deterministic order; this is not locale collation. |

The command vocabulary keeps common `ls` flags and a small set of image and
appearance controls. Colors, icons and readable details take inspiration from
[eza](https://eza.rocks/); Ghostty documents its
[built-in Nerd Font support](https://ghostty.org/docs/config). New terminal
evidence belongs in [compatibility](compatibility.md).

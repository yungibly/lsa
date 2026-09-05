# Prototype decisions

2026-09-05. Revisit from measurements and Ghostty feedback.

| Choice | Reason / boundary |
| --- | --- |
| Rust, Unix first | Toolchain available; native executable, lossless Unix names, and selected Rust image codecs. No async runtime. macOS arm64 build verified; Linux pending. |
| Kitty only | User narrowed scope. No multi-backend trait or Sixel dependency before a second backend is useful. |
| Small direct Kitty writer | Inline lifetime needs ordered rows, no alternate screen or event loop. Quiet RGBA chunks, anonymous placements, cursor control, and exact byte accounting are enough for this experiment. |
| `image` with five format features | One decoder interface; no external image executables or native codec libraries in the tested build. Default features disabled. JPEG orientation and offset GIF first-frame behavior tested. |
| `libc`, `unicode-width`, `unicode-segmentation`, `base64` | Window size, safe regular-file opens, Unix metadata/time; grapheme-aware text wrapping; protocol encoding. Five direct runtime crates total, including `image`. |
| Automatic layout, environment hints | After the explicit grid was user-verified, add conservative defaults: ≥4 candidates, ≥50%, full grid within screen minus two rows and configured preview caps. Text wins on pipes; explicit overrides win on TTYs. No queries, raw mode, or stdin consumption. |
| Row-wise text columns | Preserve the grid reading order. Measure escaped labels with Unicode display widths, retain only widths, search at most 64 column counts, and omit trailing padding. A very long name can reduce the listing to one column. |
| Shared grid geometry | Selection and rendering use the same tile size and wrapped-label height. Large directories fail the cheap minimum-height check before candidate classification or label formatting. Diagnose can deliberately calculate full estimates. |
| Sequential decode, no cache | Keeps concurrency/queues at one/zero and exposes cold-thumbnail work. Source/geometry/allocation/output caps first; measure before adding workers/cache. No hard decode deadline. |
| Box optional metadata | The first 10,000-file baseline showed unused stat storage inflated ordinary listing memory. Boxing requested metadata cut measured peak RSS about 46%. Names/path collection still scales with directory size. |
| Fixed inline rows | Reserve vertical room before placements, leave rightmost column unused, wrap names, flush each tile/row. Decode with cursor below placements. Nothing rewrites old rows after emission. |

Reviewed [ratatui-image's widget model](https://docs.rs/ratatui-image/latest/ratatui_image/):
it remains a browser candidate; introducing its rendering lifecycle is unnecessary
for this inline experiment. This was an API/dependency review, not a comparative
benchmark of TUI stacks.

The [Kitty specification](https://sw.kovidgoyal.net/kitty/graphics-protocol/) defines
direct transmission, 4096-byte base64 chunks, quiet replies, and cursor control.
Automated tests decode emitted payloads and model the cursor subset; the original
inline renderer is user-verified in Ghostty 1.3.1. New default selection/columns
remain headless-tested. Anonymous inline images avoid ID reuse across
invocations. Browser image ownership will need a separate implementation.

[`image::Limits`](https://docs.rs/image/0.25.10/image/struct.Limits.html) distinguishes
strict dimension checks from best-effort allocation limits. lsa additionally caps
source bytes, total pixels, decoded output, attempts, thumbnail size, and emitted
image bytes. In-process codec internals, allocator overhead, filesystem waits,
entry-list memory, and terminal residency are not covered by a hard memory/time
guarantee. No shrink-on-load, ICC workflow, animation, or cache claims yet.

`otool -L target/release/lsa` showed only macOS `libSystem` and `libiconv` runtime
links. Dependencies and versions are pinned in `Cargo.lock`. Cargo downloads were
kept in the gitignored repository-local `.cargo-home/`.

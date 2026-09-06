# Prototype decisions

2026-09-05. Revisit from measurements and Ghostty feedback.

| Choice | Reason / boundary |
| --- | --- |
| Rust, Unix first | Toolchain available; native executable, lossless Unix names, and selected Rust image codecs. No async runtime. macOS arm64 build verified; Linux pending. |
| Kitty only | User narrowed scope. No multi-backend trait or Sixel dependency before a second backend is useful. |
| Small direct Kitty writer | Inline lifetime needs ordered rows, no alternate screen or event loop. Quiet RGBA chunks, anonymous placements, cursor control, and exact byte accounting are enough for this experiment. |
| `image` with five format features | One decoder interface; no external image executables or native codec libraries in the tested build. Default features disabled. JPEG orientation and offset GIF first-frame behavior tested. |
| `libc`, `unicode-width`, `unicode-segmentation`, `base64`, `crc32fast` | Window size, safe regular-file opens, Unix metadata/time and cache locks; grapheme-aware text wrapping; protocol encoding; cache checksums. Six direct runtime crates total, including `image`. CRC32 was already a locked transitive dependency. |
| Automatic inline layout, environment hints | After the explicit grid was user-verified, add conservative defaults: ≥4 candidates, ≥50%, full grid within screen minus two rows and configured preview caps. Text wins on pipes; explicit overrides win on TTYs. Inline output makes no queries, enters no raw mode, and consumes no stdin. |
| Row-wise text columns | Preserve the grid reading order. Measure escaped labels with Unicode display widths, retain only widths, search at most 64 column counts, and omit trailing padding. A very long name can reduce the listing to one column. |
| Shared grid geometry | Selection and rendering use the same tile size and wrapped-label height. Large directories fail the cheap minimum-height check before candidate classification or label formatting. Diagnose can deliberately calculate full estimates. |
| Sequential decode, optional cache | Decode concurrency/queues remain one/zero. The opt-in cache skips decoding on hits while preserving source checks and attempt/output budgets. No hard decode deadline. |
| Fixed cache slots and raw RGBA | The four-image warm case falls from 71.76 to 4.60 ms. 64 replaceable slots bound storage without an index, scans, or writes on hits; collisions may lower hit rate. Versioned full keys and checksums reject stale/corrupt records. One staging file and nonblocking local locks bound concurrent writes; atomic rename publishes complete pixels. Details in [cache design](cache.md). |
| Eight candidates per cache key | Measured direct mapping hit only 56.25% for 32 independent sources. Eight-slot groups reached 100% / 15.97 ms versus 216.42 ms, keeping the 64-record cap. A four-slot trial also helped, but retained misses in that case. Header probes have a fixed bound. Randomly seeded victim selection avoids repeated-listing FIFO churn without writes on hits; overloaded working sets still have limited benefit. |
| Directories-first grouping | Sort actual directory entries ahead of others, then apply the selected sort/reverse within groups. Reuse existing entry kinds; no symlink-target or extra metadata lookup. |
| Configurable long fields | `--fields` selects/reorders a bounded six-field vocabulary and implies long text. Filenames always remain. Numeric IDs keep account lookup out of the pipeline; no xattr/ACL expansion yet. |
| Content-sized long columns | Two passes retain only six widths and format ASCII fields as needed. This adds ~3.3 ms to the 10,000-entry long case versus fixed widths, with similar RSS; the ordinary text path is unchanged. |
| Box optional metadata | The first 10,000-file baseline showed unused stat storage inflated ordinary listing memory. Boxing requested metadata cut measured peak RSS about 46%. Names/path collection still scales with directory size. |
| Fixed inline rows | Reserve vertical room before placements, leave rightmost column unused, wrap names, flush each tile/row. Decode with cursor below placements. Nothing rewrites old rows after emission. |
| One-listing image pager | The user rejected the browser's presentation and scope after `59f5cec`, and explicitly chose a simple pager. `--page` reads one mixed listing; remove directory navigation, history, refresh, and revision state. Inline remains the default. |
| Small pager terminal guard and `pselect` | Retain the tested separate terminal lifecycle without an event runtime or new dependency. Restore on quit/errors/signals and before suspend. One Escape timeout, no idle refresh loop. |
| Spatial keys and conventional paging | Arrows/hjkl follow grid columns/rows. Space/b and PgDn/PgUp page; Enter opens name inspection. Directory entries have no navigation actions. |
| Clipped overview plus full-name inspection | Bound redraws to 512×256 while keeping complete names accessible through Enter. Tile-width selection and a selected-name status improve orientation without introducing a richer TUI. |
| One pager decoder worker | The preceding browser measured ~3 ms to names while four previews took ~79 ms at a drained PTY. Retain one outstanding request/completion and the sequential decoder's resource bounds; additional concurrency is not justified. Those historical timings are not a new pager measurement. |
| Viewport plus two entries | Selected source first, then visible candidates and one entry on each side. Retain at most 34 records, place at most 32 images, and reject stale viewport/geometry results. Cache defaults stay unchanged. |
| Per-viewport pager budgets | A lifetime cap prevents useful scrolling of large image directories. Renew attempt/output allowances on viewport changes, counting retained records and reserving resident-image cleanup. Selection redraws cannot renew them. Each batch remains bounded; cumulative interactive-session output can grow with user input. Inline caps remain invocation-wide. |
| Numbered pager images, separate writer | Retain randomized Kitty image numbers, one movable placement per number, targeted deletion, and reserved cleanup bytes. Text-row erasure preserves placements. No global deletion or anonymous inline ownership reuse. |
| No worker join on quit | In-process decoders/filesystem calls lack safe cancellation. Drop queued work, discard stale pixels, restore the terminal, and let process exit end a remaining worker. A blocked-loader test and JPEG input trial verify responsiveness; directory reads/output can still block the main thread. |

The earlier review of [ratatui-image's widget model](https://docs.rs/ratatui-image/latest/ratatui_image/)
was an API/dependency review, not a TUI benchmark. The current direction is a
simple pager; there is no pending framework adoption or rich-browser roadmap.

The [Kitty specification](https://sw.kovidgoyal.net/kitty/graphics-protocol/) defines
direct transmission, 4096-byte base64 chunks, quiet replies, and cursor control.
Automated tests decode emitted payloads and model the cursor subset; the original
inline renderer is user-verified in Ghostty 1.3.1. Default selection/columns
are now user-verified as well. The user passed metadata/grouping and cache commands
at `7050349`; subsequent replacement changes have automated output equivalence
and compatibility coverage. Anonymous inline images avoid ID reuse across
invocations. The pager retains a separate numbered-image writer and ownership
guard; its graphics behavior is automated-tested and awaits a Ghostty visual pass.

[`image::Limits`](https://docs.rs/image/0.25.10/image/struct.Limits.html) distinguishes
strict dimension checks from best-effort allocation limits. lsa additionally caps
source bytes, total pixels, decoded output, attempts, thumbnail size, and emitted
image bytes. In-process codec internals, allocator overhead, filesystem waits,
entry-list memory, and terminal residency are not covered by a hard memory/time
guarantee. No shrink-on-load, ICC workflow, or animation claims. Cache storage has
its own file-content cap; it does not change decoder limits or terminal residency.

`otool -L target/release/lsa` showed only macOS `libSystem` and `libiconv` runtime
links. Dependencies and versions are pinned in `Cargo.lock`. Cargo downloads were
kept in the gitignored repository-local `.cargo-home/`.

# Image pager

`lsa --page [DIRECTORY]` pages one listing. It replaces the rejected browser
experiment; `--browse` now reports the replacement. Directory navigation, history,
refresh, and a richer file-browser interface are outside this scope.

```sh
./target/release/lsa --page img-test/generated/many
./target/release/lsa --page --dirs-first img-test/generated
./target/release/lsa --page --no-images img-test
```

One sorted collection contains images, ordinary files, directories, links, and
special files. Membership and order are read once, with the existing sorting and
hidden flags. A directory symlink supplied as the operand is resolved once; links
inside the listing retain their identity. Enter inspects an entry and never enters
it. There are no file mutations, external launches, or clipboard actions.

| Key | Action |
| --- | --- |
| Left/Right or h/l | Previous/next grid column, without wrapping to another row |
| Up/Down or k/j | Previous/next grid row; one entry in text mode |
| Space, PgDn, or Ctrl-F | Next viewport |
| b, PgUp, or Ctrl-B | Previous viewport |
| Home/End or g/G | First/last entry |
| Enter or Tab | Inspect complete escaped name, kind, and full path |
| Arrows, pages, Home/End in inspection | Scroll detail text vertically |
| Esc, Backspace, or Enter in inspection | Close inspection |
| q or Ctrl-D | Quit with exit 0 |
| Ctrl-C | Restore the terminal and exit 130 |
| Ctrl-Z, then shell `fg` | Restore before suspending; redraw on resume |

Left/Right have no action in a one-column text listing or in inspection. At a
partial final grid row, Down selects its nearest available entry. Backspace and
Esc outside inspection do not navigate or quit. Long overview labels end in `>`;
inspection wraps and scrolls through the complete name/path. Control characters
and invalid bytes use the same escaping as inline output.

Known Kitty/Ghostty sessions (or `--protocol=kitty`) use a mixed grid when there
are preview candidates and at least 12×10 cells. Names appear before asynchronous
previews. Unknown terminals/multiplexers, `--no-images`, `--protocol=none`, and
non-image directories use text. Below 12×5 a resize message replaces the screen;
selection survives resizing. Five image rows, a filename row, and a gap use the
inline thumbnail geometry, including aspect, orientation, and transparency.
The selected filename fills its tile width; the status also shows its name.

Paging requires stdin and stdout on the same foreground cursor-addressable TTY.
Pipes fail before input is consumed or modes change. Multiple paths and inline
layout/diagnostic options (`--grid`, `-1`, `-l`, `--fields`, `--diagnose`,
`--clear-cache`) are rejected with exit 2. Initial listing and terminal I/O errors
exit 1; partial listing errors retain readable entries and a status message.

## Work and output limits

One lazy worker uses the existing source validation/cache/decoder. Only one job
or completion is outstanding. Selected candidates have priority, followed by the
viewport, then one entry on each side. Viewport/geometry changes invalidate old
work; both worker and main loop reject stale results. Overlap keeps ready pixels
and moves existing placements instead of uploading them again.

There are at most 32 visible images and 34 retained viewport/margin records.
At 320×240 RGBA, retained pixels use at most 10,444,800 bytes, plus one worker
result/in-flight canvas and one base64 buffer. Source limits remain 32 MiB input,
16 million pixels, 16,384 per axis, 64 MiB decoded output, and a best-effort decoder
allocation cap. Sorted entries scale with directory size; these are not hard
process-memory or time bounds.

`--preview-limit` allows 64 attempts by default, at most 256, **per viewport**.
A new viewport receives that allowance minus retained records, including failed
previews; hits and prefetch consume attempts. An old in-flight decode may finish
and be discarded before the new batch starts. Output is capped at 8 MiB of image
commands per viewport change, counting uploads/moves and reserving deletion bytes
for all owned images. `[limit]` keeps entries visible when either allowance runs out.

Scrolling to a different viewport, changing geometry, or restoring from inspection
or suspend renews the allowances. Selection-only and completion redraws do not.
Thus a long session can exceed 64 attempts and 8 MiB in total without growing
memory or queues. This differs deliberately from the inline invocation-wide caps.
No background work continues once a viewport settles.

Listing membership/order is a snapshot; file contents are read when their preview
is requested. External edits may appear on a later decode. There is no watcher or
refresh key; rerun to enumerate changes. Optional cache settings and source checks
are shared with inline output. Text and zero-attempt paths never open storage.
`--cache-stats` prints completed worker counters after restoration; an abandoned
operation may not be in that snapshot.

## Terminal lifetime

Pager images use randomized Kitty image numbers, one placement each, and targeted
cleanup before leaving the alternate screen or suspending. Inline images retain
their independent anonymous/history lifetime. Grid redraws erase text rows, keeping
placements until explicitly moved/deleted; full-screen erasure follows cleanup.
The writer's protocol was checked during the preceding browser experiment against
the [Kitty specification](https://sw.kovidgoyal.net/kitty/graphics-protocol/).
Terminal eviction can remove previews; SIGKILL or a vanished terminal cannot
guarantee cleanup. Visible Ghostty retention remains a user check.

The terminal guard owns raw mode, cursor/paste state, and temporary signal handlers.
Managed signals are blocked before worker creation; main-thread `pselect` atomically
unmasks them while waiting, giving input priority over completions. The worker
waits on a condition variable. Input batches are 256 bytes; escape buffers are at
most 32 bytes; bracketed paste is ignored. A lone Escape uses one 100 ms timeout.
Text redraw geometry is capped at 512×256. No new dependency was added.

A codec/filesystem call cannot be cancelled. It may delay newer previews; quitting
drops scheduling without joining the worker, and process exit ends remaining work.
No worker survives as a service. Initial enumeration and terminal writes remain
synchronous and can delay input/signals. Inline output uses neither this worker
nor this terminal session.

## Verification

Controlling-PTY tests check spatial movement, paging, fixed listing membership,
full names, safe input, resize, cache equivalence, viewport bounds and budgets,
scoped cleanup, signals, suspend/resume, and idle output. A 100-image trial passes
both former whole-session limits, while oversized-view tests enforce each batch's
byte cap. Controlled workers verify stale pixels and prompt drop during a decode.
These checks model commands and pixels; they do not render a terminal.

The earlier browser's text pass and [saved measurements](../benchmarks/browser.json)
are historical evidence. The replacement pager needs its own
[Ghostty review](compatibility.md#pager-checklist); no visual polish claim is made
from automated tests.

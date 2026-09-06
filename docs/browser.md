# Browser

`lsa --browse [DIRECTORY]` explicitly enters an alternate screen. The text browser
received a user pass at `1ea10e5`; the new image slice has automated PTY coverage
and still needs a Ghostty visual pass. One sorted collection contains images,
ordinary files, directories, links, and special files. No file mutation, external
application launch, or clipboard action is performed.

On Kitty/Ghostty environment hints (or `--protocol=kitty`), directories with preview
candidates use a mixed thumbnail grid when the terminal is at least 12×10. Names
and placeholders appear first; previews fill in progressively. Other directories,
unknown terminals/multiplexers, `--no-images`, and `--protocol=none` use the text
browser. Preview failures retain their tiles and names. Browser stdin and stdout
must share one foreground cursor-addressable terminal; pipes fail before reading
input or changing modes.

```sh
./target/release/lsa --browse --dirs-first img-test
./target/release/lsa --browse img-test/generated/many
./target/release/lsa --browse --no-images img-test
./target/release/lsa --browse --cache-dir=benchmarks/local/thumbnails img-test
```

| Key | Action |
| --- | --- |
| Up/Down or k/j | Previous/next entry in listing order (also across grid rows) |
| PgUp/PgDn or Ctrl-B/Ctrl-F | Move one viewport |
| Home/End or g/G | First/last entry |
| Enter, Right, or l | Enter a directory; inspect a regular file's full name |
| Left, h, or Backspace | Return along the navigation route, then use the physical parent |
| Space or Tab | Inspect the complete escaped name, entry kind, and full path |
| Up/Down, pages, Home/End in inspection | Scroll detail text |
| Esc, Space, Enter, or Left in inspection | Close inspection |
| r in the listing | Refresh, retaining the selected raw filename when present |
| q or Ctrl-D | Quit with exit 0 |
| Ctrl-C | Restore the terminal and exit 130 |
| Ctrl-Z, then shell `fg` | Clean up/restore before suspending; re-enter and redraw on resume |

Long overview labels end in `>`. The detail view wraps at grapheme boundaries and
scrolls through the complete name/path, including content longer than a screen.
Controls and invalid bytes use the same escaping as inline output. Images are
released while inspecting text; closing inspection requests previews again.

Resize preserves selection and adjusts the viewport. Grid geometry includes current
cell pixels (8×16 estimated when unavailable), with five image rows, one clipped
label row, and a gap. It reuses the inline thumbnail dimensions, aspect preservation,
orientation, and transparency handling. Below 12×10, the browser switches to text;
below 12×5 it shows a resize message and still accepts keys.

Navigation resolves directory symlinks only when explicitly entered; labels and
sorting retain link identity. Returning restores the route's selected link even
when its target has a different physical parent. Initial directory operands resolve
to physical paths. At most 64 lightweight return bookmarks are retained, with no
historical directory lists. Refresh falls back to the nearest index if the selected
name disappears. Navigation/refresh errors leave the old view usable; partial
listing errors retain available names and show a status message.

`--browse` rejects multiple operands and inline layout/diagnostic operations
(`--grid`, `-1`, `-l`, `--fields`, `--diagnose`, `--clear-cache`) with exit 2.
Sorting/hidden flags and optional cache controls apply. Search remains later work.
A handled navigation error does not make a later `q` unsuccessful. Terminal I/O
errors exit 1; SIGINT/TERM/HUP/QUIT restore and exit 128 + signal number.

## Preview work and limits

One lazy worker runs the existing source validation/cache/decoder. It has one
outstanding request or completion, with bounded mailbox storage and a wake socket.
Selection gets priority, then visible candidates, then one entry on each side of
the viewport. No additional jobs are submitted while a request is outstanding.
Viewport, directory/refresh revision, and geometry changes invalidate old work.
The worker checks the generation before and after loading; the main loop checks
it again before retaining any result. Overlapping entries keep their ready pixels.

At most 32 images are placed, with at most 34 retained viewport/margin records.
Only visible images are sent to the terminal. At 320×240 RGBA, retained thumbnail
pixels are at most 10,444,800 bytes, plus at most one worker result/in-flight canvas
and one base64 transmission buffer. The existing source/decode limits still apply:
32 MiB input, 16 million pixels, 16,384 per axis, 64 MiB decoded output and a
best-effort decoder allocation cap. These are not a hard process-memory bound.
Sorted entries still scale with directory size; the worker stack and codec
allocations add overhead.

`--preview-limit` remains a **session-wide** budget: 64 dispatched attempts by
default, maximum 256, including cache hits, failed/stale requests, and prefetch.
The 8 MiB session image-command cap counts uploads and placement moves, reserving
space for each image's eventual cleanup. Limits do not reset on navigation, refresh,
resize, detail view, or suspend. Once exhausted, tiles show `[limit]`; filenames and
navigation remain usable. Returning to a released view may use more attempts/bytes.
These initial limits favor predictable bounds; tune them from actual browsing use.

An in-progress decode/filesystem call cannot be interrupted safely; it may delay
new previews, but it does not block keyboard handling. Quitting drops the worker
without joining it, and process exit terminates remaining work. No child process
or background service survives. Directory enumeration and output transport still
run on the main thread and may delay input/signals; there is no hard I/O timeout.

Caching stays opt-in. The worker opens storage only for a budgeted load, sharing
the existing keys/limits and pixel format. Text mode, zero attempts, and unavailable
output budget do not access storage. `--cache-stats` reports completed worker stats
after restoration; a decode abandoned at process exit may not appear in that
snapshot. Files are not watched: use `r` to refresh images after external edits.

## Terminal ownership and idle behavior

Browser transmissions use randomized nonzero Kitty image numbers (`I`) and one
placement per image (`p=1`). Moves reuse that placement; releases address its number
with `d=N`. Cleanup is limited to tracked browser numbers and runs before leaving
the alternate screen, on error/unwinding, and before suspension. No global image
deletion or inline image IDs are used. A new image-number transmission does not
replace an existing image with that number. Concurrent programs sharing the same
alternate screen are not a supported ownership environment.

Grid redraws erase text rows with EL, preserving graphics until explicitly moved
or deleted; full-screen erasure is used only after browser images are released.
These choices follow the [Kitty specification](https://sw.kovidgoyal.net/kitty/graphics-protocol/).
Terminal eviction can still remove a preview without a response (`q=2`); names remain
readable and `r` can retry within the remaining budget. SIGKILL or a vanished terminal
cannot guarantee cleanup. Visible Ghostty retention remains a separate user check.

The terminal guard owns raw mode, alternate screen, cursor/paste state, and temporary
signal handlers. Managed signals are blocked before spawning the worker, which
inherits the mask. The main `pselect` atomically unmasks them while waiting on input
and worker completion; input wins when both are ready. The worker blocks on a
condition variable. Neither thread polls or refreshes while idle. A lone Escape
uses one 100 ms disambiguation timeout. Input batches are 256 bytes; escape parsing
retains at most 32 bytes and ignores bracketed paste. Text redraws cap at 512×256;
grid placement also caps at 32 images. No new dependency was introduced.

Inline output retains its synchronous decoder and anonymous/history placement
lifetime. It never enters the browser event loop or starts this worker.

## Verification

39 unit + 8 CLI tests, 25 text-browser scenarios, and 21 browser-image scenarios
cover bounded scheduling, stale results, viewport placement, ordering/selection,
full names, safe input, narrow/large resize, cache equivalence, attempts/output,
cleanup, signals, and suspend/resume. The existing 16 protocol, 34 layout, and
23 cache scenarios also pass. Tests use a foreground controlling PTY and model
commands/pixels/ownership; they do not render graphics.

[Browser measurements](../benchmarks/README.md#browser-previews-2026-09-05) separate
first-name/preview readiness and input response from child CPU/RSS. The user's
text-browser pass belongs to `1ea10e5`; new graphics need the
[Ghostty checklist](compatibility.md#browser-image-checklist).

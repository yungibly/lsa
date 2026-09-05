# Text browser

`lsa --browse [DIRECTORY]` explicitly enters an alternate screen. The current
slice shows one ordered collection of names, with the same entry types, hidden
filtering, suffixes, and sorting as inline listings. It does not decode images,
initialize thumbnail storage, open files in other applications, or change files.
The browser requires stdin and stdout on the same foreground, cursor-addressable
terminal. Pipes fail with exit 1 before reading input or changing terminal modes.

```sh
./target/release/lsa --browse --dirs-first img-test
./target/release/lsa --browse -aSr img-test/generated
./target/release/lsa --browse img-test/generated/many
```

| Key | Action |
| --- | --- |
| Up/Down or k/j | Move selection one entry |
| PgUp/PgDn or Ctrl-B/Ctrl-F | Move one viewport |
| Home/End or g/G | First/last entry |
| Enter, Right, or l | Enter a directory; inspect a regular file's full name |
| Left, h, or Backspace | Return along the navigation route, then use the physical parent |
| Space or Tab | Inspect the complete escaped name, entry kind, and full path |
| Up/Down, pages, Home/End in inspection | Scroll the detail text |
| Esc, Space, Enter, or Left in inspection | Close inspection |
| r | Refresh the directory, retaining the selected raw filename when present |
| q or Ctrl-D | Quit with exit 0 |
| Ctrl-C | Restore the terminal and exit 130 |
| Ctrl-Z, then shell `fg` | Restore before suspending; re-enter and redraw on resume |

Long names are clipped with `>` in the overview. The detail view wraps at grapheme
boundaries and scrolls vertically, including when a name/path needs more than one
screen. Controls and invalid bytes use the same escaping as inline output.
Resize preserves selection and adjusts the viewport; terminals smaller than
12 columns × 5 rows show a resize message and still accept quit/navigation keys.

Navigation resolves a directory symlink only when explicitly entered. Its listing
label and sorting group remain those of a link. Returning restores the link's
selection even when its target has a different physical parent. Initial directory
operands are resolved to their physical path. The browser retains at most 64
navigation bookmarks (path, raw selected name, index, viewport); older visits are
dropped. It rereads directories on return instead of retaining old entry lists.
If the selected name disappears, refresh chooses the nearest surviving index.

Navigation or refresh failures leave the previous listing usable and display an
error in the status line. An initial unreadable/non-directory operand fails before
entering the alternate screen. Partial directory-read errors keep available names
and show a status message. A handled browsing error does not make a later `q`
unsuccessful; terminal I/O errors exit 1. SIGINT/TERM/HUP/QUIT restore and exit with
128 + signal number. SIGKILL and terminal disappearance cannot promise restoration.

The first slice is names-only. `--browse` rejects multiple operands and inline
layout/diagnostic operations (`--grid`, `-1`, `-l`, `--fields`, `--diagnose`,
`--clear-cache`) with exit 2. `-a`/`-A`, `-r`, `-t`, `-S`, and `--dirs-first` apply.
Image/cache settings have no work to perform in this slice. Search and preview jobs
remain subsequent work.

## Resource and lifetime boundaries

One thread reads the current directory synchronously. Sorted entry storage scales
with that directory, as in inline mode. There are no image jobs, background scans,
filesystem watchers, cache operations, or idle refreshes. An idle session blocks
in `pselect` until input or a signal; a lone Escape uses one 100 ms disambiguation
timeout. The input buffer is 256 bytes, escape parsing retains at most 32 bytes,
and bracketed paste is ignored rather than interpreted as keyboard commands.

Each redraw formats only the visible entries, capped at 512 columns × 256 rows,
leaving the last column free. The selected-name detail view retains only one
escaped name/path and its wrapped lines. This is a full text-screen redraw after
input/resize, without a timer; terminal rendering cost is not measured by PTY
tests. Listing and filesystem operations can delay input/signals while running.

The browser owns raw mode, alternate-screen entry/exit, cursor visibility,
bracketed-paste mode, and temporary signal handlers. A guard restores terminal
attributes and display state on normal/error exits and unwinding. An independent
nonblocking controlling-terminal input descriptor avoids changing the shell's
shared file-status flags. Managed signals stay blocked during work and are
atomically unblocked while waiting, avoiding a lost-wakeup race. No new crate was
needed beyond the existing `libc` and Unicode dependencies.

Inline output never initializes this lifecycle, reads stdin, or changes terminal
modes. Future viewport previews need their own bounded jobs and session-owned
image IDs/cleanup; anonymous inline placements are not browser image ownership.

## Verification

`python3 tests/check_browser.py` drives the release binary through a foreground
controlling PTY. It checks ordering against ordinary listings, navigation and
symlinks, selection/refresh, full-name inspection, control/Unicode display,
fragmented input and paste, screen bounds/resize, signals, suspend/resume,
restoration, pipe rejection, idle CPU, and absence of graphics/cache output.
Unit tests cover input parsing, lossless detail wrapping, and stable selection.

These are application/PTY checks, not visible Ghostty validation. The new browser
still needs the user's [Ghostty checklist](compatibility.md#browser-checklist).

On the current macOS/Rust 1.98.0 build, 25 browser scenarios pass alongside the
existing 16 protocol, 34 layout, and 23 cache checks. One empty-directory idle trial
emitted no output for one second and used 1.87 ms child CPU including startup/quit
(1.105 seconds observed including quit). No process RSS or terminal renderer cost
was measured. On macOS the harness reads attributes from the PTY master after
session exit, and excludes the transient PENDIN retype bit when checking suspension.

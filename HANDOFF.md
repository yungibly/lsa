# Fresh-session handoff

2026-09-05; this chunk implements the explicit text browser described in the prior
handoff. Starting Git state was clean at `d60d99c`. Read README → ROADMAP, then
check Git rather than assuming this snapshot is current.

## Working agreement and user verification

- Work/access files only inside this repo; keep Cargo storage and experiments here.
- Routine implementation and concise local commits are authorized; no per-chunk
  confirmation. Docs are working suggestions, not rigid requirements.
- The user handles Ghostty visual checks; do not automate terminal applications.
  Preserve the four original ignored images under `img-test/`.
- Inline commands/layouts have prior user passes in Ghostty 1.3.1, 122×40 / 8×17,
  arm64 macOS 26.6.2. The user passed the metadata/grouping and cache command set
  at `7050349`. That report did not resupply transport/theme/resize conditions.
- **The new browser has no Ghostty visual pass yet.** PTY checks establish bytes,
  interaction, and terminal attributes, not visible screen/scrollback behavior.
- Keep one ordered mixed listing, cheap text, complete filename access, Kitty
  first, inline default, explicit browsing, and distinct image/output lifetimes.

## This chunk

- Added `--browse [DIRECTORY]`, a text-only alternate-screen browser. It reuses
  existing entries/sorting/hidden filtering and suffixes. One directory operand;
  `-l`, `--fields`, `-1`, `--grid`, `--diagnose`, `--clear-cache`, and multiple
  operands are rejected when combined with browsing.
- Arrows/j/k move; PgUp/PgDn page; g/G and Home/End jump; Enter/right/l enters a
  directory; h/Left/Backspace returns along the route, then to physical parents.
  Explicitly entered directory links resolve, but keep their listing identity.
- Selection survives resize, refresh (`r`), and return by raw filename, falling
  back to the nearest index if removed. Navigation history is at most 64 small
  bookmarks; old entry lists are not retained. Failed navigation/refresh keeps
  the previous view usable with an error status.
- One entry per overview row; long labels show `>`. Space/Tab opens the complete
  escaped name, kind, and path with grapheme wrapping and vertical scrolling.
  Escape closes the detail view. Enter on a regular file also shows details.
- Browser-only terminal guard owns raw mode, alternate screen, cursor, paste mode,
  and temporary signal handlers. q/Ctrl-D exits 0; Ctrl-C exits 130;
  SIGTERM/HUP/QUIT exit 128 + signal. Ctrl-Z restores before stopping and `fg`
  re-enters/redraws with selection intact. Error exits and unwinding also restore.
- Require stdin/stdout on the same foreground terminal. Reject pipes before input
  consumption or mode changes. Open an independent nonblocking controlling-TTY
  input descriptor so the shell's shared descriptor flags remain unaffected.
- Block idle in `pselect`, atomically unblocking managed signals while waiting.
  No periodic polling or refresh; a lone Escape has a 100 ms timeout. Input batches
  are at most 256 bytes; escape state stores at most 32 bytes; bracketed paste is
  ignored. Redraw only visible rows, capped at 512×256, leaving the last column free.
- No image decoding, preview jobs, cache access, image deletion, search, external
  open/copy actions, new dependencies, or file mutations. Directory reads are
  synchronous and can delay input/signals. Inline rendering behavior is unchanged.

Details: [browser controls/design](docs/browser.md), [decisions](docs/decisions.md),
[Ghostty checklist](docs/compatibility.md#browser-checklist).

## Evidence

- 36 unit + 8 CLI tests; fmt, clippy, release binaries/examples build pass.
- 25 new browser controlling-PTY/CLI scenarios pass, including comparison to normal
  listing order, all sorting flags, links/parent navigation, refresh after changes,
  full name/path content, fragmented keys/paste, safe Unicode/control names,
  empty/failing paths, resize down to 1×1 and up to 1000×1000 (render capped),
  termination signals, Ctrl-C, Ctrl-Z/continue, pipes, and unconsumed redirected input.
- Idle empty-directory trial: no bytes during a 1-second idle window; 1.105 seconds
  including quit observation, 1.87 ms child user+system CPU including startup/quit.
  This is an application/PTY trial, not terminal rendering or an RSS benchmark.
- Existing 16 protocol PTY + 34 layout + 23 cache scenarios pass. No new cache or
  inline performance claims; the cache working-set benchmarks were not rerun.
- Release binary: 1,330,128 bytes. Rust/Cargo 1.98.0 on the existing macOS host;
  Linux and declared MSRV 1.88 remain unverified.
- Harness details: drain output while polling process exit; inspect terminal modes
  via the PTY master after a controlling session exits (macOS revokes the slave).
  On suspend, mask macOS's transient PENDIN retype bit for comparison; all other
  attributes/control bytes must match. Check that shared input/output descriptors
  do not acquire O_NONBLOCK. Initial test-harness failures were fixed, not waived.

## Resume here

Get the user's Ghostty browser pass before treating visible interaction/restoration
as verified. Provide the checklist, including mixed selection, navigation/return,
Space/full-name scrolling, resize, q/Ctrl-C, Ctrl-Z/`fg`, and prior inline-image
retention after entering/quitting the browser. Fix any reported issues.

Then add viewport-driven image jobs, a small prefetch margin, stale-result rejection,
and bounded session-owned image cleanup. Share entries/decoding but do not reuse
inline image ownership or lifetime assumptions. Keep names responsive and measure
before adding concurrency. Cache default/storage expansion and search remain driven
by concrete use; do not resume cache tuning without new evidence.

Code: `src/browser.rs` (state, input parsing, rendering), `src/browser_terminal.rs`
(terminal/signal lifetime), `src/cli.rs`, and the early browser branch in
`src/main.rs`. Existing inline code is separate. Tests: `tests/check_browser.py`.

## Local checks

Keep Cargo storage local:

```sh
export CARGO_HOME="$PWD/.cargo-home"
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --release --locked --examples --bins
python3 tests/check_browser.py
python3 tests/check_pty.py
python3 tests/check_layout.py
python3 tests/check_cache.py
```

The existing socket fixture needs local Unix-socket permission; use normal sandbox
escalation on that failure. The final browser/protocol/layout/cache suites ran in
the sandbox. Focused PTY/process diagnostics during harness debugging also used
normal escalation. No computer use or files outside this repository were used
for implementation; the executable naturally accesses its controlling terminal.

Cache measurements remain in [benchmarks/cache-working-set.json](benchmarks/cache-working-set.json)
and [benchmarks/README.md](benchmarks/README.md). Stable independent-source copies
remain ignored in `benchmarks/local/working-set/` (~119.5 MiB); do not recreate or
alter them between cache-policy comparisons. Prior ignored comparison binaries
and the four original test images are preserved. Generated fixtures already exist;
the generator refuses to overwrite them.

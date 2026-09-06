# Fresh-session handoff

2026-09-05; this chunk adds viewport-driven browser images after the user passed
the text browser. Starting Git state was clean at `1ea10e5`. Read README → ROADMAP,
then check current Git rather than assuming this snapshot is current.

## Working agreement and verification

- Work/access files only within this repo, including Cargo storage and experiments.
  Routine implementation and concise local commits are authorized.
- The user handles Ghostty visual checks. Do not automate terminal applications.
  Preserve the four original ignored images and existing working-set fixtures.
- The latest user report was “All works well. Please proceed.” after the text
  browser commands/checklist at `1ea10e5`. Record that command-set pass; exact
  version/geometry/transport were not separately resupplied. Earlier context was
  Ghostty 1.3.1, 122×40 / 8×17, arm64 macOS 26.6.2. Prior inline/layout/cache passes
  remain recorded in [compatibility](docs/compatibility.md).
- **New browser graphics have no user visual pass yet.** Protocol/PTY checks are
  separate from actual Ghostty rendering, resize, cleanup, and inline retention.
- Preserve one sorted mixed listing, complete filename access, cheap ordinary
  output, Kitty first, explicit browsing, bounded work/storage/output, and separate
  browser/inline lifetimes. No file mutations, external launches, or clipboard work.

## This chunk

- `--browse` uses a mixed grid when Kitty is enabled, a directory has candidates,
  and the terminal fits 12×10. Names/placeholders render before any worker request.
  Default/explicit protocol detection is shared with inline output. `--no-images`,
  `--protocol=none`, unknown/multiplexed terminals, and text-only directories retain
  the text browser. Grid navigation still follows entry order with arrows/j/k;
  pages use viewport capacity, and h/Left and Enter/Right keep directory actions.
- Reuse inline geometry's aspect-preserving 320×240-or-smaller thumbnail dimensions:
  five image rows, one clipped label row, gap. Resize recalculates geometry. Full
  escaped name/path inspection remains on Space, with images released during it.
- One lazy decoder thread, one outstanding request/completion, and a bounded mailbox
  with a nonblocking Unix-stream wakeup. Selected candidate first, then viewport,
  then one entry on each side. Retain at most 34 records, place at most 32 images.
- Generation changes on viewport/directory/refresh/geometry invalidate old jobs.
  Worker checks before/after load; main loop checks again before retaining pixels.
  Overlapping ready records survive scrolling; their placements move without
  retransmission. Preview source/image/cache loading is off the input thread.
- No join on quit. An in-progress decode may finish while cleanup runs, but no more
  jobs are dispatched; process exit ends any remaining worker. No child process or
  daemon. In-flight filesystem/codec calls are not cancellable and may delay newer
  previews. Directory enumeration and terminal writes can still delay main input.
- Browser-specific Kitty writer uses randomized nonzero image numbers (`I`), one
  placement (`p=1`) each, quiet replies, targeted `d=N` deletion, and no global
  deletion. Session guard owns cleanup on exit/errors/suspend. Text-row erase (EL)
  preserves placements; full-screen erase happens after browser images are released.
  Inline anonymous images/rows are unchanged. Protocol choices were checked against
  the official Kitty specification; no terminal UI automation was used.
- The same **whole-session** preview caps apply: 64 dispatched attempts by default,
  maximum 256, and 8 MiB of image commands. Hits, failures, stale work, and prefetch
  count as attempts. Upload/move bytes count; deletion bytes are reserved at upload.
  Limits do not reset on navigation, refresh, resize, detail view, or suspend.
  `[limit]` retains all names. Revisit this policy only from actual browsing feedback.
- Cache remains opt-in with the same file format, keying, storage bound, and source
  checks. A worker owns its cache. `--cache-stats` reports completed worker counters
  after restoration, so an abandoned in-flight operation may not be included.
  Text/zero-attempt paths never start a worker or open storage. No new dependency.
- Managed signals are blocked before thread creation using `pthread_sigmask`; the
  worker inherits that mask. Main `pselect` waits for input/completions/signals and
  gives input priority. Worker uses a condition variable; neither polls when idle.

## Evidence

- 39 unit + 8 CLI tests; fmt, clippy, release binaries/examples build pass.
- 25 text-browser scenarios still pass (`--no-images`), plus 21 new image-browser
  scenarios: mixed labels/failures, viewport-only placement, selected-first work,
  overlap movement, pixel/label association, scroll/navigation/refresh, resize from
  tiny to 1000×1000, caps, zero/no-image paths, cache equality, scoped cleanup on
  quit/signals/suspend, and zero redraws once settled.
- Controlled loader tests verify one outstanding request, stale pixels rejected,
  overlapping retention, and prompt worker drop even with a blocked decode.
- Existing 16 inline protocol + 34 layout + 23 cache scenarios pass. Cache replacement
  and ordinary inline output algorithms are unchanged.
- [Saved measurements](benchmarks/browser.json), methods in [benchmarks/README.md](benchmarks/README.md):
  nine fresh trials after two warmups, OS cache warm, 122×40 / 8×17, drained PTY.
  Four user images: names 3.33 ms, first preview 4.94 ms, all previews 78.86 ms
  uncached; warm cache all previews 12.31 ms. CPU 68.68 → 2.49 ms; RSS 31.39 →
  2.31 MiB. These are application/transport measurements, without a renderer.
- Input trial links the unchanged 2924×1932 JPEG plus a marker. At 10 ms after names,
  before any preview completes, G selection arrives in median 0.035 ms; direct quit
  emits restoration bytes in 0.034 ms. CPU reaches ~12–13 ms, showing active decode.
  Exit/termios checked separately; no hard latency guarantee.
- Measured binary hash matches the final release, 1,364,656 bytes. Rust/Cargo 1.98.0;
  Linux, MSRV 1.88, other terminals/transports remain unverified. Random image-number
  widths change command lengths slightly; cached pixel payloads match exactly.

## Resume here

Get the user's [browser image/cleanup pass](docs/compatibility.md#browser-image-checklist).
Check progressive images and mixed selection, paging/navigation, no stale pictures,
resize/text fallback, Space inspection, q/Ctrl-C/Ctrl-Z/fg, optional warm cache,
and prior inline-image retention after entering/quitting the browser. Fix reported
issues before claiming Ghostty image-browser compatibility.

Then tune interaction and session budgets from daily use, and consider search for
a concrete need. Do not add parallel decoders, a default cache path, more storage,
or broad format/terminal support without evidence. Packaging/platform checks remain
later roadmap items. Detailed design/limits: [docs/browser.md](docs/browser.md).

Code: `src/browser.rs` (mixed grid/state/input), `src/browser_previews.rs` (worker,
viewport generation/retention), `src/browser_graphics.rs` (owned Kitty placements
and byte budget), `src/browser_terminal.rs` (signal/event/cleanup lifetime).
`src/preview.rs` and the inline writer are unchanged; `src/cache.rs` only makes its
Stats snapshot copyable. Tests: `tests/check_browser_images.py` and existing suites.

## Local checks

```sh
export CARGO_HOME="$PWD/.cargo-home"
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --release --locked --examples --bins
python3 tests/check_browser.py
python3 tests/check_browser_images.py
python3 tests/check_pty.py
python3 tests/check_layout.py
python3 tests/check_cache.py
python3 benchmarks/browser.py
```

The full Rust suite needs local Unix-socket permission for the existing fixture;
use normal sandbox escalation. PTY suites and the browser benchmark ran in the
sandbox. The harness drains while polling exit; on macOS it reads terminal modes
from the master after controlling-session exit, and excludes transient PENDIN
retype state during suspend comparison.

Browser benchmark writes ignored `benchmarks/local/browser.json` and its own
cache/JPEG-link fixture. It never overwrites the committed report. Avoid concurrent
heavy work when comparing small timings. Cache working-set sources/binaries remain
unchanged under `benchmarks/local/working-set/` and `target/`; do not recreate those
source identities for policy comparisons. The four original images and generated
fixtures are untouched. No new cache-policy benchmark was needed in this chunk.

# Fresh-session handoff

2026-09-05. Starting Git state was clean at `59f5cec` (main, ahead of origin).
Read README → ROADMAP, then inspect current Git; this snapshot is not Git authority.

## User direction — overrides the old browser roadmap

The user rejected the image browser's presentation and scope. The supplied Ghostty
screenshot showed a mixed grid with weak selection, disconnected placeholders,
and unwanted directory navigation on Left/Right. They wanted scrolling for image
listings, not a competing terminal file browser. When asked about direction, they
explicitly selected **“Replace browsing with a simple image pager.”**

The implementation now exposes `--page [DIRECTORY]`. Do not restore directory
navigation, history, refresh, search, or a rich-TUI plan without a new request.
The earlier text-browser pass at `1ea10e5` is historical. The image browser did
not receive a visual pass; the replacement pager's visual review is still pending.
The latest screenshot did not resupply version, geometry, or transport. Earlier
context was Ghostty 1.3.1, 122×40 / 8×17, macOS 26.6.2 arm64.

## Working agreement

- Access/work only within this repo, including Cargo storage, tests, and benchmarks.
  Routine implementation and concise local commits are authorized. No push requested.
- The user handles Ghostty visual checks. Do not automate terminal apps. Preserve
  the four original ignored images and existing source identities in benchmarks.
- One ordered mixed listing, complete filename access, cheap ordinary text,
  inline default, explicit paging, Kitty first, bounded work/storage/output, and
  separate inline/pager cleanup. No file-manager actions or new dependencies.

## Current implementation

- Renamed the active modules to `pager.rs`, `pager_previews.rs`,
  `pager_graphics.rs`, and `pager_terminal.rs`; CLI/main use `--page`. `--browse`
  exits 2 with a migration message. One directory operand, same sorting/hidden/
  cache flags, foreground TTY validation and incompatible inline flags as before.
- Pager reads membership/order once. Remove navigation, history, refresh, raw-name
  restoration, and directory revisions. Initial directory symlink operands still
  resolve once; entry links remain links. File contents load on demand, so this
  is not a frozen-content snapshot. Rerun to enumerate filesystem changes.
- Arrows/hjkl move spatially. Left/Right do not wrap rows; Up/Down move one grid
  row, clamping a partial last row to its nearest entry. Text uses one column.
  Space/PgDn advances a whole viewport; b/PgUp goes back, preserving selection's
  relative position when possible. g/G or Home/End jump first/last.
- Enter/Tab opens full escaped name/kind/path inspection for every entry, including
  directories and dangling links. Esc/Backspace/Enter closes it. q/Ctrl-D quits.
  Backspace/Esc outside inspection do not navigate. Ctrl-C/Z restoration remains.
- Selected filename fills its tile width. Centered placeholders, full-width
  selected-name status, and shorter pager footer. Thumbnail geometry stays five
  image rows + filename + gap. These modest presentation changes are not a visual
  polish claim. User verification is still required.
- Existing one lazy worker, one outstanding job/completion, generation rejection,
  overlap movement, up to 32 placed images / 34 retained records remain. Cache
  keys/format/policy and source decoder are unchanged; no new concurrency.
- Attempt/output allowances renew only on viewport/geometry changes or re-entry
  after inspection/suspend. Default 64 attempts (max 256) minus retained records;
  failed records and prefetch count. An old in-flight job may complete and be
  discarded before the new batch. Same-view selection/completion redraws cannot
  renew limits. Output gets 8 MiB minus resident cleanup reservations; releases
  remain possible after exhaustion. Long sessions intentionally exceed cumulative
  invocation caps; inline caps are still whole-invocation.
- Fixed a discovered output-limit rendering issue: the final successful upload
  can leave too little budget for another. Redraw placeholders once at that
  threshold so unattempted entries show `[limit]`, not permanent `[loading]`.
- Terminal ownership remains targeted Kitty image numbers with cleanup before
  screen restore/suspend. Text-row erasure preserves images. Main pselect handles
  input/signals/completions; worker blocks when idle. Quit never joins a decoder.
  Directory enumeration and terminal writes can still block; no hard I/O timeout.

## Verification and evidence

- Rust: 39 unit + 8 CLI tests; fmt/clippy and release binaries/examples build.
- 25 text-pager PTY scenarios: sorting/mixed entries, fixed membership after edits,
  folders/links inspected without entering, full names, keys, resize, input safety,
  signals/restoration, suspend/resume, pipes, idle.
- 23 image-pager scenarios: spatial keys and full-page movement, labels/pixel
  association, viewport residency, overlap moves, stale work, mixed failures,
  preview attempts, bytes, cache equivalence, cleanup/signals/suspend/idle.
  100 independent images all preview with default attempts across pages, exceeding
  8 MiB cumulative commands. An oversized 32-slot viewport still caps its own
  output, and selection cannot renew that cap. These are byte/PTY checks, no renderer.
- Existing 16 inline protocol + 34 layout + 23 cache scenarios pass; ordinary
  inline output, source decoding, and cache implementation were not changed.
- Historical `benchmarks/browser.json` remains the measured `59f5cec` report.
  The harness is now `benchmarks/pager.py`, writing ignored `local/pager.json`.
  Do not attribute historical browser timings/binary hash to the replacement.

## Next task

Get a focused Ghostty review of the pager:

```sh
./target/release/lsa --page img-test/generated/many
./target/release/lsa --page --dirs-first img-test/generated
```

Check full-page scrolling (Space/b), spatial arrows, label selection and full names
(Enter, Esc/Backspace), resize, q/Ctrl-C/Ctrl-Z/fg, and prior inline-image retention.
The complete short checklist is in [compatibility](docs/compatibility.md#pager-checklist).
Fix concrete pager issues; keep inline daily use and later packaging as the larger
product direction. No search, browser expansion, default cache, more storage, or
parallel decoder work is pending. Linux/MSRV/other terminals remain unverified.

## Local checks

```sh
export CARGO_HOME="$PWD/.cargo-home"
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --release --locked --examples --bins
python3 tests/check_pager.py
python3 tests/check_pager_images.py
python3 tests/check_pty.py
python3 tests/check_layout.py
python3 tests/check_cache.py
```

Full Rust tests need normal escalation for the existing Unix-socket fixture.
PTY tests run in the sandbox and keep scratch files under `target/`. macOS PTY
harnesses drain while polling exit, inspect terminal modes through the master after
exit, and exclude transient PENDIN during suspend comparison. Do not use computer
control as a substitute for the user's real Ghostty review.

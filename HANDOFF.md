# Fresh-session handoff

2026-09-05; implementation snapshot: `d8d7e7d` (directory grouping and selectable
metadata). This update only records context; thumbnail caching has not started.
Read README → ROADMAP first; inspect current Git state before continuing.

## Working agreement

- Work/access files only inside this repo, including Cargo storage and experiments.
- Docs are flexible suggestions. User authorized routine implementation and concise
  commits; the earlier request for confirmation before each chunk was superseded.
- Kitty first; Sixel optional later. Keep one mixed listing, cheap ordinary text,
  complete filenames/fallback, inline output by default, browsing explicit.
- User handles Ghostty visual checks. Computer Use denied terminal access; do not
  retry through another UI/command route. User images in ignored `img-test/` are
  useful fixtures; preserve the originals.

## Evidence and limits

- Rust/Cargo 1.98.0 on arm64 macOS 26.6.2; declared MSRV 1.88, Linux unverified.
- Snapshot passed 23 unit + 8 CLI tests, 16 protocol + 34 layout PTY scenarios,
  fmt, clippy, and release build. PTY checks inspect output, not rendered images.
- User visually passed inline graphics (`f079a23`) and default layouts (`364976f`)
  in Ghostty 1.3.1, 122×40 cells, 8×17 pixels/cell. Latest grouping/metadata changes
  have automated coverage only. SSH, multiplexers, resize/theme/retention remain
  unverified; see [compatibility](docs/compatibility.md).
- No cache, concurrency, browser, terminal queries, or stdin reads. Inline images
  are anonymous and retained in history; future browser cleanup needs its own lifetime.
- Preview attempts/bytes and source/decode sizes are bounded; decoder allocation
  limits are best effort, with no hard process-memory bound or decode timeout.
- [Measurements](benchmarks/README.md): 10k plain names ~6.8 ms / 3.59 MiB;
  four original images ~71 ms / 31 MiB. OS cache was warm, thumbnails decoded every
  run. PTY drain time does not measure terminal rendering.

## Resume here

Next: bounded thumbnail-cache experiment, initially opt-in. Preserve lazy text
paths; compare warm hits against uncached decoding before considering a default.
Use versioned identity/mtime/size/geometry/transform keys, atomic writes, capped
storage, corruption recovery, and disable/clear controls. Exercise invalidation,
replacement, read-only storage, concurrent writes, eviction, and existing limits.
Keep experimental storage inside the repo; cache thumbnail data, not terminal IDs.

Entry/sort/lazy metadata: `src/entry.rs`; layout choice: `src/layout.rs`;
preview pipeline: `src/preview.rs`; budgets/emission: `src/grid.rs`, `src/kitty.rs`;
orchestration: `src/main.rs`. Design rationale: [decisions](docs/decisions.md).

## Local checks

From repo root, set `export CARGO_HOME="$PWD/.cargo-home"`. Run `cargo fmt --check`,
`cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`, then
`cargo build --release --locked --examples` before the Python PTY checks:
`python3 tests/check_pty.py` and `python3 tests/check_layout.py`.
Generate fixtures with `./target/release/examples/fixtures` only if
`img-test/generated/` is absent; the generator refuses to overwrite it.
`python3 benchmarks/measure.py` writes ignored `benchmarks/local/metadata.json`.
Full tests need local Unix-socket permission; macOS RSS measurement needs kernel
timing permission. Request normal sandbox escalation if blocked. Raw invalid-byte
filename integration coverage is Linux-only; macOS retains byte-level unit tests.

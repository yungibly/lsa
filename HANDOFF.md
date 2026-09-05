# Fresh-session handoff

2026-09-05; implementation: bounded opt-in thumbnail cache, included in the same
commit as this handoff. Starting Git state was clean at `9ec490a`; cache work is
now implemented and measured. Read README → ROADMAP, then recheck Git state.

## Working agreement

- Work/access files only inside this repo, including Cargo storage and experiments.
- Docs are flexible suggestions. User authorized routine implementation and concise
  commits; no per-chunk confirmation needed.
- Kitty first; Sixel optional later. Keep one mixed listing, cheap ordinary text,
  complete filenames/fallback, inline output by default, browsing explicit.
- User handles Ghostty visual checks. Do not use computer automation to access
  terminals. Preserve the original ignored images in `img-test/`.

## Implemented in this session

- `--cache-dir=PATH` or `--cache-dir PATH` opts in; `--no-cache` wins in either
  order. `--clear-cache` clears and exits; requires a cache directory and no
  operands/diagnose/no-cache. `--cache-stats` prints counters to stderr.
- Cache initialization is lazy, after a budgeted source is opened and validated.
  Text, pipes, diagnose, non-candidates, and exhausted budgets never access it.
- 64 direct-mapped slots, one staging file, one persistent empty lock. Raw RGBA
  records have versioned full device/inode/size/mtime/ctime/pixel-size keys and
  CRC32 checksums. Collisions evict safely; no index/scans or writes on hits.
- Bound is 19,973,460 bytes of lsa-written file contents in one namespace,
  including maximum staging, plus filesystem overhead. Directory is private.
  Nonblocking shared/exclusive flock serializes bounded cache I/O only; busy or
  broken storage falls back to decoding. Atomic rename, no fsync promise.
- Source fd metadata is rechecked after hits and before insertion. Cached
  thumbnails still count against attempt and byte budgets. No terminal IDs cached.
- Added `crc32fast` as a direct dependency; its version was already in Cargo.lock.
  No new dependency download. Details and limitations: [cache design](docs/cache.md).

## Verification and measurements

- Rust/Cargo 1.98.0, arm64 macOS 26.6.2; declared MSRV 1.88, Linux unverified.
- 30 unit + 8 CLI tests; 16 original PTY + 34 layout + 23 cache scenarios pass.
  Release build, fmt, clippy pass. Cache tests cover invalidation, restored mtime,
  replacement/links, geometry, corruption, read-only storage, concurrent processes,
  eviction/full byte cap, stale staging, lock contention, safe clear, and limits.
- [Saved cache measurements](benchmarks/cache.json): four originals at 122×40 /
  8×17 take 71.76 ms uncached, 71.82 ms empty, 4.60 ms warm (~15.6× faster);
  application RSS 31.36 → 2.03 MiB. Four records occupy 239,696 bytes. PTY output
  is byte-identical in these modes. Invalidated copies take 72.34 ms.
- 40 links to one cheap synthetic source take 27.16 ms disabled, 17.17 ms empty
  (one miss, 39 same-invocation hits), 17.43 ms warm. Plain 10,000 names remain
  ~7.2–7.3 ms in a before/after comparison, even with cache configured but unused.
- OS cache was warm; no visible terminal rendering or terminal memory measured.
  Cache remains **off by default**. Original images were not modified.
- User visually passed inline graphics (`f079a23`) and layouts (`364976f`) in
  Ghostty 1.3.1, 122×40 / 8×17. Metadata/grouping and this cache have automated
  coverage only. [Cache visual commands](docs/compatibility.md) are ready for the
  user. SSH, multiplexers, resize/theme/retention still need terminal trials.

## Resume here

Evaluate hit rate/latency on independent-source working sets near/above 64 slots,
and alternating geometry, before changing replacement or default policy. The
four-image case shows clear benefit but cannot establish broader hit rates.
Gather the user's Ghostty empty/warm/disabled comparison and daily-use feedback.
Keep experiment data inside the repo and cache opt-in. No concurrency/browser
expansion without new evidence; no default storage path has been chosen.

Code: `src/cache.rs` owns records/storage/locking/tests; `src/preview.rs` owns
source checks/decoding; `src/grid.rs` still owns preview budgets/emission.
CLI/orchestration: `src/cli.rs`, `src/main.rs`. Existing layout/entry behavior is
unchanged. Inline lifetime stays anonymous/history-retained; a future browser
needs separate image ownership and cleanup.

## Local checks

Set `export CARGO_HOME="$PWD/.cargo-home"`. Run `cargo fmt --check`,
`cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`, and
`cargo build --release --locked --examples --bins`, then:

```sh
python3 tests/check_pty.py
python3 tests/check_layout.py
python3 tests/check_cache.py
python3 benchmarks/cache.py --before target/lsa-before-cache
```

The before binary is a saved ignored copy of the metadata build; omit `--before`
if absent. Cache measurements write `benchmarks/local/cache.json`; committed
reports are not overwritten. `benchmarks/measure.py` remains the broader baseline
and writes `benchmarks/local/metadata.json`. Generated fixtures already exist;
the generator refuses to overwrite them.

Full tests need local Unix-socket permission; macOS RSS measurement needs kernel
timing permission. Both needed normal sandbox escalation this session. PTY checks
ran within the sandbox. Invalid-byte filename integration remains Linux-only;
macOS uses byte-level display tests. In-process decoder allocations are best
effort; filesystem/decode/terminal time and total process memory are not sandboxed.

# Fresh-session handoff

2026-09-05; this commit measures larger cache working sets and improves replacement.
Starting Git state was clean at `7050349`. Read README → ROADMAP, then check Git.

## Working agreement and user verification

- Work/access files only inside this repo, including Cargo storage and experiments.
- Routine implementation and concise local commits are authorized; no per-chunk
  confirmation. Docs are working suggestions, not rigid requirements.
- User handles Ghostty visual checks; do not use computer automation for terminals.
  Preserve the four original ignored images under `img-test/`.
- The user reported all supplied commands worked exactly as expected at `7050349`:
  metadata/grouping and empty/warm/disabled cache commands now have a user pass.
  Earlier reports established Ghostty 1.3.1, 122×40 / 8×17, arm64 macOS 26.6.2.
  The latest report did not resupply version/geometry/transport; do not invent new
  SSH/multiplexer/resize/theme evidence. See [compatibility](docs/compatibility.md).
- Keep one ordered mixed listing, cheap text, complete filenames/fallback, Kitty
  first, inline default, browsing explicit. No file mutations/file-manager expansion.

## This chunk

- Measured 4/16/32/48/64/96 independent source identities with fixed and alternating
  thumbnail geometry; originals were copied, never altered. Stable copies live in
  ignored `benchmarks/local/working-set/`. They occupy about 119.5 MiB and are reused
  across binaries because changing identities changes collision distribution.
- Direct mapping missed excessively below capacity. Tested four and eight candidate
  slots per key, retaining the same 64 total slots / 19,973,460-byte file-content cap.
- Selected eight slots per group. Lookup probes at most eight bounded headers with
  one candidate fd open at a time. Full keys/checksums still validate hits.
- Insertion rechecks candidates under the existing exclusive lock: replace an
  existing key, else a missing/unreadable/malformed-header slot, else a victim
  selected by a per-invocation randomly seeded standard-library hash of the key.
  Trace models showed FIFO thrashing on repeated scans above group capacity.
  No hit-time writes, index, directory scan, new dependency, or background work.
- Record format, v1 namespace, fixed filenames, lock, and atomic staging remain
  unchanged. Old records in new candidate slots are reusable; other old placements
  miss and repopulate. Old/new binaries share the same storage bound; no migration.
- Source validation, decoder limits, output budgets, layout, CLI, and inline image
  lifetime are unchanged. Cache is still opt-in via `--cache-dir`; no default path.

## Evidence

[Saved comparison](benchmarks/cache-working-set.json) and [methods](benchmarks/README.md):

- 32 repeated sources: 56.25% hits / 216.42 ms → 100% / 15.97 ms (~13.6×).
- 16 alternating-size sources: 62.50% / 98.74 ms → 100% / 9.01 ms.
- 48 repeated sources: 50% / 456.10 ms → 96.67% / 63.55 ms.
- Four warm sources stay ~4.76 ms. 96 sources alternating two sizes exceed capacity
  (192 keys) and gain little. Eight candidates are not fastest for every overloaded
  case; random replacement and unequal codec costs affect individual runs.
- Output digests match for all 18 cases across all three policies; no cache errors.
  All 96 previews fit attempt/output caps. OS cache warm, fresh processes, drained
  PTY, no renderer. Five timed repeated/off runs, ten alternating runs, two warmups.
- Separate RSS: 32 repeated sources drop 35.97 → 2.09 MiB; four warm stay 2.03 MiB.
  Cases decoding misses remain roughly 31–39 MiB; no hard process-memory bound.
  Four-slot pilot has no RSS sample. The final release binary is 1,279,600 bytes.
- 32 unit + 8 CLI tests; 16 original PTY + 34 layout + 23 cache scenarios pass.
  Release build, fmt, clippy pass. New tests cover colliding-key coexistence,
  full-group eviction, duplicate prevention, hits behind malformed candidates, and
  prior direct-mapped records. Storage-policy changes have automated coverage;
  the user visual pass belongs to the prior `7050349` command set.

## Resume here

Start an explicit browser as a usable text-first slice: `--browse`, alternate
screen, mixed-entry selection, scrolling, directory navigation, quit, resize,
and Ctrl-C/terminal restoration. Block while idle. Preserve sort/filter behavior
and complete filename access. Validate with PTY input/output tests, then the user's
Ghostty check. Viewport-driven images/jobs and session-only cleanup follow in a
separate slice. Shared entries/decoding are useful; inline image IDs/lifetimes must
not be reused as browser ownership. Keep cache opt-in until a concrete need changes
its default/storage policy; avoid further cache tuning without new evidence.

Code: `src/entry.rs`, `src/layout.rs`, `src/cli.rs`, `src/main.rs` for listing and
orchestration; `src/cache.rs` for storage; `src/preview.rs` for source/decode checks;
`src/grid.rs`, `src/kitty.rs` for inline output. Cache details: [docs/cache.md](docs/cache.md).

## Local checks

Set `export CARGO_HOME="$PWD/.cargo-home"` and run:

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --release --locked --examples --bins
python3 tests/check_pty.py
python3 tests/check_layout.py
python3 tests/check_cache.py
python3 benchmarks/cache_working_set.py --label current
python3 benchmarks/cache_working_set.py --label current --memory-only
```

Memory-only augments an existing timing report on macOS and verifies its binary
hash. Reports stay in `benchmarks/local/working-set-LABEL.json`; the committed
summary is not overwritten. Saved ignored binaries: `target/lsa-before-cache-associativity`
(the `7050349` binary), `target/lsa-cache-four-way` (pilot), and `target/lsa-before-cache`
(the earlier metadata binary). Use `--binary` and a distinct `--label` to compare.
Do not recreate the working-set sources between comparisons.

`benchmarks/cache.py` remains the original cache benchmark; `benchmarks/measure.py`
remains the broader listing baseline. Generated image fixtures already exist; the
generator refuses to overwrite them. Full tests need local Unix-socket permission;
macOS memory measurement needs kernel timing permission. Both use normal sandbox
escalation. PTY checks ran in the sandbox. Linux/MSRV 1.88 remain unverified;
Rust/Cargo 1.98.0 is the local tested toolchain.

# Session handoff

Recheck Git state before relying on this snapshot. Keep this file short: current
state and next step. History belongs in the [changelog](CHANGELOG.md), decisions
in [docs/decisions.md](docs/decisions.md).

## 2026-11-06 — v0.6.0 compatibility pass

The user requested practical drop-in `ls` behavior, including symlink controls,
richer metadata and recursion, with PRs merged when confident and a new release
published afterward. [PR #2](https://github.com/yungibly/lsa/pull/2) merged at
`5823fe1`; [v0.6.0](https://github.com/yungibly/lsa/releases/tag/v0.6.0) tags that
commit and is published. The Homebrew tap is on 0.6.0.

Implemented: format precedence, columns in pipes, sorted operands, distinct
`-a`/`-A`, compact inode/allocation prefixes, `-H`/`-L`/`-P`, iterative `-R` with
cycle detection and shared image budgets, fuller explicit long output, bounded
xattr/ACL/flags/context details, and common option aliases. Automatic compact
details and plain output retain their fast paths.

Local validation: formatting/Clippy, 110 Rust tests, 257 headless scenarios,
9 Ghostty VT text checks, release-tool tests and extracted-package smoke tests
pass. Before/after measurements are in `benchmarks/drop-in.json`; ordinary
listing/gallery costs are essentially unchanged, explicit `-l` adds metadata
query work. The prior binary is `target/lsa-before-v060`.

Hosted validation: all five PR CI jobs and all eleven
[release jobs](https://github.com/yungibly/lsa/actions/runs/42305020841) passed,
including Homebrew installs on Apple Silicon, Intel Mac and Linux. One ARM Mac
gallery-memory sample failed; investigation and an unchanged retry passed.
Details are in [compatibility](docs/compatibility.md). The public Apple Silicon
archive passed checksum and CLI smoke checks, and its hash matches the tap.

**Next:** a real Ghostty visual pass is optional user follow-up and remains
unverified; no computer use. See the roadmap for cases to check. The practical
compatibility implementation, merge and release are complete.

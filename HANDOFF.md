# Session handoff

Recheck Git state before relying on this snapshot. Keep this file short: current
state and next step. History belongs in the [changelog](CHANGELOG.md), decisions
in [docs/decisions.md](docs/decisions.md).

## 2026-11-06 — v0.6.0 compatibility pass

The user requested practical drop-in `ls` behavior, including symlink controls,
richer metadata and recursion, with PRs merged when confident and a new release
published afterward. Branch: `codex/ls-drop-in`.

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

**Next:** open/merge the implementation PR after hosted CI, then push `v0.6.0`
and follow the release/Homebrew workflow through completion. A real Ghostty
visual pass is optional user follow-up and remains unverified; no computer use.

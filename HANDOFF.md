# Session handoff

Recheck Git state before relying on this snapshot. Keep this file short: current
state and next step. History belongs in the [changelog](CHANGELOG.md), decisions
in [docs/decisions.md](docs/decisions.md).

## 2026-11-06 — v0.7.0 terminal-independent icons

The user completed the v0.6.0 checks successfully, but reported missing Nerd
Font icons in Rex and requested a new minor release. Exact terminal version,
font, geometry and transport were not supplied. v0.6.0 is already released.

Branch: `codex/terminal-independent-icons`. Nerd Font icons now default on for
normal TTY output regardless of terminal brand, multiplexer or SSH. The new
`--icon-style=nerd|portable` selects glyphs independently of `--icons` visibility.
Pipes and dumb terminals stay plain by default. No graphics or decoder changes.

Local validation: formatting/Clippy, 111 Rust tests, 115 headless layout/style
checks and release-tool tests pass. The terminal matrix simulates environment
variables and does not verify Rex rendering. Details are in
[compatibility](docs/compatibility.md).

**Next:** merge the PR after hosted CI, then tag v0.7.0 and follow publication,
Homebrew tests and the tap update through completion. Rex icon appearance is
user follow-up; no computer use.

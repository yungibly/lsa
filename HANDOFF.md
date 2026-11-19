# Session handoff

Recheck Git state before relying on this snapshot. Keep this file short: current
state and next step. History belongs in the [changelog](CHANGELOG.md), decisions
in [docs/decisions.md](docs/decisions.md).

## 2026-11-18 — v0.7.1 Rex graphics detection

The user found that `--grid` showed no grid in Rex. `--diagnose` reported
`terminal=rex`, `kitty=false`, `reason=unknown terminal`: graphics detection was
an allow-list of Ghostty/Kitty names. The v0.6.0 ls-parity changes were not the
cause, and terminal detection is unchanged since v0.5.0. With `--protocol=kitty`,
the user reported that rendering and scrollback worked in Rex 0.1.0 (preview,
macOS), and asked for the simple fix and a release. The user agreed not to add
capability queries, because of latency and the no-input-handling rule.

`TERM_PROGRAM=rex` / `TERM=xterm-rex` now enable Kitty graphics. Nested
tmux/screen/zellij, SSH and `TERM=dumb` still use text. Help, README, usage,
decisions and compatibility are updated. A `tests/cli.rs` case no longer
inherits the shell's `LS_COLORS`/`NO_COLOR`.

Local validation (x86-64 Linux, inside Rex): formatting/Clippy, 109 Rust tests,
30 PTY checks (9 new Rex cases), cache/thumbnail/gallery/recursive checks and
release-tool tests pass. `check_layout.py` (115) passes with umask 022 but has a
pre-existing umask-002 false failure; see [compatibility](docs/compatibility.md).

**Next:** user confirms automatic Rex previews and Nerd Font icons on v0.7.1.

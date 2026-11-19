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

[PR #6](https://github.com/yungibly/lsa/pull/6) merged after all five CI jobs
passed. [v0.7.1](https://github.com/yungibly/lsa/releases/tag/v0.7.1) tags merge
commit `7215b06`, whose tree matches the tested PR head. All eleven
[release jobs](https://github.com/yungibly/lsa/actions/runs/44437842850) passed
on the first attempt, including Homebrew installs and the tap update. The tap
formula's hashes match `SHA256SUMS`. The public x86-64 Linux archive enabled
graphics automatically in the agent's own Linux Rex session (bytes only).

After upgrading, the user confirmed that `lsa img-test` and `lsa --grid`
preview in Rex without `--protocol=kitty`, and that both the default Nerd Font
icons and `--icon-style=portable` symbols render. This closes the Rex follow-ups.

**Next:** see the roadmap's detailed terminal evidence list. No computer use.

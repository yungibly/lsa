# Session handoff

Recheck Git state before relying on this snapshot. Keep this file short: the
current state and next step. History belongs in the [changelog](CHANGELOG.md),
decisions in [docs/decisions.md](docs/decisions.md).

## 2026-09-28 — v0.5.0

The user asked for the full set of recommendations from a whole-program review,
committed at a sensible cadence and released. The work landed as separate
commits:

- JPEG memory growth fix.
- Name-only entries with classification computed once.
- Appearance fixes: default foreground for plain files, dangling links,
  directory sizes, LS_COLORS, hyperlink paths, SSH graphics policy, `--grid`
  notices.
- `ls` flags, with column-major `-C` and a new `-x`.
- zlib-compressed payloads and reused artwork.
- Parallel decoding.
- macOS ImageIO previews.
- An optional Ghostty VT check.
- Documentation restructure and release.

Evidence for each commit is in its message and in
[benchmarks/README.md](benchmarks/README.md).

Tools under the ignored `target/` directory:

- `target/tools/tui-test/tui-test`: the microsoft/tui-test 0.1.0-beta.5 release
  binary (checksum-verified), suggested by the user. Run it with
  `HOME=target/tools/tui-test/home`. `tests/check_ghostty_vt.py` uses it.
- `target/lsa-before-v050`: the v0.4.0 release binary, used for comparisons.

Unix-socket CLI fixtures ran without sandbox escalation this session.

**Next:** the user's Ghostty pass (see [roadmap](ROADMAP.md)), then daily use.

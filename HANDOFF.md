# Session handoff

2026-09-05. Recheck Git state before relying on this snapshot. The user authorized
commits, a broad product reset, and removal of prior designs without reconfirmation.
No push, release publication, personal shell edits or installation was requested.

## Direction

Make lsa a fast, familiar ls alias with useful eza-like appearance and intelligent
images. The judgment call is to remove paging entirely: always print, leave output
in scrollback, and return to the shell. One mixed entry ordering, complete names,
quiet preview failures, conservative Kitty detection and bounded work remain.

Work/access stays inside this repository. Cargo storage, fixtures, checkpoints,
measurements and packages are local. The user handles Ghostty visual verification;
do not use computer control. Preserve original img-test images and benchmark sources.

## This change

The starting tree already contained uncommitted automatic-pager/UI/packaging work.
Its patch, untracked files and release binary were saved under
`target/before-inline-overhaul/` before editing. The final source replaces that pager
work, retains host packaging, and rewrites the docs around the new product.

Added a shared style layer (colors, icons, validated LS_COLORS, optional OSC 8),
allocation-free natural name comparison, familiar flags and directory-link operands,
readable metadata and bounded formatting caches. Consecutive file operands share
one layout. Galleries use the sequential inline writer with a 16-attempt default
and an 8 MiB shared command budget, then compact names. No new runtime dependency.

Removed pager modules, input/event/worker handling, residency management, pager
harnesses and active pager docs. Old benchmark reports are retained as history;
`benchmarks/inline.py` measures the current application and supports paired binaries.

## Validation and next work

36 unit + 12 CLI tests, strict clippy, formatting, release build, 16 protocol + 48
inline layout/style + 23 cache PTY scenarios and extracted host archive smoke checks
pass.
See README for commands, compatibility for counts/limitations and benchmarks for
paired measured results. Unix socket fixture tests require normal sandbox escalation.

The next task is concrete Ghostty feedback on the new defaults, then suitable Linux
and Rust 1.88 validation. The previous terminal context was Ghostty 1.3.1, arm64 macOS
26.6.2, 122×40 cells and 8×17 pixels; that is historical inline evidence, not a pass
for new styling or gallery tails. Use docs/compatibility.md's focused commands.

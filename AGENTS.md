# Working on lsa

Read [README.md](README.md) for product direction, then [ROADMAP.md](ROADMAP.md) for current status, acceptance criteria, and the next development task.
See [HANDOFF.md](HANDOFF.md) for recent session context; recheck Git state before relying on its snapshot.

Please only work and access files within this directory. The img-test directory (gitignored) has a few test images you can use. If you ever want more, feel free to ask me.

Ghostty is the current target/test terminal. I can do visual verification for you; handle everything you can do without computer use.

## Preserve these decisions

- Images, ordinary files, folders, and symlinks form one listing with shared ordering and interaction. A preview is an entry representation; do not split the default experience into image and non-image sections.
- This is an everyday human-oriented `ls` replacement. Keep ordinary text listings fast and shell behavior sensible; exact GNU/BSD scripting parity is not required.
- Inline output is the default. Interactive browsing is explicit; automatic paging is opt-in.
- Kitty graphics is the initial target; Sixel can come later if useful. Record actual terminal compatibility rather than inferring it from protocol support.
- Bound preview work, allocations, queues, cache storage, and output. Browser previews follow the viewport. Preview failures never hide files.
- Filenames and metadata remain terminal text. The fallback is a complete usable text listing.

## Development practice

- Read the current roadmap and inspect the workspace before assuming implementation or validation status. Docs are working suggestions, not fixed requirements.
- Implementation language, libraries, numeric performance targets, and exact CLI details are provisional. Make routine implementation decisions from prototype evidence and record the reasons.
- Keep inline output and browser output lifetimes distinct, including progressive rendering and image cleanup.
- Verify graphics in real terminals. Report terminal/version, protocol, mode, and transport conditions; upstream library support is not lsa validation.
- Use meaningful tests and measurements for the change at hand. Separate application cost from terminal cost and cache-warm from cache-cold results.
- Update the roadmap's status and next task after implementation work. Keep intended behavior, implemented behavior, and verified behavior distinguishable.
- Avoid expanding into a general file manager, background service, or plugin system without a concrete need. Prioritize the complete mixed-directory experience and resource efficiency.

# lsa roadmap

Last updated: 2026-09-05.

**Current state:** Planning documents only. No application, build configuration, benchmarks, terminal validation, or Git repository has been created. Every milestone below is pending. This is a sequence of deliverables and decision gates, not a calendar estimate.

Read [README.md](README.md) for product intent. The essential requirement is a single directory listing containing images, ordinary files, folders, and symlinks under the same ordering and interaction rules.

## Milestone 0 — Prove both graphics backends

**Purpose:** Resolve the highest-risk rendering assumptions before building around a particular library or terminal behavior.

Tasks:

- [ ] Inspect the actual development environment and available test terminals; record versions.
- [ ] Evaluate a small Rust prototype and existing rendering libraries. Compare dependencies, startup overhead, inline suitability, and viewport rendering; select a stack from evidence.
- [ ] Render the same small mixed-directory fixture through Kitty and Sixel. Include folders, text files, a symlink, transparent PNG, JPEG, and GIF stills, with labels rendered as terminal text.
- [ ] Exercise several rows and dozens of simultaneous thumbnail placements. A single-image demo is not sufficient.
- [ ] Test repeated inline invocations, the bottom terminal row, natural scrolling, scrollback, resize, and return to the prompt.
- [ ] Test a minimal alternate-screen viewport with scroll/page changes and cleanup on quit and Ctrl-C.
- [ ] Test explicit backend selection, unsupported detection, bounded query timeouts, and plain redirected output.
- [ ] Record baseline timing, application memory, terminal resource use, and payload size.
- [ ] Record the library/decoder decision and any native runtime dependencies.

Initial test candidates are Kitty and Ghostty for Kitty graphics, and a Sixel-enabled xterm or foot for Sixel. These are test candidates, not claims of lsa compatibility. Use available equivalents where necessary and document the substitution.

**Exit criteria:** Static mixed-entry grids work through both protocols on at least one actual terminal per protocol. Names remain text, repeated output does not corrupt the prompt, and a basic interactive view can redraw and exit cleanly. A saved matrix distinguishes tested, failed, and untested combinations and includes reproduction commands.

If continuous Sixel scrolling is troublesome, test whole-page browser redraw. If a backend is still blocked, document the exact terminal/version, observed failure, attempted alternatives, and narrower feasible scope. Do not silently drop Sixel or declare text fallback to be Sixel success. A limited prototype may proceed while the limitation is investigated; release claims must reflect the evidence.

Suggested evidence files, to create only when they contain real results:

- `docs/compatibility.md`: terminal/mode matrix and reproduction steps.
- `docs/decisions.md`: selected stack and reasons.
- `benchmarks/README.md`: fixtures, method, and baseline observations.

## Milestone 1 — Build the everyday text listing

**Purpose:** Make the command useful and efficient in directories without images.

Tasks:

- [ ] Implement entry enumeration, path operands, multiple operands, and `--` handling.
- [ ] Implement hidden-file behavior, name sorting, reverse order, time/size sorting, and compact/detailed/one-per-line text output.
- [ ] Support the agreed common flags and document their semantics; reject unsupported options clearly.
- [ ] Preserve all entry types and visible symlink identity; handle broken links and permissions.
- [ ] Handle long names, Unicode display width, non-UTF-8 paths, control characters, leading dashes, and files disappearing during enumeration.
- [ ] Keep redirected output free of graphics and terminal interrogation. Define errors, exit codes, and closed-pipe behavior.
- [ ] Ensure `--no-images` skips preview work and capability negotiation even in an image directory.
- [ ] Establish a reproducible baseline against the platform's `ls` for equivalent listing work. Record any deliberate semantic differences.

**Exit criteria:** The command is a credible daily text listing tool. Tests cover path handling, sorting/filtering, display escaping, partial failures, and non-TTY behavior. An ordinary directory does not open file contents or initialize the graphics pipeline unnecessarily. Measured performance and remaining overhead are recorded.

## Milestone 2 — Deliver unified inline thumbnails

**Purpose:** Ship the defining experience without requiring an interactive browser.

Tasks:

- [ ] Build the shared preview-request and thumbnail pipeline with bounded workers, allocations, queues, and output buffering.
- [ ] Support JPEG, PNG, GIF stills, WebP, and BMP. Correct orientation, transparency, and aspect ratio handling are part of the baseline.
- [ ] Implement fixed-geometry grid tiles for every entry type under one sort order.
- [ ] Render filenames/metadata as text, including predictable long-name handling and type placeholders.
- [ ] Integrate Kitty and Sixel backends, bounded capability detection, and explicit overrides.
- [ ] Emit ordered inline rows incrementally; handle slow/failed previews without blocking indefinitely or revisiting inaccessible scrollback rows.
- [ ] Implement a configurable inline thumbnail/output budget. Every entry must still appear after preview limiting.
- [ ] Implement the versioned, bounded thumbnail cache with invalidation, atomic writes, eviction, disable/clear controls, and graceful cache failures.
- [ ] Add a simple auto-layout heuristic with explicit overrides. Keep full content decoding out of layout selection.
- [ ] Provide a diagnostic command and publish tested behavior/limitations.

**Exit criteria:** A mixed directory remains complete and ordered in text and grid layouts. Both protocols show recognizable thumbnails on their tested terminals. Corrupt images, unsupported formats, and exhausted budgets produce visible entries rather than omissions. Warm-cache runs reuse valid thumbnails. Repeated invocations preserve useful shell output and do not leak application-owned browser state into later commands.

This is a useful inline alpha. It may precede the browser, but must be described as incomplete relative to the full product roadmap.

## Milestone 3 — Browse large directories within bounded resources

**Purpose:** Make thousands of images usable without rendering all of them into terminal history.

Tasks:

- [ ] Add explicit browser mode with an alternate screen, stable selection, keyboard movement, paging, and quit.
- [ ] Reuse the complete mixed-entry collection, sort rules, and grid representation from inline mode.
- [ ] Display names and placeholders promptly; fill thumbnails asynchronously without shifting tiles or changing the selected entry.
- [ ] Schedule visible entries and a small prefetch margin, drop obsolete requests, and reject stale completions after navigation.
- [ ] Bound decoded/encoded memory and terminal image residency where the backend permits it. Account for entry-list memory separately.
- [ ] Handle resize, image clipping, overlays/status text, terminal backpressure, and rapid scrolling.
- [ ] Verify Sixel continuous scrolling; use a documented page-redraw strategy on tested combinations that require it.
- [ ] Add name search, thumbnail-size adjustment, enter/leave folder navigation, open selected file, and copy selected path.
- [ ] Restore terminal modes and remove only this browser session's graphics on quit, Ctrl-C, and recoverable failure.
- [ ] Add optional automatic paging based on estimated output height, decided before inline output starts. Keep explicit browser invocation fully usable without it.

**Exit criteria:** Thousands of eligible images can be browsed while preview memory/work remains bounded by configured budgets. Navigation and cancellation remain responsive during slow decoding. Returning to an already visited area reuses appropriate cached work. Both backends have verified browser behavior on their advertised terminals, and ordinary files are equally selectable and actionable.

There is no requirement to keep all directory entry metadata in constant memory; document the enumeration/sorting cost for very large directories rather than confusing lazy previews with constant-cost listing.

## Milestone 4 — Validate and package the first release

**Purpose:** Make the initial supported experience reproducible for other users.

Tasks:

- [ ] Review TIFF, SVG, AVIF, and HEIC/HEIF support against real fixtures, limits, packaging cost, and target users. Implement justified additions and explicitly document exclusions.
- [ ] Add optional image dimensions, format, and animation indicators without eager full decoding.
- [ ] Test local sessions, SSH, and tmux separately; publish exact verified combinations and fallbacks.
- [ ] Verify installation and runtime dependencies on selected macOS and Linux targets. Record unsupported targets explicitly.
- [ ] Verify light/dark backgrounds, narrow/wide windows, different cell sizes, long names, and icon-free operation.
- [ ] Run the benchmark and robustness matrix below; set numerical regression budgets based on observed baselines.
- [ ] Document installation, everyday examples, options, cache behavior, supported formats, known limitations, and troubleshooting.
- [ ] Publish the first release only with claims supported by recorded tests. Source-control and distribution setup can occur during implementation when needed; the planning phase does not imply they already exist.

**Exit criteria:** Installation works from the documented instructions; the text path, inline grid, and browser satisfy their acceptance criteria; both graphics protocols have a useful tested baseline; performance data and compatibility boundaries are available. Outstanding issues are recorded with clear impact.

## Follow-up candidates

Prioritize from actual use and measurements:

- Selected-image animation with bounded playback resources; never animate an entire directory by default.
- Thumbnail-enhanced detailed rows if the result remains readable.
- Additional high-value formats and preview providers.
- iTerm2 protocol where it improves tested coverage or behavior.
- Mouse navigation, file hyperlinks, and more accessible/character-art display options.
- Directory refresh/watch behavior, preserving selection and cache correctness.
- Windows support and additional terminal/multiplexer combinations.
- Optional Git information or machine-readable output only if justified without slowing the default path.

File mutation, complex plugin APIs, background services, and broad file-manager functionality remain separate scope decisions.

## Validation matrix

These are planned scenarios, not completed checks. Exact fixture sizes can change if the same stress conditions are represented. Use generated or redistributable fixtures, and keep large datasets out of the source repository.

| Scenario | What it establishes |
| --- | --- |
| Empty directory and nonexistent/inaccessible operand | Clear output, errors, and exit behavior. |
| Ordinary source tree and image directory with images disabled | No unnecessary decoding/probing; credible text speed. |
| Small mixed directory with folders, links, images, and text | One complete collection, consistent sorting, readable names. |
| 1,000–10,000 images, cold and warm thumbnail cache | Startup, first viewport, queue bounds, cache reuse, and scrolling behavior. |
| Approximately 100,000 directory entries | Enumeration/sorting overhead and metadata memory, independent of preview memory. |
| Very large JPEG/PNG, corrupt/truncated images, animated GIF, special files | Decode limits, still-frame correctness, errors, and continued responsiveness. |
| Replaced/deleted source, corrupt/read-only cache, simultaneous invocations | Invalidation, atomic cache behavior, and graceful recovery. |
| Control characters, non-UTF-8 paths, emoji/wide characters, long names, leading dashes | Safe and correct path/display handling. |
| Piped/redirected output, no controlling TTY, closed pipe | Shell behavior without leaked graphics or query sequences. |
| Kitty and Sixel: repeated inline output, bottom row, resize, scrollback | Output lifetime and terminal-specific limits. |
| Both backends: fast browser scroll, directory change, quit, Ctrl-C | Stale-job rejection, selection stability, resource cleanup, shell restoration. |
| Local, SSH, and tmux combinations | Actual transport/multiplexer compatibility, explicitly distinguished. |

Use automated tests for core data/CLI behavior and meaningful image/limit cases. Use real-terminal checks or appropriate terminal integration tests for graphics, scrolling, and cleanup. A protocol snapshot or a headless test does not prove that visible images render correctly.

Benchmark reports should separate application and terminal resource use, cold/warm thumbnail caches, filesystem-cache conditions, first useful output, complete viewport, and total work. Do not claim GUI superiority based only on application process time or output redirected to a sink.

## Next development session

Start with Milestone 0. Inspect available terminals and build a small mixed-entry rendering experiment through both Kitty and Sixel. Save concrete compatibility observations before selecting the final application stack or claiming support. No implementation task has been started or completed yet.

When development advances, update milestone checkboxes and this final section with completed work, evidence paths, remaining blockers, and the next bounded task. Keep implementation status distinct from planned behavior.

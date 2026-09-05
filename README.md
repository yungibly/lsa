# lsa

A fast, everyday directory listing command with image thumbnails and an optional scrollable browser.

**Status:** Idea and planning phase, 2026-09-05. There is no implementation yet. `lsa` is the working name. Commands below describe proposed behavior, not available functionality. No implementation language or dependency stack has been finalized.

This document records product intent and architectural direction for future development sessions. See [ROADMAP.md](ROADMAP.md) for milestones, acceptance criteria, and the next task.

## Product intent

Make inspecting a directory in the terminal as visually useful as inspecting it in Finder or a file explorer, while preserving the speed and convenience of an everyday `ls` command.

**Images, ordinary files, folders, and symlinks belong to one directory listing.** A thumbnail is one way to represent an entry. It does not make the entry part of a separate image collection. Sorting, filtering, selection, and navigation operate on the same entries regardless of preview support.

The user specifically wants to avoid the split between an image viewer and a normal file listing. Do not introduce separate image and non-image sections by default. Image-only filtering can be an explicit option later.

The primary use is human inspection. Exact GNU/BSD `ls` output and scripting parity are not requirements. Familiar everyday behavior and basic shell correctness are requirements.

## Priorities and boundaries

In order of importance:

1. Useful, unified listings of every kind of directory entry.
2. Very fast startup and low resource use, especially when images are absent or disabled.
3. Recognizable thumbnails with readable, selectable filenames.
4. Reliable Kitty graphics and Sixel support on an explicitly tested set of terminals.
5. Large directories remain responsive without decoding or retaining every image.
6. Predictable defaults and straightforward overrides.
7. Broader formats and convenience features as the foundations prove themselves.

“Faster and lighter than GUI explorers” is an aspiration to measure against concrete workflows. It is not an established benchmark result. GUI explorers may already have warm thumbnail caches, and terminal rendering itself consumes resources.

The first release focuses on listing, recognition, and lightweight browsing. File mutation, batch renaming, synchronization, indexing services, and a plugin platform are outside the initial scope. Browsing into a folder is in scope; becoming a full file-management suite is not necessary to validate this product.

## User experience

### One collection, several layouts

The same filtered and sorted entry collection feeds every layout:

| Layout | Intended use | Representation |
| --- | --- | --- |
| Compact text | Source trees, ordinary directories, narrow terminals | Familiar dense names and optional small type markers. |
| Detailed list | Explicit long listing or metadata inspection | Aligned metadata and names; thumbnails are an optional extension to this layout. |
| Thumbnail grid | Image-heavy or explicitly selected directories | Images get previews; folders and other entries get a clear type representation in the same ordered grid. |

In a grid, consistent tile geometry keeps sorting and navigation understandable. Ordinary files still occupy real positions in that grid. Recover density by choosing a compact layout for text-heavy directories, rather than moving non-images into a separate section.

Use ordinary terminal text for filenames and metadata. Do not rasterize names into the thumbnail. No special icon font should be required; provide simple text or standard-character type markers. Preserve aspect ratio by default, fit the full image, and keep transparency visible against both light and dark backgrounds.

Long names need intentional truncation or wrapping. Preserve useful suffixes where practical, and provide a way to inspect/copy the complete name in browser mode. A tiny or unsupported preview must not make the entry disappear.

### Inline output and browser output have different lifetimes

**Inline output** writes a listing into the shell's normal history and returns to the prompt. It is the default. It should remain useful after the process exits.

**Browser output** owns an alternate terminal screen until the user exits. It supports scrolling, selection, search, and thumbnails that load as entries become visible. It restores the shell on exit, including after an interrupt or recoverable error.

Share the entry model, sorting, thumbnail pipeline, and layout rules between these modes. Do not assume they can share identical drawing and cleanup behavior.

For inline output, prefer ordered, incremental row emission. Once output has scrolled away, do not depend on revisiting old rows to finish previews. Asynchronous placeholder replacement belongs primarily in browser mode. Slow previews may produce a placeholder under a documented budget; names must still be listed.

For browser output, paint names/type placeholders first, then fill reserved thumbnail areas without moving entries. Render the visible viewport and a small prefetch margin. Preserve selection by entry identity when layout changes.

### Automatic behavior

Auto-layout should consider the proportion of likely images, a minimum image count, terminal width, and the expected size of the resulting layout. Initial image classification can use filename extensions; decoding every file just to choose a layout is unacceptable.

Examples:

- Eight images among ten entries is a reasonable grid candidate.
- Fifty images among ten thousand source files is a reasonable compact-list candidate.
- Explicit long-list or layout flags take precedence over the heuristic.

Choose an inline layout once. In browser mode, resize may recompute geometry, but completed or failed thumbnail jobs must not flip the layout between list and grid.

Entering a browser is independent of choosing a layout. Do not take over the screen by default just because the directory is large. Make automatic paging opt-in. When enabled, base it on estimated output height, not only file count, and choose before emitting the listing.

Default inline output should have a thumbnail/output budget so a huge directory does not flood the terminal with image payloads. Continue listing every entry after that budget is exhausted, and clearly indicate preview limiting. Never silently truncate the directory listing. An explicit browser invocation provides access to all eligible previews as the user scrolls.

### Proposed command surface

Flag spelling is provisional. Keep the surface small and avoid maintaining redundant names without a reason.

```text
lsa                         # Automatic inline layout
lsa path/to/directory       # Inspect another directory
lsa -la                     # Familiar detailed listing, including hidden entries
lsa --grid                  # Force a unified thumbnail grid
lsa --no-images             # Skip all image decoding and graphics negotiation
lsa --browse                # Interactive, scrollable directory browser
lsa --pager=auto             # Opt into browser mode when output would be large
lsa --protocol=kitty        # Override backend detection
lsa --protocol=sixel        # Override backend detection
lsa --protocol=none         # Disable graphics; retain a usable listing
lsa --diagnose              # Report capabilities, selected backend, and limitations
```

The conceptual settings are layout, image enablement, paging, and graphics backend. They should not be conflated. CLI options override user configuration; configuration overrides defaults. Invalid or conflicting options should produce understandable errors.

The browser's initial actions should be movement, page movement, search by name, adjust thumbnail size, enter/leave a directory, open a file, copy its path, and quit. Keyboard operation must be complete before optional mouse support. Opening files and writing to the clipboard occur only after an explicit user action.

## Everyday listing contract

- List all entry types, even when they have no preview: directories, regular files, symlinks, broken links, and special files.
- Preserve familiar hidden-file behavior and prioritize common options such as `-a`, `-A`, `-l`, `-h`, `-1`, `-t`, `-S`, and `-r`. Specify exactly which options are supported; reject unknown flags instead of ignoring them.
- Support explicit paths, multiple operands, and `--` for names beginning with a dash.
- Use one deterministic sort policy across layouts. Default to name order initially. Optional directories-first sorting must be explicit and consistent.
- Keep filenames intact internally, including non-UTF-8 names on Unix. Escape control characters for display so names cannot issue terminal commands. Align Unicode text by display width.
- Do not follow directory symlinks recursively by default. A preview may resolve a symlink to a regular image while preserving its link identity in the listing.
- Never feed FIFOs, sockets, or device files into an image decoder.
- When stdout is redirected or piped, default to plain text without graphics, terminal capability queries, interactive paging, color, or hyperlinks. The human-oriented output need not become a new machine-readable format.
- Write diagnostics to stderr, use meaningful exit codes, and handle a closed pipe normally. A thumbnail failure should not turn an otherwise successful directory listing into a fatal error.
- Show partial results and clear errors when some entries or operands are inaccessible. Directory contents may change while being listed.

Detailed listing is text-first for the initial release. Thumbnail rows with detailed metadata can follow once they have a layout that remains readable and efficient.

## Kitty and Sixel strategy

**Decision: plan for both protocols from the first graphics prototype and target both for the first usable thumbnail release.** Neither should be treated as a speculative distant extension. This refines the earlier suggestion to start with only one protocol.

Supporting both is realistic. The risk is consistent behavior across terminals and modes, especially when scrolling, resizing, clipping, or clearing images. A successful single-image demo is insufficient evidence.

[Kitty's protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/) provides image transmission, placements, deletion, and Unicode placeholders. These facilities offer useful building blocks for redraw and text-associated images; actual support still needs testing in the chosen terminals and multiplexers.

Sixel requires a different rendering approach. Palette encoding, redraw cost, background handling, and terminal scrolling/erase behavior need explicit attention. [libsixel](https://github.com/libsixel/libsixel) provides an encoder/decoder implementation to evaluate rather than assuming a custom encoder is necessary.

[ratatui-image](https://github.com/ratatui/ratatui-image) already supports both protocols and publishes a compatibility matrix, demonstrating a viable reuse path. Its documentation distinguishes Sixel drawing behavior from Kitty's retained image placements. Its tests and compatibility claims are upstream evidence, not proof that lsa works in those environments.

Practical policy:

- Evaluate existing rendering libraries before writing protocol implementations.
- Keep protocol capabilities behind a backend boundary. Do not pretend every backend supports reusable placements, partial erasure, or animation identically.
- Support static thumbnails in both. Continuous scrolling, animation, and scrollback retention may have different limits.
- If continuous Sixel scrolling is unreliable on a terminal, evaluate page-at-a-time redraw for the browser. Document any such behavior and test cleanup; do not label a text fallback as successful Sixel support.
- Use bounded terminal queries only when graphics are relevant. Avoid per-file negotiation and repeated detection delays. Separate query replies from user keystrokes and leave the terminal clean.
- Prefer a tested backend for a detected terminal/version combination when multiple protocols are available. Let users override detection. Do not switch backends halfway through a listing.
- Fall back to a complete text listing if capability detection fails. Character-art previews may be an explicit future option, not an expensive default fallback.
- Treat SSH and tmux as separate test dimensions. Do not assume a remote process shares the terminal's filesystem or that every multiplexer forwards graphics identically.
- In browser mode, remove this session's obsolete images and restore terminal state. In inline mode, preserve emitted thumbnails where supported; do not globally clear other programs' images on exit.
- Verify scrollback and resize behavior separately. Protocol support does not imply indefinite image retention in terminal history.

iTerm2's own image protocol is a later candidate if it materially improves coverage or reliability at reasonable cost. Its [official documentation](https://iterm2.com/documentation-images.html) also describes `imgls` and animated GIF support. Do not promise universal terminal coverage merely because two protocols are implemented.

The first prototype must produce an evidence-based support matrix listing terminal/version, OS, protocol, inline behavior, browser behavior, and SSH/tmux conditions. All lsa support is currently **unverified**.

## Image support

Listing support and thumbnail support are separate: every entry is listed whether or not its contents can be previewed.

| Scope | Formats/behavior | Notes |
| --- | --- | --- |
| Initial static thumbnail target | JPEG, PNG, GIF, WebP, BMP | Required baseline; GIF support initially means a correctly composited still frame. |
| First-release expansion candidates | TIFF, SVG | High value, subject to decoder limits and packaging evidence. Start with one page for TIFF; SVG rendering should not fetch external resources. |
| Early follow-up candidates | AVIF, HEIC/HEIF | Valuable for modern web and phone images. Investigate dependency, distribution, and decode-cost tradeoffs early. |
| Explicit later scope | RAW, PDF, video, document previews | Separate providers if justified; avoid making heavyweight external tools mandatory for ordinary listings. |
| Animation | Selected GIF/WebP playback in the browser | Opt-in and limited to the selection; not required for either protocol's static-thumbnail baseline. |

Treat “all common formats” as a direction with a published support table, not an unbounded promise. Revisit the initial boundary after testing real user directories and decoder packaging.

Correctness includes orientation metadata, preserved aspect ratio, transparency, corrupt inputs, very large dimensions, and bounded handling of multi-frame/multipage images. Dimensions, format, and animation indicators are useful optional metadata, but obtaining them must not force eager decoding of every entry.

## Performance and architecture

### Shared pipeline, separate presentation modes

```text
arguments/configuration
    -> directory enumeration + requested metadata
    -> filter/sort + cheap preview-candidate classification
    -> layout + eligible preview requests
    -> bounded decode/resize/cache/encode work
    -> inline row writer OR interactive viewport renderer
    -> selected terminal backend
```

Keep filesystem logic and sorting independent of the image renderer. Keep decoding independent of Kitty/Sixel encoding. Use a stable entry identity and a directory generation to prevent completed jobs from painting the wrong file after navigation or refresh.

Rust is the leading implementation candidate because this is a performance-sensitive CLI/TUI, but that is a provisional choice. Evaluate a small library combination in the compatibility prototype. For Rust, Ratatui/Crossterm and ratatui-image are candidates for the browser; inline output needs its own lifecycle review. Inspect current crate features and transitive native dependencies before committing to packaging assumptions.

### Work only on what is needed

- Keep the text path free of image decoding, heavyweight image initialization, and unnecessary capability queries.
- Request metadata only when presentation or sorting requires it. Avoid eager content sniffing, recursive sizes, Git status, and background indexing.
- Enumerating and sorting names still has directory-size cost. Viewport-based decoding does not make total memory or startup independent of entry count. Measure entry metadata and thumbnail memory separately.
- Use reduced-resolution decoding or embedded previews where suitable, then resize to the required thumbnail dimensions before terminal transmission. [libvips thumbnailing](https://www.libvips.org/API/8.17/ctor.Image.thumbnail.html) is one candidate that exploits shrink-on-load where the decoder supports it.
- Bound workers, queued requests, decoded bytes, encoded output, and cache storage. A thread limit alone is insufficient when several huge images decode at once.
- Prioritize visible entries, then a small prefetch margin. Drop queued work after navigation and ignore stale completions.
- Do not promise that every running native decode can be interrupted. Use decoder allocation limits, bounded concurrency, and subprocess isolation only where necessary to enforce hard limits.
- Throttle rendering and apply output backpressure when the terminal cannot keep up. Avoid re-encoding unchanged images or redrawing the whole viewport for every completed thumbnail.
- Static browsing should not need a continuous busy redraw loop. Idle CPU is a metric.

[Yazi's performance notes](https://yazi-rs.github.io/blog/why-is-yazi-fast/) describe paged preloading and thumbnail caches. They are useful architectural references; lsa still needs its own measurements for a grid of simultaneous previews.

### Cache deliberately

Use a bounded, versioned thumbnail cache in the platform's user-cache location. A cache key should include file identity/path policy, file size, sufficiently precise modification time, target size, and rendering/decoder version as appropriate. Avoid hashing entire source files during routine listing. Metadata-based invalidation is a performance tradeoff and needs documented limitations.

Provide a way to disable and clear the cache. Handle read-only cache locations, concurrent invocations, interrupted writes, eviction, and file replacement. A corrupt cache entry should trigger regeneration. Failed previews may be cached briefly, but must not remain broken permanently after the file changes.

Keep decoded thumbnail reuse separate from protocol-specific/session-specific payloads. Evicting an application cache does not necessarily release images already retained by the terminal. Inline output budgets and browser backend cleanup must address that separately.

### Measure the actual experience

Track time to first useful names, first thumbnail, complete visible viewport, total inline completion, scroll responsiveness, application peak memory, terminal CPU/memory, idle CPU, bytes sent, and cache size.

Measure cold and warm thumbnail caches separately, and distinguish those from the operating system's file cache. Use the same directory, thumbnail dimensions, terminal size, and relevant metadata when comparing alternatives. Benchmarks redirected to a sink test listing/pipeline cost, not visible rendering latency.

Include ordinary source directories, small mixed directories, thousands of images, a very large entry count, large compressed images, long/Unicode names, and failures. Record hardware, filesystem, OS, terminal/version, protocol, and build configuration. Set numeric budgets after the initial baseline; do not invent performance claims before measurement.

## Decisions and open questions

### Settled product direction

- Unified entries and ordering in every layout; no default separation of images and other files.
- An everyday `ls` replacement, with human usage prioritized over exact scripting compatibility.
- Inline output by default; browser mode explicit and automatic paging opt-in.
- Kitty and Sixel are initial targets; test both early.
- Text fallback always remains useful.
- Static thumbnails by default, bounded work, and lazy browser previews.
- Filenames remain terminal text, and preview failures never hide entries.

### Provisional implementation choices

- Rust is the leading candidate; libraries, decoder selection, and cache format are open.
- macOS and Linux are proposed initial OS targets; Windows follows demonstrated demand and a feasible compatibility path.
- Auto-layout thresholds, thumbnail sizes, and inline preview budgets need prototypes and measurements.
- The exact first-release format boundary beyond JPEG/PNG/GIF/WebP/BMP is open.
- Precise browser keybindings, navigation details, and copying behavior need implementation design.
- Terminal/multiplexer support must be earned through testing rather than inferred from protocol names.

Change provisional decisions when evidence warrants it. Record the reason and update the roadmap in the same development session. Preserve settled product direction unless the user changes it or concrete findings require a clearly documented revision.

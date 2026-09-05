# lsa

A fast, human-oriented directory listing with inline image thumbnails. An explicit
browser can follow. Kitty graphics first; Sixel is an optional future addition.

**Status:** Implementation starting. Ghostty is the first visual test target;
terminal compatibility is not yet verified. See [ROADMAP.md](ROADMAP.md).

## Direction

- One sorted collection of images, files, folders, links, and special files.
  A preview changes an entry's representation, never its membership or ordering.
- Inline output returns to the shell and remains useful in history. Browsing is
  explicit; automatic paging, if added, is opt-in.
- Names and metadata are terminal text. Unsupported images and preview failures
  retain a readable entry. Piped output is complete plain text.
- Make the ordinary text path cheap: no content reads, graphics queries, eager
  metadata, cache initialization, or background work unless needed.
- Bound thumbnail input, dimensions, allocations, work, and transmitted bytes.
  Listing every name is more important than previewing every image.
- Preserve aspect ratio, orientation, and transparency. No required icon font.

The docs are working design notes, not fixed requirements. Prefer a measured,
useful slice over speculative interfaces. Exact BSD/GNU `ls` parity, file mutation,
indexing services, plugins, animation, and broad preview providers are outside the
initial scope.

## First slice

Build a small Rust CLI with ordinary text listings and an explicit mixed-entry
Kitty grid. Use direct image transmission, so no terminal-side filesystem access
is required. Keep decoding separate from output; defer a backend framework until
another backend exists.

Start with static PNG, JPEG, GIF, WebP, and BMP through selected `image` crate
features. Reject excessive input before full decoding; process one preview at a
time. Add workers, caching, and reduced-resolution decoding only where measured
latency justifies them. Decoder limits are not a process memory or time sandbox.

Use conservative environment detection and terminal dimensions without reading
stdin. Explicit overrides should be available; non-TTY output always stays text.
Multiplexers need their own validation. Names sort by raw Unix bytes, are retained
losslessly internally, and are escaped for safe display.

## Later slices

1. Validate inline geometry, bottom-row output, repeated invocations, scrollback,
   resize, and return to the prompt in Ghostty. Record exact conditions.
2. Improve density and latency from actual directories: compact text columns,
   auto-layout, thumbnail cache, and bounded parallel work as needed.
3. Add an alternate-screen browser with viewport-driven previews, stable
   selection, stale-job rejection, and session-only image cleanup. Share entries
   and decoding with inline output, not output lifetime assumptions.
4. Package tested macOS/Linux targets; expand formats and terminals from demand.

See [AGENTS.md](AGENTS.md) for development practice.

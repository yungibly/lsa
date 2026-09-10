# Application measurements

## Listing efficiency and long defaults — 2026-09-10

[Equal-output report](performance.json) and [image/cache controls](performance-images.json).
Before: v0.2.0 source at `02762c1`, saved as `target/lsa-before-performance`.
After: v0.3.0 release binary; both reports record exact binary SHA-256 values.
Arm64 macOS 26.6.2, Rust 1.98.0, thin-LTO/stripped. Two warmups followed by seven
paired/interleaved fresh processes, OS caches warm and not flushed, no concurrent
builds/tests. PTY output is drained at 122×40 cells / 8×17 pixels without a renderer;
stdin is `/dev/null`. Child CPU and peak RSS come from `wait4`.

| 10,000 entries, equal output | Elapsed before → after | Peak RSS before → after |
| --- | ---: | ---: |
| Plain names, pipe | 8.26 → 7.69 ms | 3.95 → 3.95 MiB |
| Styled compact columns, PTY | 29.68 → 27.66 ms | 4.17 → 4.17 MiB |
| Long, repeated timestamp, pipe | 27.52 → 24.27 ms | 5.83 → 5.03 MiB |
| Long, distinct timestamps, pipe | 995.72 → 530.71 ms | 5.86 → 5.06 MiB |
| Numeric fields without time, pipe | 26.08 → 23.90 ms | 5.56 → 4.53 MiB |
| Long, repeated timestamp, PTY | 44.68 → 43.22 ms | 5.89 → 5.09 MiB |
| Long, distinct timestamps, PTY | 975.44 → 510.10 ms | 5.91 → 5.09 MiB |

The distinct fixture spaces times by 64 seconds, exceeding/defeating the bounded
exact-second cache. It demonstrates the removed second `localtime_r` conversion:
roughly **47–48% less elapsed/CPU time** on this host. This is not a universal speedup;
libc/timezone costs differ by platform and repeated times already hit the cache.
Repeated-time pipe CPU falls **13%** (25.64 → 22.35 ms); plain/compact elapsed gains
are about **7%**. Complete output hashes match in every equal-work case.

Long-listing peak RSS falls about **14%**, or **19%** without the time column.
Each retained metadata record now contains 48 bytes of used stat fields on the
supported 64-bit targets. A modified column retains another 24 bytes of civil-time
state per entry so output never repeats the conversion. This still uses less memory
than the old full stat records; no per-entry formatted timestamp strings are retained.
Safe filenames borrow their original storage, and permission characters no longer
allocate individually. Plain output retains no per-entry metadata; grids do not
collect long details just because long is now the terminal fallback.

The **changed default is a different workload**: styled output grows from 300,000
to 1,750,000 bytes, elapsed from 28.29 to 43.92 ms, and RSS from 4.16 to 5.08 MiB in
the repeated-time fixture. Explicit `-C` restores compact columns; the equal-work
PTY rows compare old `-l` with the new default. Automatic sparse-image long listings
also perform miniature decoding just as explicit `-l` does; `--no-images` avoids it.

Image controls are essentially unchanged: four original images take 73.06 → 72.51 ms
with cache off, 73.20 → 73.04 ms with an empty cache, and 5.58 → 5.42 ms with a warm
cache. The 40-image gallery takes 17.66 → 17.67 ms. Long miniatures, images, artwork and all output bytes agree across versions
and off/empty/warm cache modes. Image RSS
and graphics traffic are essentially unchanged. Cache setup is outside timing.
The stripped binary grows by only 80 bytes (2,288,960 → 2,289,040), with no
new dependencies. These measurements establish application/transport costs, not Ghostty rendering
speed, image memory, or a new real-terminal visual pass.

```sh
python3 benchmarks/performance.py --before target/lsa-before-performance --runs=7
python3 benchmarks/inline.py --before target/lsa-before-performance --runs=7
```

The first harness fixes timestamps and alternates version order per pair. The
second retains the existing image/cache controls. Reports first go under ignored
`benchmarks/local/`; checked final reports are copied explicitly.


## Final artwork pass for v0.2.0 — 2026-09-07

[Paired report](artwork.json), same arm64 macOS/122×40 drained-PTY conditions as
below. Seven paired/interleaved runs after two warmups; no thumbnail cache and no
concurrent builds/tests. Before is the user's Ghostty-verified gallery build saved
as `target/lsa-before-final-efficiency`; after is the local v0.2.0 release binary.
The report records exact hashes. Every case asserts identical complete output.

| 256 entries | Elapsed before → after | CPU before → after |
| --- | ---: | ---: |
| Folders with grid artwork | 58.29 → 52.24 ms | 37.05 → 31.28 ms |
| All eleven artwork categories | 60.85 → 51.85 ms | 39.73 → 31.15 ms |
| Raster image control | 84.21 → 83.67 ms | 61.84 → 61.64 ms |

Integer rectangles now paint directly, and polygons scan only their vertex bounds
instead of all 8,000 canvas pixels. Folder CPU falls about **16%** and mixed artwork
CPU about **22%**, with identical pixels/bytes and essentially unchanged peak memory.
The complete offline artwork sheet is also byte-identical. Source image decoding
is unaffected. No additional retained cache, queue or worker was introduced.

```sh
python3 benchmarks/artwork.py --before target/lsa-before-final-efficiency --runs=7
```

Earlier measurements below describe the gallery build before this final pass.

## Larger galleries, variable size and SVG/ICO — 2026-09-07

[Everyday cases](gallery-options.json) and [equal-work/EXIF cases](gallery-efficiency.json),
arm64 macOS 26.6.2, Rust 1.98.0, release/thin-LTO/stripped. Before: checkout
`e966c9e`, saved as `target/lsa-before-gallery-options`. Both reports identify the
exact before/after binaries by SHA-256. Two warmups, seven paired/interleaved fresh
processes per case; OS caches warm, not flushed. Output goes to a drained 122×40
PTY with 8×17 cell pixels, no renderer, and stdin `/dev/null`. `wait4` records child
CPU/RSS. Cache clearing/priming and output parsing are outside elapsed timing.
The helper preserves the queued PTY tail after process exit.

| Median elapsed time | Before | After |
| --- | ---: | ---: |
| 10,000 plain names, pipe | 7.98 ms | 8.14 ms |
| 10,000 styled names, terminal | 26.94 ms | 27.08 ms |
| 10,000 long entries, pipe | 24.90 ms | 24.98 ms |
| Four original images, cache off | 71.02 ms | 70.94 ms |
| Four original images, empty cache | 71.56 ms | 70.90 ms |
| Four original images, warm cache | 5.28 ms | 5.43 ms |
| 40-image gallery, defaults | 10.27 ms (16 previews + 4 artwork) | 16.93 ms (40 previews) |
| 256-image gallery, explicit equal limits | 84.43 ms | 85.35 ms |
| 6.4 MP rotated JPEG, cache off | 20.82 ms | 15.60 ms |
| Same JPEG, empty cache | 20.91 ms | 15.53 ms |
| Same JPEG, warm cache | 4.56 ms | 4.57 ms |
| Same JPEG, long miniature, cache off | 20.75 ms | 15.41 ms |

The 256-image case uses **distinct copies** of one synthetic 320×160 PNG, cache
off, with `--preview-limit=256` on both versions. Both print 256 source previews
and byte-identical output: 7,834,880 graphics bytes. Its speed and roughly 2.9 MiB
RSS are essentially unchanged. The default 40-image case instead does more work:
graphics traffic doubles to 1,224,200 bytes. It links one small source and does not
model 256 large photographs. More previews increase terminal traffic and residency;
these measurements do not establish Ghostty rendering time or image memory.

Bounded buffered reads reduce the original four-image case from **31.45 to 29.97
MiB** peak RSS, about 1.5 MiB, without changing output bytes. Whole-file base64
allocation is replaced with fixed 4 KiB scratch space. These changes do not produce
a broad timing gain in the sampled PNGs or ordinary text.

Moving EXIF rotation after thumbnailing gives the clearest gain: the generated
3,200×2,000 JPEG with orientation 6 takes **25% less elapsed time**, **30% less CPU**,
and **47% less peak RSS** (39.58 → 21.12 MiB) with caching off. Long miniatures show
similar savings. Fractional resize-edge pixels can differ slightly when rotating
after sampling; all eight orientations have content/coverage tests, and cache v2
prevents stale transform reuse. Off, empty and warm caches remain byte-identical
within each binary. This sample is one synthetic JPEG, not a universal decoder gain.

SVG/ICO support adds about 0.90 MiB to the stripped binary: **1,345,936 → 2,289,008
bytes**. resvg 0.48.1 has text/system-font/raster-image/SVGZ features disabled.
The SVG path renders at thumbnail resolution with bounded input complexity; it
does not allocate a native-resolution image. CPU/allocation bounds remain best
effort for third-party decoders/renderers; no hard wall-clock timeout was added.

```sh
cargo build --release --locked --examples --bins
./target/release/examples/gallery_fixtures  # once; refuses to overwrite
python3 benchmarks/inline.py --before target/lsa-before-gallery-options --runs=7
python3 benchmarks/gallery.py --before target/lsa-before-gallery-options --runs=7
```

Run measurements after builds/tests finish to avoid local contention. Reports go
under `benchmarks/local/`; copying checked reports into the repository is explicit.

## Smaller grids and long-view miniatures — 2026-09-06

[Paired report](thumbnail-ux.json), arm64 macOS 26.6.2, Rust 1.98.0,
release/thin-LTO/stripped. The before binary is commit `4a5269b`; SHA-256 identities
and sizes are in the report. Two warmups, then seven paired/interleaved fresh
processes per case. OS caches are warm, not flushed. Output goes to a drained
122×40 PTY with 8×17 cell pixels and no renderer; stdin is `/dev/null`.

| Median elapsed time | Before | After |
| --- | ---: | ---: |
| 10,000 plain filenames, pipe | 9.64 ms | 9.71 ms |
| 10,000 filenames, default terminal output | 27.46 ms | 29.99 ms |
| 10,000 entries, default long listing, pipe | 30.21 ms | 30.25 ms |
| Four user images, cache off | 80.44 ms | 78.98 ms |
| Four user images, empty thumbnail cache | 82.02 ms | 79.26 ms |
| Four user images, warm thumbnail cache | 4.58 ms | 3.59 ms |
| 40-image gallery, default output | 15.54 ms | 9.85 ms |
| Four user images, long view, cache off | 4.15 ms (text) | 78.26 ms (miniatures) |
| Four user images, long view, empty cache | 3.64 ms (text) | 79.71 ms (miniatures) |
| Four user images, long view, warm cache | 3.36 ms (text) | 3.64 ms (miniatures) |
| Release binary size | 1,329,312 bytes | 1,345,936 bytes |

The mixed grid sends **52.2% fewer graphics bytes**, from 320,340 to 153,025, even
with a fifth placement for folder artwork. Frames shrink from 176×85 to 112×51
pixels. The gallery now emits 16 source previews and four built-in images to finish
its last grid row; all 40 names remain present. Its graphics traffic falls from
1,281,360 to 612,100 bytes. This fixture links one small synthetic source, so its
decode time does not represent 16 large photographs. `images` in the report counts
all placements, including artwork; `image_bytes` counts complete Kitty APC bytes.

Long view sends four 24×17 miniatures in 8,896 graphics bytes. Their small output
does not remove source decoding cost: the uncached four-image sample takes about
78 ms and 31.6 MiB peak RSS, versus 3.6 ms and 2.2 MiB with a warm thumbnail cache.
Use `--no-images` for text-only long listings. Cache remains opt-in; clearing and
priming occur outside timed runs. Off, empty and warm caches produce identical
output bytes within each binary and layout.

Plain filename and long metadata pipe output is byte-identical across versions,
with essentially unchanged measured cost. Default styled text adds a neutral
foreground for otherwise unclassified files: 300,000 versus 210,000 output bytes
and about 2.5 ms more in this 10,000-entry fixture. No blanket speedup is claimed.

The harness records child CPU/RSS with `wait4`. It measures application and PTY
transport cost, not visible rendering or terminal image memory.

```sh
python3 benchmarks/inline.py --before target/lsa-before-tile-polish --runs=7
```

The updated harness includes long-view image cases and graphics byte counts.
Reports are written to `benchmarks/local/inline.json`; committed evidence is copied
explicitly after checking that the release binary hash matches.


## Previous inline product reset — 2026-09-05

[Paired report](inline.json), arm64 macOS 26.6.2, Rust 1.98.0, release/thin-LTO/stripped.
The before binary is the uncommitted automatic-pager implementation saved at the
start of this change; both binaries are identified by SHA-256 in the report.
Two warmups, then 11 paired/interleaved fresh processes per case. OS caches warm,
not flushed. Terminal output goes to a drained 122×40 PTY with 8×17 cell pixels;
stdin is `/dev/null`, so the former automatic pager cannot capture input.

| Median elapsed time | Before | At that change |
| --- | ---: | ---: |
| 10,000 plain filenames, pipe | 8.01 ms | 9.13 ms |
| 10,000 filenames, default terminal output | 10.50 ms | 26.40 ms |
| 10,000 entries, default long listing, pipe | 1,020.02 ms | 29.61 ms |
| Four user images, cache off | 76.55 ms | 76.39 ms |
| Four user images, empty thumbnail cache | 78.01 ms | 77.77 ms |
| Four user images, warm thumbnail cache | 3.98 ms | 3.99 ms |
| 40-image gallery, default output | 2.31 ms (text only) | 12.22 ms (16 previews + all names) |
| Release binary size | 1,381,072 bytes | 1,329,312 bytes |

Natural sorting adds roughly 1 ms in the plain fixture; byte output is identical
for these zero-padded names. Automatic terminal styling adds mode reads to identify
executables, icon formatting and more output. Its measured 10,000-entry peak RSS is
about 3.97 MiB, the same as before; mode reads retain only a boolean per entry.
The default has useful decoration but is not faster than the former plain columns.

The long-listing fixture contains many identical modification timestamps. A bounded
64-slot exact-second formatting cache avoids repeated expensive `localtime_r` calls
across the width/output passes on this host. This is a large fixture-specific gain,
not a universal long-listing speedup; distinct times or cache collisions still need
conversion. Default long columns also changed to readable owner/size details.
The local timezone environment was inherited. Historical long measurements were
much faster than this before sample; compare paired results within a session.

Four original user sources and 176×85 thumbnail canvases are unchanged. Cache off,
empty and warm produce identical bytes within each binary; the new binary adds
styled labels. Cache clear/priming occur outside measured runs. Uncached image RSS
is about 31.44 MiB; warm-cache RSS about 2.09 MiB. The gallery fixture uses 40 links
to one small synthetic source, so its decode times do not describe 16 large photos.

The harness drains output and records child CPU/RSS with `wait4`. It does **not**
measure visible rendering, terminal image memory or time until the user sees pixels.
No kernel/OS-cache flush, terminal renderer or external timing executable is used.

```sh
python3 benchmarks/inline.py
python3 benchmarks/inline.py --before target/before-inline-overhaul/lsa
```

Reports go to `benchmarks/local/inline.json`; saving committed evidence is explicit.
Fixtures/cache stay under `benchmarks/local/inline/`. The separately built host
archive is smoke-tested after extraction; the report describes `target/release/lsa`.

## Historical evidence

[Historical reports](history/) retain earlier measurements, not the current product.
Their output policies, columns, harnesses and sampling conditions differ. Reproduce
older implementations using their corresponding Git revision and harness.

| Topic | Report |
| --- | --- |
| Initial inline cost | [baseline](history/baseline.json) |
| First automatic grids and columns | [defaults](history/defaults.json) |
| Metadata alignment | [metadata](history/metadata.json) |
| Opt-in cache | [cache](history/cache.json) |
| Cache working sets | [working sets](history/cache-working-set.json) |
| Removed interactive implementations | [browser](history/browser.json), [automatic pager](history/pager-auto.json) |

The cache working-set report compares stable independent source identities and
64 slots with eight candidate locations per key. Its 32-source repeated case reached
100% hits versus 56.25% for direct mapping; 96 alternating sources exceed capacity.
`cache_working_set.py --label NAME` remains a focused current-build experiment;
it explicitly raises the attempt cap and uses undecorated output. Preserve its
ignored source files to retain source identities across measurements. It relies on
the four original user images; its historical-binary option requires the matching
version of the PTY helper. The cache itself is unchanged by the product reset.

# Application measurements

## Browser previews: 2026-09-05

Historical results for `59f5cec`, before the user replaced the browser with a
one-listing pager. These numbers are preserved, not attributed to the new binary.

[Saved results](browser.json), same macOS arm64 host and Rust 1.98.0 release build.
Browser geometry is 122×40 at 8×17 pixels/cell (176×85 thumbnail canvases). The normal
fixture is the unchanged four user images plus `generated/`. Nine fresh measured
processes follow two warmups for each case; OS cache is warm, not flushed. Thumbnail
storage is off, empty before each process, or primed separately as shown.

| Four-image browser | Names ready | First preview | All previews | Child CPU | Peak RSS |
| --- | ---: | ---: | ---: | ---: | ---: |
| Cache off | 3.33 ms | 4.94 ms | 78.86 ms | 68.68 ms | 31.39 MiB |
| Cache empty | 2.82 ms | 4.63 ms | 78.54 ms | 68.82 ms | 31.41 MiB |
| Cache warm | 2.68 ms | 4.15 ms | 12.31 ms | 2.49 ms | 2.31 MiB |

Times are medians measured at a drained controlling PTY, **without a renderer**.
Names readiness means the first complete text frame has arrived; previews mean
complete Kitty transmission bytes, not visible images. The first candidate is a
small GIF; its early result does not represent the expensive JPEG's decode time.
All four images fit this viewport. Image payload equivalence between empty/warm
storage is tested separately, with completed cache counts of 4 misses versus 4 hits
in these full-view samples. Randomized image-number lengths make command byte
counts vary slightly (~324.8 KB). A text-browser control reports 4.28 ms names,
2.79 ms child CPU, and 1.86 MiB RSS; do not infer a text regression from the ordering
of these small samples and differing harnesses.

A separate input fixture links the unchanged 2924×1932 user JPEG and adds a plain
marker. Input is sent 10 ms after names appear, before any preview completes.
Selection (`G`) arrives in a median **0.035 ms**, and a direct quit emits restoration
bytes in **0.034 ms**. Child CPU reaches about 12–13 ms in these trials, consistent
with active decoding before process exit. Tests with a deliberately blocked loader
independently prove that stale results are discarded and worker drop does not join.
The timing trial is a responsiveness sample, not a hard decode/transport deadline.

Clock starts before PTY setup and process creation. Incremental command parsing
avoids repeatedly scanning the full image stream; read timestamps still include
Python drain/scheduling overhead. The quit endpoint is observed cleanup/alternate-
screen exit bytes; process exit and restored termios are checked separately. Child
user+system CPU and peak RSS come from `wait4`, include the worker, and exclude the
Python reader. Wall time also includes PTY transport/wakeups and is not comparable
to terminal rendering cost. No cold-filesystem, terminal-memory, or GUI comparison.

The current harness is `python3 benchmarks/pager.py`, adapted to `--page`. It
writes only `benchmarks/local/pager.json` and its own ignored cache/fixture. Use
the earlier commit to reproduce browser behavior. The historical committed report
remains unchanged and records the measured release SHA-256 and 1,364,656-byte
binary. Browser tests also verify no redraws after work settles; the text control
idle trial consumed ~2.6 ms child CPU including startup/quit during a 1-second wait.

## Cache working sets: 2026-09-05 follow-up

[Saved comparison](cache-working-set.json), same M4 / 16 GiB host and Rust 1.98.0.
The four originals are copied into 96 separate files (distinct device/inode keys),
then prefix cohorts of 4/16/32/48/64/96 are listed. Content cycles through the four
source images and their decode costs; this is a controlled identity/collision experiment, not a sample of
96 different photographs. Sources occupy about 120 MiB and remain unchanged across
the three binary runs. Recreating their identities changes hash collisions.

Each listing is an explicit grid, with `--preview-limit=256`; all names/previews
fit the existing 8 MiB output cap. Geometry A is 122×40 cells at 8×17 pixels/cell
(176×85 thumbnail), and B uses 8×16 (176×80). Fixed-size cases repeat A; alternating
cases switch A/B. The OS cache is warm, with two full passes before timing, then
five fresh processes for fixed/off modes and ten for alternating mode. A drained
PTY has no terminal renderer. Cache stats are included in every timed invocation.

| Sources / pattern | Direct slot: hits / median | Four-slot pilot | Eight-slot selected |
| --- | ---: | ---: | ---: |
| 4, repeated | 100% / 4.76 ms | 100% / 4.98 ms | 100% / 4.76 ms |
| 16, alternating | 62.50% / 98.74 ms | 100% / 9.17 ms | 100% / 9.01 ms |
| 32, repeated | 56.25% / 216.42 ms | 93.12% / 35.52 ms | 100% / 15.97 ms |
| 32, alternating | 15.62% / 477.11 ms | 68.44% / 208.30 ms | 77.50% / 157.29 ms |
| 48, repeated | 50.00% / 456.10 ms | 87.50% / 97.61 ms | 96.67% / 63.55 ms |
| 64, repeated | 43.75% / 621.44 ms | 73.75% / 220.10 ms | 82.19% / 224.10 ms |
| 96, repeated | 25.00% / 1,239.89 ms | 39.79% / 899.61 ms | 44.79% / 976.11 ms |
| 96, alternating | 6.25% / 1,692.69 ms | 4.06% / 1,654.36 ms | 6.04% / 1,664.10 ms |

Eight candidates materially improve small/medium working sets without raising the
64-record / 19.05 MiB storage bound or writing on hits. The 32-source repeated
case is about 13.6× faster than direct mapping. The four-slot trial is cheaper in
some overloaded cases; eight is not universally fastest. Random victims and the
different costs of JPEG/GIF/WebP misses mean hit percentage does not map directly
to elapsed time. These are modest sample counts with no statistical-significance
claim; the report retains minima/maxima where collected.

The 96-source alternating case requests 192 thumbnail keys from 64 slots and gains
little versus uncached (~1,692 ms for current geometry A). Keep caching opt-in and
avoid increasing storage without a daily-use need. The uncached fixed runs remain
close across binaries (e.g. 32 sources: 540/545/550 ms); four warm sources remain
about 4.8 ms. No text/layout/decoder path was changed by this chunk.

Trace-only models also compare 1/2/4/8-way FIFO and 2/4/8-way random replacement.
For 64 sources alternating sizes, both four/eight-way FIFO models get zero hits
after warmup: sequential listings evict the next required record. Random-victim
models avoid that systematic pattern without persisting access times. Model
numbers are not timings; Python seeded random sampling approximates the Rust
per-invocation randomly seeded key hash. The actual executable measurements above
are the evidence for the selected policy.

Separate RSS samples at geometry A show the 32-source repeated case dropping from
35.97 to 2.09 MiB. Four warm sources remain 2.03 MiB. Cases that decode misses
still reach about 31–39 MiB; the 96-source alternating sample rises from 32.64 to
39.28 MiB with different missed sources/allocator reuse. No new process-memory
bound is claimed. The four-slot pilot has no RSS sample. Output digests match
across every policy/mode; the largest listing emits ~7.7 MB and retains all 96
previews. All measured cache error counts are zero.

Reproduce with `python3 benchmarks/cache_working_set.py --label current`.
On macOS, append `--memory-only` after the timing run to add separate RSS samples
(kernel timing permission is required). To compare a saved binary, supply
`--binary target/lsa-before-cache-associativity --label direct`; do not recreate
the ignored fixture between comparisons. Reports go to
`benchmarks/local/working-set-LABEL.json`; the committed summary stays unchanged.
The final release binary is 1,279,600 bytes, 144 more than `7050349`.

## Opt-in thumbnail cache: 2026-09-05 experiment

Initial direct-mapped implementation at `7050349`; same Apple M4 / 16 GiB host,
arm64 macOS 26.6.2, Rust 1.98.0 release build.
[Saved results](cache.json). Graphics use 122×40 cells, 8×17 pixels per cell,
15 fresh timed processes after three warmups, warmed OS cache, and a drained PTY
without a renderer. RSS uses a separate `/usr/bin/time -l` process.

| Four user images | Median elapsed | App peak RSS | Cache stats |
| --- | ---: | ---: | --- |
| Disabled | 71.76 ms | 31.36 MiB | No cache access |
| Empty before each run | 71.82 ms | 31.41 MiB | 4 misses, 4 writes |
| Warm | 4.60 ms | 2.03 MiB | 4 hits, no writes |
| Metadata invalidated before each run | 72.34 ms | 31.39 MiB | 4 misses, 4 writes |

Warm application/PTY elapsed time improves about **15.6×** in this fixture.
Disabled, empty, and warm runs emit identical 320,685-byte output, including all
four image payloads and the generated-folder entry. Four records store 239,696
bytes. The invalidation fixture copies the original images, omits the generated
folder, and advances source mtimes before each process; setup is excluded from
timings. Original user images remain untouched. Stale keys occupied 44 slots /
2.64 MB by the end of this invalidation run, within the fixed slot cap.

The 40-link synthetic grid references one cheap source. Disabled/empty/warm runs
take 27.16/17.17/17.43 ms; an initially empty cache produces one miss followed by
39 hits in the same invocation. Warm uses ~2.09 MiB RSS and one 59,924-byte record,
while output stays 3,205,136 bytes. The small empty/warm timing difference is noise
at this resolution, not evidence that empty storage is faster.

| 10,000 names to sink | Median elapsed | App peak RSS |
| --- | ---: | ---: |
| Saved pre-cache binary | 7.23 ms | 4.19 MiB |
| Current, cache disabled | 7.32 ms | 4.22 MiB |
| Current, cache configured but unused | 7.20 ms | 4.23 MiB |

Text uses 21 fresh processes after three warmups. All variants use the same
directory; configured text output is checked not to create storage. This fixture
has a longer absolute path than the earlier text reports, which can affect retained
entry memory; use the same-session comparison above for the cache change. Binary size is
1,279,456 bytes, 17,152 more than the metadata build.

Reproduce with `python3 benchmarks/cache.py --before target/lsa-before-cache`
(omit `--before` without that saved binary). It writes only
`benchmarks/local/cache.json` and ignored local fixtures/storage. Clear and mtime
setup occur outside measured intervals; cache creation/insertion remains inside.
No filesystem-cold, visible-terminal latency, terminal memory, default-cache, or
large-working-set benefit is claimed. Keep caching opt-in while gathering those
use cases; see [cache design](../docs/cache.md).

## Metadata controls: 2026-09-05 update

Same host, 10,000 ordinary files, warmed OS cache, output to sink. Twenty-one fresh
processes after three warmups; RSS measured separately. [Saved results](metadata.json).
The before binary was copied from `364976f` before rebuilding; comparison used the
same prepared directory in one session. No image decoding in these cases.

| Invocation | Before | After | After peak RSS |
| --- | ---: | ---: | ---: |
| `-1` | 6.76 ms | 6.77 ms | 3.59 MiB |
| `-l` | 21.62 ms | 24.87 ms | 5.33 MiB |
| `--fields=size,modified -h` | — | 22.53 ms | 5.34 MiB |
| `--dirs-first -1` | — | 7.04 ms | 3.59 MiB |

Dynamic metadata alignment adds a formatting pass and about 3.3 ms to this large
long listing, retaining only six widths. Both long modes request the same metadata;
selecting fewer fields reduces formatting, not filesystem calls. The default path
stayed steady. Grouping correctness is tested on mixed fixtures; the timing fixture
contains ordinary files. No claim about a different filesystem or cold OS cache.

Reproduce with `python3 benchmarks/measure.py --before target/lsa-before-metadata`
when that saved binary is available, or omit `--before` for current-only results.
The script now writes `benchmarks/local/metadata.json`; committed reports remain
unchanged. Release binary: 1,262,304 bytes. Existing image/default PTY cases were
rerun and are included in the saved data.

## Default layouts: 2026-09-05 update

Same Apple M4 / 16 GiB host and build settings; binary 1,262,288 bytes. The original
results below remain unchanged. [New results](defaults.json) include default TTY
output at the user's 122×40 geometry, with 8×17 pixels per cell.

| Default invocation | Layout | Median elapsed | App peak RSS | Output |
| --- | --- | ---: | ---: | --- |
| Empty directory | Columns | 1.96 ms | 1.70 MiB | 0 bytes |
| 10,000 ordinary files | 7 text columns | 9.79 ms | 3.69 MiB | 1,429 lines, 170,000 bytes |
| 40 synthetic image links | 8 text columns | 2.95 ms | 1.81 MiB | 5 lines, 600 bytes; no decoding |
| Four user images plus generated folder | Automatic grid | 71.01 ms | 31.27 MiB | 4 previews, 320,685 bytes |

Seven measured runs after three warmups, fresh processes, warmed OS cache, no
thumbnail cache. PTY times include Python drain/poll overhead; no visible terminal
rendering is measured. The larger image fixture stays text because its grid would
exceed one screen. Column planning stores one width per entry, not escaped labels.

The old sink/explicit-grid workloads were rerun alongside these: 10,000 names to a
sink took 6.71 ms versus the original 6.69 ms; four user images with explicit grid
at 80×24 took 71.30 ms versus 71.05 ms. Both stayed close to the initial baseline.
Reproduce with `python3 benchmarks/measure.py`;
that report was saved as `defaults.json`; the current script includes those workloads
and writes `benchmarks/local/metadata.json` without replacing committed reports.

## Original inline prototype

2026-09-05; Apple M4, 16 GiB RAM; local macOS 26.6.2 (25G83), arm64.
Rust/Cargo 1.98.0, release build with thin LTO and stripping; binary 1,245,792 bytes.
[Saved results](baseline.json); reproduce with `python3 benchmarks/measure.py`.

| Work | Median elapsed | App peak RSS | Output |
| --- | ---: | ---: | ---: |
| lsa, empty directory | 1.84 ms | 1.67 MiB | Text to sink |
| system ls, empty directory | 1.34 ms | 1.19 MiB | Text to sink |
| lsa, 10,000 ordinary files | 6.69 ms | 3.59 MiB | Text to sink |
| system ls, same files | 11.52 ms | 2.58 MiB | Text to sink |
| lsa, 40 synthetic previews | 26.58 ms | 2.58 MiB | 3,290,658 bytes to PTY |
| lsa, four user images | 71.05 ms | 31.34 MiB | 329,263 bytes to PTY |

Text: 21 timed fresh processes after three warmups, `-1`, `LC_ALL=C`, fixed ASCII
names in `benchmarks/local/text-10000/`. Results include process launch and complete
output. These names have no type suffixes/escaping differences between programs;
general `ls` semantics still differ. RSS is a separate `/usr/bin/time -l` invocation,
reported in bytes by macOS. Local socket/kernel timing calls need sandbox permission
in the Codex execution environment; ordinary shell execution does not.

Graphics: seven fresh processes after three warmups; `--grid`, 80×24 cells,
640×384 reported pixels, three columns of 192×80-pixel thumbnail canvases. Python
drains a local PTY with no renderer; timings include harness/drain overhead and up
to roughly 1 ms final polling delay. Synthetic fixture: 40 symlinks to one 320×160
PNG, each decoded again. User fixture: one 2924×1932 JPEG, one 190×200 GIF, two WebP
files, plus the generated directory entry. User images are not committed.

At this baseline there was **no thumbnail cache**: every run decoded source images.
The OS file cache was warmed and not flushed; these are not filesystem-cold measurements. Terminal
CPU/memory, time to visible names/first image, scrollback cost, real-terminal output
latency, and browser idle cost are unmeasured. No GUI performance comparison.

The first baseline exposed wasted entry storage: `Option<Metadata>` reserved the
large platform stat object for every ordinary name. Moving requested metadata to
an optional box reduced 10,000-entry RSS from 6.64 to 3.59 MiB (~46%); elapsed time
changed from 7.54 to 6.69 ms in consecutive runs. RSS remains proportional to entry
count; full paths and names are retained for sorting and preview requests. No hard
performance threshold is set from this single-host sample.

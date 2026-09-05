# Application measurements

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
the script writes `benchmarks/local/defaults.json` and does not replace saved reports.

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

There is **no thumbnail cache**: every run decodes source images. The OS file cache
is warmed and was not flushed; these are not filesystem-cold measurements. Terminal
CPU/memory, time to visible names/first image, scrollback cost, real-terminal output
latency, and browser idle cost are unmeasured. No GUI performance comparison.

The first baseline exposed wasted entry storage: `Option<Metadata>` reserved the
large platform stat object for every ordinary name. Moving requested metadata to
an optional box reduced 10,000-entry RSS from 6.64 to 3.59 MiB (~46%); elapsed time
changed from 7.54 to 6.69 ms in consecutive runs. RSS remains proportional to entry
count; full paths and names are retained for sorting and preview requests. No hard
performance threshold is set from this single-host sample.

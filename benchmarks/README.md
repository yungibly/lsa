# Application measurements

## Inline product reset — 2026-09-05

[Paired report](inline.json), arm64 macOS 26.6.2, Rust 1.98.0, release/thin-LTO/stripped.
The before binary is the uncommitted automatic-pager implementation saved at the
start of this change; both binaries are identified by SHA-256 in the report.
Two warmups, then 11 paired/interleaved fresh processes per case. OS caches warm,
not flushed. Terminal output goes to a drained 122×40 PTY with 8×17 cell pixels;
stdin is `/dev/null`, so the former automatic pager cannot capture input.

| Median elapsed time | Before | Current |
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

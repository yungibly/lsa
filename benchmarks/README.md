# Application measurements

Paired measurements compare a saved "before" binary with the current release
build. The harnesses run warmups, then fresh processes in alternating order. They
drain output from a pipe or from a 122×40 PTY with 8×17-pixel cells, without a
renderer, with stdin at `/dev/null`. `wait4` reports child CPU time and peak RSS;
elapsed time ends when the child exits. OS caches are warm. Reports record exact
binary hashes. Fixtures and fresh reports stay under the ignored
`benchmarks/local/`; committed reports are copied explicitly. None of these
measure terminal rendering time or terminal image memory.

## v0.5.0 — 2026-09-28

[Report](previews-0.5.0.json), [harness](previews.py). Before: the v0.4.0 release
binary. After: the v0.5.0 release build. Apple M4 (10 cores, 4 performance),
16 GB, macOS 27.0, two warmups and 9 paired runs, thumbnail cache off. Galleries
use 64 symlinks to one 2924×1932 JPEG from `img-test` (4.5 MB), a HEIC copy made
with `sips`, or generated folders. Medians:

| Case | v0.4.0 | v0.5.0 | Notes |
| --- | ---: | ---: | --- |
| 10,000 plain names, pipe | 9.58 ms, 4.2 MiB | 9.00 ms, 2.8 MiB | identical output |
| 10,000 long entries, pipe | 23.61 ms, 5.3 MiB | 23.07 ms, 4.0 MiB | identical output |
| 10,000 long entries, terminal | 53.10 ms | 47.09 ms | 2.88 MB → 2.51 MB of output |
| 64 JPEG photos, grid | 2,753 ms, 534 MiB | 381 ms, 53 MiB | 1.97 MB → 1.15 MB sent |
| 64 JPEG photos, long-view miniatures | 2,687 ms, 535 MiB | 378 ms, 51 MiB | |
| 256 folders, grid | 65.3 ms | 5.8 ms | 7.88 MB → 0.33 MB sent |
| Four sample images, default layout | 73.8 ms | 32.2 ms | |
| 64 HEIC photos, grid | 20 ms (artwork only) | 1,056 ms | real previews now |

Where the gains come from:

- **Memory:** the JPEG decoder's whole-file read now reserves its length once;
  about 8 MiB per photo above 4 MiB had stayed resident. A 64-photo gallery
  peaked at 561 MB with the previous binary and 24 MB with only this fix.
- **Speed:** up to four decoders in parallel. On macOS, JPEGs decode through
  ImageIO at reduced resolution. One photo previews in ~30 ms at a 9 MB peak
  footprint instead of ~40 ms at 29 MB, and 48 MP JPEG/HEIC/AVIF files peak at
  about 13-25 MB.
  Workers alone took the 64-photo gallery from 3.34 s to 0.71 s (1 to 4 workers:
  2.76, 1.40, 0.71 s; 8 workers 0.58 s at twice the memory).
- **Bytes:** zlib payloads (`o=z`) and artwork rendered once; decoded pixels are
  unchanged.
- **Listings:** name-only entries, classification computed once (and skipped for
  plain pipes), coalesced permission colors and default-foreground names.
- **HEIC:** 64 previews take about a second, and elapsed time equals CPU time,
  so ImageIO appears to serialize HEVC decoding; the 64-JPEG gallery, by
  contrast, spreads 1.5 s of CPU across four workers.

Earlier distinct-timestamp reports (below, 2026-09-10/12) measured ~530-560 ms
for 10,000 long entries on macOS 26.6.2. The same fixture takes ~23 ms on this
machine: that cost was the earlier host's local-time conversion, not lsa's.

```sh
cargo build --release --locked --examples --bins
python3 benchmarks/previews.py --before target/lsa-before-v050 --runs=9
```

## Earlier reports

| Date | Change | Reports | Harness | Summary |
| --- | --- | --- | --- | --- |
| 2026-09-22 | Automatic hyperlinks (v0.4.0) | [hyperlinks](hyperlinks.json) | [hyperlinks.py](hyperlinks.py) | Links add 0.95-4.26 ms CPU per 10,000 entries; forced-link streaming 8% cheaper |
| 2026-09-12 | Formatting and artwork efficiency (v0.3.1) | [listings](efficiency-listings.json), [artwork](efficiency-artwork.json), [images](efficiency-images.json) | [performance.py](performance.py), [artwork.py](artwork.py), [inline.py](inline.py) | 4-7% less CPU for styled/long listings, 4.8-7.5% less for artwork |
| 2026-09-10 | Long defaults, compact metadata (v0.3.0) | [listings](performance.json), [images](performance-images.json) | [performance.py](performance.py), [inline.py](inline.py) | Long-listing RSS 14% lower; one local-time conversion per entry |
| 2026-09-07 | Final artwork pass (v0.2.0) | [artwork](artwork.json) | [artwork.py](artwork.py) | 16-22% less artwork CPU, identical pixels |
| 2026-09-07 | Larger galleries, SVG/ICO (v0.2.0) | [options](gallery-options.json), [efficiency](gallery-efficiency.json) | [gallery.py](gallery.py), [inline.py](inline.py) | Rotating JPEGs after thumbnailing: 25% faster, 47% less memory |
| 2026-09-06 | Smaller grids, miniatures | [thumbnail UX](thumbnail-ux.json) | [inline.py](inline.py) | 52% fewer graphics bytes for mixed grids |
| 2026-09-05 | Inline product reset | [inline](inline.json) | [inline.py](inline.py) | Pager removed; plain output unchanged |
| Earlier | Baseline, defaults, metadata, cache, removed browser/pager | [history/](history/) | — | Superseded implementations |

[cache_working_set.py](cache_working_set.py) remains a focused cache experiment
(eight candidate slots: 100% hits for a repeated 32-source working set).

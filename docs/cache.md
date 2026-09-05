# Thumbnail cache experiment

Implemented and automated-tested on arm64 macOS 26.6.2 with Rust 1.98.0.
Caching remains opt-in. No new Ghostty visual pass or Linux verification is claimed.

## Controls and lazy behavior

- `--cache-dir=PATH` (or `--cache-dir PATH`) enables storage in
  `PATH/lsa-thumbnails-v1/`. No environment variable or default user cache path.
- `--no-cache` wins regardless of option order and disables reads and writes.
- `--clear-cache --cache-dir=PATH` clears known thumbnail slots and staging, then
  exits. Paths, `--diagnose`, and `--no-cache` cannot accompany clear. It returns
  status 1 on contention or storage errors, and succeeds without creating a missing
  namespace. The lock file and unrelated files remain. Other invocations can
  populate the cache again after clear releases the lock.
- `--cache-stats` reports hits, misses, successful writes, and cache errors to
  stderr. Errors include corruption, inaccessible storage, and lock contention;
  collisions and absent records are ordinary misses. A miss and failed write can
  count two errors. Disabled/unused caches report zeros. This flag is for measuring
  the experiment; normal successful listings remain quiet.
- `--diagnose` reports the configured directory, namespace, slots, and byte cap
  without opening storage. Text layouts, pipes, entries without preview candidates,
  and exhausted attempt/byte budgets do not initialize or access the cache.

The cache object is invocation-local and shared across operands. Storage opens
lazily on the first eligible, budgeted preview with a readable bounded source.
Initialization failure is remembered for the invocation. Ordinary cache failures
fall back to decoding; decoding failures keep the original entry and placeholder.

## Identity and records

Each 80-byte key contains the version, source device/inode, byte size, nanosecond
mtime and ctime, and thumbnail pixel width/height. Including ctime handles edits
that restore mtime. The source is opened and checked as a regular file within the
32 MiB input cap before lookup, so warm records cannot bypass source access or
file-type checks. An fd metadata recheck follows a hit and precedes insertion.
Sources that change during decoding are not inserted.

Symlinks and hard links to the same source share its key. Retargeting/replacement
changes identity; changing geometry changes the key. Names, tile cell coordinates,
terminal image IDs, and placements are never cached. The record holds a complete
key, four-byte CRC32 checksum over key and pixels, and raw RGBA pixels. Raw pixels
avoid running another image codec on hits and match the existing Kitty payload.
The version must change when the record, decoder/limits, orientation, resize, or
checker transform changes. Both the namespace and record magic are versioned.

A checksum of the key chooses one of 64 slots. A hit requires the complete key to
match; hash collisions cannot return another source's thumbnail. Reads validate
regular-file type, bounded/exact length, and checksum before returning pixels.
Truncation, corruption, and old versions become misses and are replaced after a
successful decode. CRC32 detects accidental corruption, not malicious forgery.
Identity assumes trustworthy filesystem metadata; there is no source-content hash.

## Storage and concurrency

Direct mapping deliberately replaces an LRU index: there is no directory scan,
access-time update, background cleaner, or unbounded record list. A collision
replaces its slot even if other slots are empty. Old source/geometry versions can
occupy slots until replaced or cleared. A small or unlucky working set can thrash;
64 slots do not promise 64 simultaneously useful hits.

At most 64 records plus one staging file contain lsa-written data. Each is at most
307,284 bytes (84-byte header plus 320×240×4 pixels), for **19,973,460 bytes** total,
about 19.05 MiB. One persistent empty lock file is additional. This is a bound on
file contents in the current namespace, not filesystem block/metadata overhead,
externally supplied files, or the sum of independently selected cache directories.

All processes share the same permanent lock inode. Nonblocking shared locks cover
reads; exclusive locks cover insertion and clear. Locks never cover decoding or
terminal output. Busy operations skip caching immediately. Serialization bounds
staging to one file and prevents readers retaining old unlinked records while
writers refill slots. Processes need to honor local Unix `flock` semantics; network
filesystem behavior is unverified. Filesystem calls themselves have no timeout.

Writers remove stale staging, create it exclusively, write a complete record,
and rename it atomically over its slot. Failed writes attempt staging cleanup;
interrupted writes leave at most the reserved staging file for the next writer or
clear. No fsync is performed: this disposable cache promises atomic publication,
not survival of a power loss. Invalid post-crash records recover by decoding.

The namespace is private (0700), owned by the current user; files are created 0600.
Namespace and record opens refuse final-component symlinks. File operations use an
open directory descriptor, and clear removes only known names without following
symlinks. Read-only storage can serve existing hits; misses still decode if writes
fail. The selected parent path follows normal filesystem path resolution.

## Evidence and next decision

`cargo test --locked` covers identity/mtime restoration, replacement, link sharing
and retargeting, geometry, corruption, obsolete versions, oversized records,
collision eviction, the full storage cap, stale staging, read-only storage, source
permissions, lock contention, safe clear, and symlink refusal.
`python3 tests/check_cache.py` adds 23 release-binary PTY/CLI scenarios including
lazy paths, exact output equivalence, budgets across operands, concurrent fresh
processes, source replacement with FIFO/oversized files, and graceful fallback.

[The measurements](../benchmarks/README.md) show a large benefit for the four user
images, a smaller benefit for a cheap shared synthetic source, and steady text
cost. Keep the experiment opt-in while gathering a Ghostty empty/warm/disabled
visual comparison and daily-directory hit rates. These results do not yet justify
parallel decoding, a default cache location, or a browser implementation.

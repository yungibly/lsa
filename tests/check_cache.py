#!/usr/bin/env python3
"""Cache end-to-end checks; PTY payloads, not visible terminal rendering."""
from concurrent.futures import ThreadPoolExecutor
import fcntl
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

from check_pty import APC, BIN, ROOT, capture, images

STATS = re.compile(rb"lsa: cache: (\d+) hits, (\d+) misses, (\d+) writes, (\d+) errors\n")
NAMESPACE = "lsa-thumbnails-v1"


def run(args, **kwargs):
    code, data, err, _ = capture(["--cache-stats", "--preview-limit=64", *args], **kwargs)
    assert code == 0, err
    match = STATS.search(err)
    assert match, err
    return data, tuple(map(int, match.groups())), STATS.sub(b"", err)


def clear(cache):
    return subprocess.run([str(BIN), f"--cache-dir={cache}", "--clear-cache"],
                          capture_output=True)


def snapshot(directory):
    return {p.name: (p.stat().st_ino, p.stat().st_size, p.stat().st_mtime_ns)
            for p in directory.iterdir()}


def main():
    cases = 0
    fixture = ROOT / "img-test/generated"
    many = fixture / "many"
    with tempfile.TemporaryDirectory(dir=ROOT / "target", prefix="cache-pty-") as temp:
        root = Path(temp)
        cache = root / "cache"
        flag = f"--cache-dir={cache}"
        # No cache access on pipes, text overrides, diagnostics, empty listings,
        # no candidates, or a zero preview budget, even when explicitly enabled.
        for args in [["--grid"], ["-l"], ["--diagnose"]]:
            process = subprocess.run([str(BIN), flag, *args, str(many)], capture_output=True)
            assert process.returncode == 0 and not process.stderr and not cache.exists()
            cases += 1
        for args in [["-1"], ["--no-images"], ["--protocol=none"], ["--fields=size", "--no-images"],
                     ["--diagnose"], ["--preview-limit=0"], ["--no-cache"]]:
            _, stats, _ = run([flag, "--grid", *args, many])
            assert stats == (0, 0, 0, 0) and not cache.exists()
            cases += 1
        empty = root / "empty"
        empty.mkdir()
        for name in [None, "ordinary.txt"]:
            if name:
                (empty / name).touch()
            _, stats, err = run([flag, "--grid", empty])
            assert not err and stats == (0, 0, 0, 0) and not cache.exists()
            cases += 1

        # All forty symlinks share one source identity and size.
        uncached, _, _ = run(["--grid", many])
        cold, stats, err = run([flag, "--grid", many])
        assert cold == uncached and not err and stats == (39, 1, 1, 0), stats
        namespace = cache / NAMESPACE
        before = snapshot(namespace)
        warm, stats, err = run([flag, "--grid", many])
        assert warm == uncached and not err and stats == (40, 0, 0, 0)
        assert snapshot(namespace) == before, "hits rewrote cache files"
        cases += 1
        for flags in [[flag, "--no-cache"], ["--no-cache", flag]]:
            data, stats, err = run([*flags, "--grid", many])
            assert data == uncached and stats == (0, 0, 0, 0) and not err
            assert snapshot(namespace) == before
            cases += 1

        # Budgets count cached previews too, and remain shared across operands.
        data, stats, err = run([flag, "--grid", "--preview-limit=2", many, many])
        assert len(images(data)) == 3 and stats == (2, 0, 0, 0) and not err
        cases += 1
        large_args = [flag, "--grid", many]
        run(large_args, pixels=(2560, 1920))
        data, stats, err = run(large_args, pixels=(2560, 1920))
        assert 0 < stats[0] == len(images(data)) < 40 and stats[1:] == (0, 0, 0)
        assert not err and sum(len(m[0]) for m in APC.finditer(data)) <= 8 * 1024 * 1024
        cases += 1
        # Cache corruption cannot alter either pixels or mixed-entry text.
        expected, _, expected_err = run(["--grid", fixture])
        for _ in range(2):
            data, _, err = run([flag, "--grid", fixture])
            assert data == expected and err == expected_err
        for path in namespace.glob("*.rgba"):
            payload = bytearray(path.read_bytes())
            payload[-1] ^= 1
            path.write_bytes(payload)
        data, stats, err = run([flag, "--grid", fixture])
        assert data == expected and err == expected_err and stats[3] > 0
        cases += 1
        # A locked cache never stalls previews, and clear reports failure.
        with (namespace / "lock").open("rb") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            data, stats, err = run([flag, "--grid", many])
            assert data == uncached and not err and stats == (0, 40, 0, 80)
            assert clear(cache).returncode == 1
        cases += 1
        # Concurrent fresh processes contend for the same slots and staging file.
        assert clear(cache).returncode == 0
        with ThreadPoolExecutor(max_workers=8) as pool:
            results = list(pool.map(lambda _: run([flag, "--grid", many]), range(16)))
        for data, _, err in results:
            assert data == uncached and not err
        _, stats, err = run([flag, "--grid", many])
        assert not err and stats == (40, 0, 0, 0)
        assert not (namespace / "staging.tmp").exists()
        assert len(list(namespace.iterdir())) == 2
        cases += 1
        # Replacing a previously previewable source with a FIFO/oversized source
        # must fail safely before touching cache; all names still appear.
        source = root / "source.png"
        shutil.copyfile(fixture / "landscape.png", source)
        run([flag, "--grid", source])
        source.unlink()
        os.mkfifo(source)
        link = root / "link.png"
        link.symlink_to(source)
        data, stats, err = run([flag, "--grid", link])
        assert len(images(data)) == 1 and stats == (0, 0, 0, 0) and not err
        source.unlink()
        with source.open("wb") as output:
            output.truncate(32 * 1024 * 1024 + 1)
        data, stats, err = run([flag, "--grid", source])
        assert len(images(data)) == 1 and stats == (0, 0, 0, 0) and not err
        cases += 1
        # A broken cache path falls back without emitting preview failures.
        bad = root / "bad"
        bad.write_text("keep")
        data, stats, err = run([f"--cache-dir={bad}", "--grid", many])
        assert data == uncached and not err and stats == (0, 40, 0, 40)
        assert bad.read_text() == "keep"
        cases += 1
        marker = namespace / "keep"
        marker.write_text("unrelated")
        assert clear(cache).returncode == 0
        assert {p.name for p in namespace.iterdir()} == {"lock", "keep"}
        assert marker.read_text() == "unrelated"
        assert clear(root / "missing").returncode == 0 and not (root / "missing").exists()
        cases += 1
    print(f"{cases} cache PTY/CLI scenarios passed; visual rendering unverified.")


if __name__ == "__main__":
    main()

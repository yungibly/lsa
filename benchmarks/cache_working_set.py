#!/usr/bin/env python3
"""Independent-source cache capacity and alternating-geometry measurements."""
import argparse
from datetime import date
import hashlib
import importlib.util
import json
from pathlib import Path
import platform
import random
import re
import shutil
import statistics
import struct
import subprocess
import zlib

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("check_pty", ROOT / "tests/check_pty.py")
pty_check = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pty_check)
STATS = re.compile(rb"lsa: cache: (\d+) hits, (\d+) misses, (\d+) writes, (\d+) errors\n")
GEOMETRIES = [dict(cols=122, rows=40, pixels=(976, 680)),
              dict(cols=122, rows=40, pixels=(976, 640))]


def summarize(runs):
    hits = sum(run[1][0] for run in runs)
    misses = sum(run[1][1] for run in runs)
    return {"median_ms": round(statistics.median(run[0] for run in runs) * 1000, 3),
            "min_ms": round(min(run[0] for run in runs) * 1000, 3),
            "max_ms": round(max(run[0] for run in runs) * 1000, 3),
            "runs": len(runs), "hits": hits, "misses": misses,
            "hit_percent": round(100 * hits / (hits + misses), 2) if hits + misses else None,
            "writes": sum(run[1][2] for run in runs),
            "errors": sum(run[1][3] for run in runs)}


def key_hash(path, geometry):
    """v1 key hashes for trace-only policy comparisons; not used to validate lsa."""
    stat = path.stat()
    key = b"LSATHM01" + struct.pack("<9Q", stat.st_dev, stat.st_ino, stat.st_size,
                                  stat.st_mtime_ns // 10**9, stat.st_mtime_ns % 10**9,
                                  stat.st_ctime_ns // 10**9, stat.st_ctime_ns % 10**9,
                                  176, 85 if geometry == 0 else 80)
    return zlib.crc32(key)


def simulate(paths, alternate, ways, seed=None):
    """64 slots, FIFO or seeded random victims; a model, not measured I/O."""
    sets = [[] for _ in range(64 // ways)]
    rng = random.Random(seed)
    hashes = {(str(path), geometry): key_hash(path, geometry)
              for path in paths for geometry in range(2 if alternate else 1)}
    hits = attempts = 0
    for invocation in range(12):
        geometry = invocation % 2 if alternate else 0
        for path in paths:
            key = (str(path), geometry)
            bucket = sets[hashes[key] % len(sets)]
            hit = key in bucket
            if not hit:
                if len(bucket) == ways:
                    bucket.pop(0 if seed is None else rng.randrange(ways))
                bucket.append(key)
            if invocation >= 2:
                hits += hit
                attempts += 1
    return round(100 * hits / attempts, 2)


def models(paths, alternate):
    return {"fifo": {str(ways): simulate(paths, alternate, ways) for ways in [1, 2, 4, 8]},
            "random_mean_10_seeds": {str(ways): round(statistics.mean(
                simulate(paths, alternate, ways, seed) for seed in range(10)), 2)
                for ways in [2, 4, 8]}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/lsa")
    parser.add_argument("--label", default="current")
    parser.add_argument("--runs", type=int, default=5)
    parser.add_argument("--memory-only", action="store_true",
                        help="Add separate macOS RSS samples to an existing timing report")
    args = parser.parse_args()
    assert re.fullmatch(r"[a-z0-9-]+", args.label) and 3 <= args.runs <= 21
    binary = args.binary.resolve()
    binary.relative_to(ROOT)
    pty_check.BIN = binary
    local = ROOT / "benchmarks/local/working-set"
    destination = ROOT / f"benchmarks/local/working-set-{args.label}.json"
    if args.memory_only:
        assert platform.system() == "Darwin"
        report = json.loads(destination.read_text())
        assert report["binary_sha256"] == hashlib.sha256(binary.read_bytes()).hexdigest()
        for case, record in report["cases"].items():
            count, mode = case.split("_")
            cache = local / args.label / f"storage-{count}-{mode}"
            flags = ["--grid", "--preview-limit=256", "--cache-stats",
                     "--no-cache" if mode == "off" else f"--cache-dir={cache}",
                     local / f"entries-{count}"]
            for geometry in [0, 1 if mode == "alternate" else 0]:
                code, _, err, _ = pty_check.capture(flags, **GEOMETRIES[geometry])
                assert code == 0 and STATS.fullmatch(err), err
            code, data, err, _ = pty_check.capture(flags, prefix=("/usr/bin/time", "-l"), **GEOMETRIES[0])
            assert code == 0, err
            rss = re.search(rb"(\d+)\s+maximum resident set size", err)
            assert rss and STATS.search(err), err
            assert hashlib.sha256(data).hexdigest() == record["output_sha256"]["0"]
            record["peak_rss_bytes"] = int(rss[1])
            destination.write_text(json.dumps(report, indent=2) + "\n")
        print(f"Added separate peak RSS samples at geometry A to {destination}")
        return
    sources = local / "sources"
    sources.mkdir(parents=True, exist_ok=True)
    originals = sorted(path for path in (ROOT / "img-test").iterdir() if path.is_file())
    assert len(originals) == 4, "This fixture expects the four original user images"
    # Exclusive first creation; reuse unchanged identities on subsequent runs.
    paths = []
    for index in range(96):
        original = originals[index % len(originals)]
        path = sources / f"image-{index:03}{original.suffix}"
        if not path.exists():
            with original.open("rb") as source, path.open("xb") as target:
                shutil.copyfileobj(source, target)
        assert path.stat().st_size == original.stat().st_size
        paths.append(path)
    assert len({(path.stat().st_dev, path.stat().st_ino) for path in paths}) == 96
    report = {"date": date.today().isoformat(), "label": args.label,
              "os": platform.platform(), "machine": platform.machine(),
              "binary_bytes": binary.stat().st_size,
              "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
              "conditions": "OS cache warmed, not flushed. Fresh processes, drained PTY, no renderer. Two warmup passes before timed repeats. Cache stats included in all timings. All previews fit existing attempt/output caps.",
              "fixtures": "96 independent file copies, cycling four user images; stable inodes reused across binary comparisons. Different identities can change collisions.",
              "source_bytes": sum(path.stat().st_size for path in paths),
              "geometry_a": "122x40, 8x17 cell pixels; 176x85 thumbnails",
              "geometry_b": "122x40, 8x16 cell pixels; 176x80 thumbnails",
              "cases": {}}
    for count in [4, 16, 32, 48, 64, 96]:
        directory = local / f"entries-{count}"
        directory.mkdir(exist_ok=True)
        for path in paths[:count]:
            link = directory / path.name
            if not link.exists():
                link.symlink_to(path)
        assert len(list(directory.iterdir())) == count
        expected = {}
        for mode in ["off", "repeat", "alternate"]:
            cache = local / args.label / f"storage-{count}-{mode}"
            cache_flag = f"--cache-dir={cache}"
            subprocess.run([str(binary), cache_flag, "--clear-cache"], capture_output=True, check=True)
            flags = ["--grid", "--preview-limit=256", "--cache-stats",
                     "--no-cache" if mode == "off" else cache_flag, directory]
            runs = []
            # Alternate mode records equal numbers of A and B invocations.
            run_count = args.runs * 2 if mode == "alternate" else args.runs
            for index in range(run_count + 2):
                geometry = index % 2 if mode == "alternate" else 0
                code, data, err, elapsed = pty_check.capture(flags, **GEOMETRIES[geometry])
                match = STATS.fullmatch(err)
                assert code == 0 and match, err
                stats = tuple(map(int, match.groups()))
                assert stats[3] == 0, err
                digest = hashlib.sha256(data).hexdigest()
                if geometry not in expected:
                    # Validate alternate geometry against a separate uncached run.
                    if geometry == 1:
                        code, baseline, baseline_err, _ = pty_check.capture(
                            ["--grid", "--preview-limit=256", "--no-cache", directory], **GEOMETRIES[geometry])
                        assert code == 0 and not baseline_err, baseline_err
                        assert baseline == data
                    expected[geometry] = digest
                    assert len(pty_check.images(data)) == count
                assert digest == expected[geometry], "cache changed output"
                assert sum(len(m[0]) for m in pty_check.APC.finditer(data)) <= 8 * 1024 * 1024
                if mode != "off":
                    assert stats[0] + stats[1] == count
                if index >= 2:
                    runs.append((elapsed, stats))
            record = summarize(runs)
            record["output_bytes_last_run"] = len(data)
            record["output_sha256"] = {str(key): value for key, value in expected.items()}
            records = list((cache / "lsa-thumbnails-v1").glob("*.rgba"))
            record["stored_records"] = len(records)
            record["stored_bytes"] = sum(path.stat().st_size for path in records)
            assert len(records) <= 64 and record["stored_bytes"] <= 19_973_460
            if mode != "off":
                record["modeled_hit_percent_by_ways"] = models(paths[:count], mode == "alternate")
            report["cases"][f"{count}_{mode}"] = record
            print(f"{args.label} {count:2} {mode:9}: {record['median_ms']:8.2f} ms, {record['hit_percent']}% hits", flush=True)
            # Preserve completed cases if a later measurement is interrupted.
            destination.write_text(json.dumps(report, indent=2) + "\n")
    print(destination)


if __name__ == "__main__":
    main()

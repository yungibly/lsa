#!/usr/bin/env python3
"""Opt-in cache experiment. All fixtures and cache data stay in benchmarks/local/."""
import argparse
from datetime import date
import importlib.util
import json
import os
from pathlib import Path
import platform
import re
import shutil
import statistics
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / "target/release/lsa"
spec = importlib.util.spec_from_file_location("check_pty", ROOT / "tests/check_pty.py")
pty_check = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pty_check)
GEOMETRY = dict(cols=122, rows=40, pixels=(976, 680))


def summary(samples):
    return {"median_ms": round(statistics.median(samples) * 1000, 3),
            "min_ms": round(min(samples) * 1000, 3), "runs": len(samples)}


def peak_rss(err):
    match = re.search(rb"(\d+)\s+maximum resident set size", err)
    assert match, err
    return int(match[1])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--before", type=Path, help="Saved binary from before cache implementation")
    args = parser.parse_args()
    local = ROOT / "benchmarks/local/cache-experiment"
    local.mkdir(parents=True, exist_ok=True)
    cache = local / "storage"
    flag = f"--cache-dir={cache}"

    def clear():
        subprocess.run([str(BIN), flag, "--clear-cache"], capture_output=True, check=True)

    result = {"date": date.today().isoformat(), "os": platform.platform(),
              "machine": platform.machine(), "binary_bytes": BIN.stat().st_size,
              "build": "cargo build --release --locked; thin LTO; stripped",
              "geometry": "122x40 cells, 8x17 pixels per cell",
              "conditions": "OS cache warmed, not flushed; fresh processes. PTY drained without renderer. Clear/touch setup excluded from timings. RSS measured separately.",
              "graphics_pty": {}, "text_sink": {}}
    user = ROOT / "img-test"
    fixtures = [("user_images", user, []),
                ("40_shared_source_links", user / "generated/many", ["--grid"])]
    # Work on copies to measure metadata invalidation without touching originals.
    copies = local / "invalidation-sources"
    copies.mkdir(exist_ok=True)
    for path in user.iterdir():
        if path.is_file():
            shutil.copyfile(path, copies / path.name)

    def invalidate():
        for path in copies.iterdir():
            stat = path.stat()
            os.utime(path, ns=(stat.st_atime_ns, stat.st_mtime_ns + 1))

    for label, path, layout in fixtures:
        expected = None
        for mode in ["off", "empty", "warm", "invalidated"]:
            source = copies if mode == "invalidated" and label == "user_images" else path
            if mode == "invalidated" and label != "user_images":
                continue
            flags = [*layout, "--no-cache" if mode == "off" else flag, source]
            clear()
            if mode in ["warm", "invalidated"]:
                code, _, err, _ = pty_check.capture(flags, **GEOMETRY)
                assert code == 0 and not err, err

            def prepare():
                if mode == "empty":
                    clear()
                elif mode == "invalidated":
                    invalidate()

            samples = []
            for i in range(18):
                prepare()
                code, data, err, elapsed = pty_check.capture(flags, **GEOMETRY)
                assert code == 0 and not err, err
                if expected is None:
                    expected = data
                if mode != "invalidated":
                    assert data == expected
                if i >= 3:
                    samples.append(elapsed)
            record = summary(samples)
            record["output_bytes"] = len(data)
            record["images"] = len(pty_check.images(data))
            prepare()
            code, _, err, _ = pty_check.capture(["--cache-stats", *flags], **GEOMETRY)
            assert code == 0, err
            record["cache_stats"] = err.decode().strip()
            if platform.system() == "Darwin":
                prepare()
                code, _, err, _ = pty_check.capture(flags, prefix=("/usr/bin/time", "-l"), **GEOMETRY)
                assert code == 0, err
                record["peak_rss_bytes"] = peak_rss(err)
            records = list((cache / "lsa-thumbnails-v1").glob("*.rgba"))
            record["stored_records"] = len(records)
            record["stored_bytes"] = sum(p.stat().st_size for p in records)
            result["graphics_pty"][f"{label}_{mode}"] = record

    text = local / "text-10000"
    text.mkdir(exist_ok=True)
    for i in range(10000):
        (text / f"entry-{i:05}.txt").touch(exist_ok=True)
    unused_cache = local / "text-cache-must-not-exist"
    assert not unused_cache.exists()
    variants = [("after_off", BIN, []), ("after_configured", BIN, [f"--cache-dir={unused_cache}"])]
    if args.before:
        variants.insert(0, ("before", args.before.resolve(), []))
    env = {**os.environ, "LC_ALL": "C"}
    for label, binary, flags in variants:
        command = [str(binary), *flags, "-1", str(text)]
        samples = []
        for i in range(24):
            start = time.perf_counter()
            subprocess.run(command, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, check=True, env=env)
            if i >= 3:
                samples.append(time.perf_counter() - start)
        record = summary(samples)
        if platform.system() == "Darwin":
            measured = subprocess.run(["/usr/bin/time", "-l", *command],
                                      stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, check=True, env=env)
            record["peak_rss_bytes"] = peak_rss(measured.stderr)
        result["text_sink"][label] = record
    assert not unused_cache.exists()
    report = ROOT / "benchmarks/local/cache.json"
    report.write_text(json.dumps(result, indent=2) + "\n")
    print(report.read_text())


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Local application baseline; creates fixtures only in benchmarks/local/."""
import importlib.util
from datetime import date
import json
import os
from pathlib import Path
import platform
import re
import statistics
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / "target/release/lsa"
spec = importlib.util.spec_from_file_location("check_pty", ROOT / "tests/check_pty.py")
pty_check = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pty_check)


def summary(samples):
    return {"median_ms": round(statistics.median(samples) * 1000, 3),
            "min_ms": round(min(samples) * 1000, 3), "runs": len(samples)}


def peak_rss(stderr):
    match = re.search(rb"(\d+)\s+maximum resident set size", stderr)
    return int(match[1]) if match else None


def main():
    local = ROOT / "benchmarks/local"
    empty, text = local / "empty", local / "text-10000"
    empty.mkdir(parents=True, exist_ok=True)
    text.mkdir(parents=True, exist_ok=True)
    for i in range(10000):
        (text / f"entry-{i:05}.txt").touch(exist_ok=True)
    result = {"date": date.today().isoformat(), "os": platform.platform(),
              "machine": platform.machine(), "binary_bytes": BIN.stat().st_size,
              "build": "cargo build --release; thin LTO; stripped",
              "cache": "No thumbnail cache. OS cache warmed, not flushed.",
              "text": {}, "graphics_pty": {}, "default_pty": {}}
    env = {**os.environ, "LC_ALL": "C"}
    for label, fixture in [("empty", empty), ("10000_files", text)]:
        for name, cmd in [("lsa", [str(BIN), "-1", str(fixture)]),
                          ("ls", ["/bin/ls", "-1", str(fixture)])]:
            samples = []
            for i in range(24):
                start = time.perf_counter()
                subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, check=True, env=env)
                if i >= 3:
                    samples.append(time.perf_counter() - start)
            record = summary(samples)
            if platform.system() == "Darwin":
                measured = subprocess.run(["/usr/bin/time", "-l", *cmd], stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, check=True, env=env)
                record["peak_rss_bytes"] = peak_rss(measured.stderr)
            result["text"][f"{label}_{name}"] = record
    fixtures = [("40_synthetic", ROOT / "img-test/generated/many")]
    user = ROOT / "img-test"
    if any(p.is_file() for p in user.iterdir()):
        fixtures.append(("user_images", user))
    for label, path in fixtures:
        samples = []
        for i in range(10):
            code, data, err, elapsed = pty_check.capture(["--grid", path])
            assert code == 0 and not err, err
            if i >= 3:
                samples.append(elapsed)
        record = summary(samples)
        record["output_bytes"] = len(data)
        record["image_bytes"] = sum(len(m[0]) for m in pty_check.APC.finditer(data))
        record["images"] = len(pty_check.images(data))
        if platform.system() == "Darwin":
            code, _, err, _ = pty_check.capture(["--grid", path], prefix=("/usr/bin/time", "-l"))
            assert code == 0
            record["peak_rss_bytes"] = peak_rss(err)
        result["graphics_pty"][label] = record
    # Measure the new default path at the user's reported Ghostty geometry.
    geometry = dict(cols=122, rows=40, pixels=(976, 680))
    for label, path in [("empty", empty), ("10000_files", text), *fixtures]:
        samples = []
        for i in range(10):
            code, data, err, elapsed = pty_check.capture([path], **geometry)
            assert code == 0 and not err, err
            if i >= 3:
                samples.append(elapsed)
        record = summary(samples)
        record["output_bytes"] = len(data)
        record["images"] = len(pty_check.images(data))
        if not record["images"]:
            record["text_lines"] = len(data.splitlines())
        code, diagnostic, err, _ = pty_check.capture(["--diagnose", path], **geometry)
        assert code == 0 and not err
        record["layout"] = re.search(rb"^layout=(\w+)", diagnostic, re.M)[1].decode()
        if platform.system() == "Darwin":
            code, _, err, _ = pty_check.capture([path], prefix=("/usr/bin/time", "-l"), **geometry)
            assert code == 0
            record["peak_rss_bytes"] = peak_rss(err)
        result["default_pty"][label] = record
    report = local / "defaults.json"
    report.write_text(json.dumps(result, indent=2) + "\n")
    print(report.read_text())


if __name__ == "__main__":
    main()

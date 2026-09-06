#!/usr/bin/env python3
"""Pager readiness/input measurements in a drained controlling PTY; no renderer."""
from datetime import date
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import select
import statistics
import subprocess
import sys
import termios
import time

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tests"))
from check_pager import Pager, LEAVE
from check_pty import APC, BIN


def trial(flags, action=None, source=ROOT / "img-test"):
    start = time.perf_counter()
    metrics = {}
    sent = None
    names_at = None
    images = 0
    command_tail = bytearray()
    data = bytearray()
    with Pager(source, *flags, "--cache-stats", graphics=True,
                 cols=122, rows=40, pixels=(976, 680)) as b:
        while time.perf_counter() - start < 10:
            if select.select([b.master], [], [], 0.001)[0]:
                chunk = os.read(b.master, 65536)
                data.extend(chunk)
                command_tail.extend(chunk)
                now = time.perf_counter()
                if "names_ms" not in metrics and b"Enter name\x1b[0m" in data:
                    metrics["names_ms"] = (now - start) * 1000
                    names_at = now
                consumed = 0
                for command in APC.finditer(command_tail):
                    controls = dict(item.split(b"=", 1) for item in command[1].split(b","))
                    images += controls.get(b"m") == b"0"
                    consumed = command.end()
                if consumed:
                    del command_tail[:consumed]
                if images and "first_preview_ms" not in metrics:
                    metrics["first_preview_ms"] = (now - start) * 1000
                if images == 4 and "all_previews_ms" not in metrics:
                    metrics["all_previews_ms"] = (now - start) * 1000
                if action == b"G" and b"\x1b[7m> z-last" in data and "selection_ms" not in metrics:
                    metrics["selection_ms"] = (now - sent) * 1000
                    os.write(b.master, b"q")
                    sent = time.perf_counter()
                if not action and sent is None and "names_ms" in metrics and (images == 4 or "--no-images" in flags):
                    sent = time.perf_counter()
                    os.write(b.master, b"q")
                if LEAVE in data:
                    metrics["quit_restoration_ms"] = (now - sent) * 1000
                    break
            if action and sent is None and names_at is not None and time.perf_counter() - names_at >= 0.010:
                assert images == 0, "slow JPEG completed before the input trial"
                sent = time.perf_counter()
                os.write(b.master, action)
        else:
            raise TimeoutError("pager trial timed out")
        # Reap directly to obtain this child's CPU/RSS, without timed-wait overhead.
        deadline = time.perf_counter() + 3
        while True:
            pid, status, usage = os.wait4(b.child.pid, os.WNOHANG)
            if pid:
                b.child.returncode = os.waitstatus_to_exitcode(status)
                break
            assert time.perf_counter() < deadline
            time.sleep(0.001)
        assert b.child.returncode == 0
        assert termios.tcgetattr(b.master) == b.before
        stats = b.child.stderr.read().decode().strip()
        assert re.fullmatch(r"lsa: cache: \d+ hits, \d+ misses, \d+ writes, \d+ errors", stats), stats
        assert stats.endswith("0 errors")
        metrics["cpu_ms"] = (usage.ru_utime + usage.ru_stime) * 1000
        metrics["peak_rss_bytes"] = usage.ru_maxrss * (1 if sys.platform == "darwin" else 1024)
        metrics["output_bytes"] = len(data)
        metrics["cache_stats"] = stats
        return metrics


def main():
    local = ROOT / "benchmarks/local/pager"
    local.mkdir(parents=True, exist_ok=True)
    cache = local / "cache"
    cache_flag = f"--cache-dir={cache}"
    slow = local / "slow"
    slow.mkdir(exist_ok=True)
    image = slow / "image.jpg"
    target = ROOT / "img-test/shutterstock_1798373137.jpg"
    if not image.is_symlink():
        image.symlink_to(target)
    assert image.readlink() == target
    (slow / "z-last").touch(exist_ok=True)
    def clear():
        subprocess.run([str(BIN), cache_flag, "--clear-cache"], capture_output=True, check=True)

    report = {
        "date": date.today().isoformat(), "os": platform.platform(), "machine": platform.machine(),
        "binary_sha256": hashlib.sha256(BIN.read_bytes()).hexdigest(), "binary_bytes": BIN.stat().st_size,
        "geometry": "122x40 cells, 8x17 pixels/cell; pager thumbnail 176x85",
        "fixture": "Four unchanged user images plus generated/ in img-test; input trials use a symlink to the original 2924x1932 JPEG plus z-last marker",
        "conditions": "Fresh processes; two warmups then nine trials. OS cache warm, not flushed. Cache clearing/priming outside timings. Clock starts before PTY setup/Popen. No renderer. Incremental command parsing, read timestamps include Python drain overhead. Input sent 10 ms after first names, before JPEG completion. CPU/RSS from wait4 for each child, including worker; exit observed separately.",
        "cases": {},
    }
    for name, flags, action in [
        ("text", ["--no-images"], None),
        ("cache_off", ["--no-cache"], None),
        ("cache_empty", [cache_flag], None),
        ("cache_warm", [cache_flag], None),
        ("select_during_decode", ["--no-cache"], b"G"),
        ("quit_during_decode", ["--no-cache"], b"q"),
    ]:
        if name == "cache_warm":
            clear()
            trial([cache_flag])
        samples = []
        for i in range(11):
            if name == "cache_empty":
                clear()
            result = trial(flags, action, slow if action else ROOT / "img-test")
            if i >= 2:
                samples.append(result)
        fields = set.intersection(*(set(sample) for sample in samples)) - {"cache_stats"}
        report["cases"][name] = {
            "median": {key: round(statistics.median(sample[key] for sample in samples), 3) for key in sorted(fields)},
            "samples": samples,
        }
        print(name, report["cases"][name]["median"], flush=True)
    target = ROOT / "benchmarks/local/pager.json"
    target.write_text(json.dumps(report, indent=2) + "\n")
    print(target)


if __name__ == "__main__":
    main()

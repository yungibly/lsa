#!/usr/bin/env python3
"""Paired application measurements. Fixtures, caches and reports stay in this repo."""
import argparse
from datetime import datetime, timezone
import errno
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import platform
import pty
import select
import statistics
import subprocess
import termios
import time
import struct

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("check_pty", ROOT / "tests/check_pty.py")
protocol = importlib.util.module_from_spec(spec)
spec.loader.exec_module(protocol)


def measure(binary, flags, terminal=False):
    env = os.environ.copy()
    for key in ["NO_COLOR", "LS_COLORS", "TMUX", "STY", "ZELLIJ"]: env.pop(key, None)
    env.update(TERM="xterm-ghostty", TERM_PROGRAM="ghostty", LC_ALL="C")
    master = slave = None
    if terminal:
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 122, 976, 680))
    start = time.perf_counter()
    child = subprocess.Popen([str(binary), *map(str, flags)], cwd=ROOT, env=env,
                             stdin=subprocess.DEVNULL, stdout=slave if terminal else subprocess.PIPE,
                             stderr=subprocess.PIPE)
    if terminal: os.close(slave)
    descriptor = master if terminal else child.stdout.fileno()
    data = bytearray()
    try:
        while True:
            if time.perf_counter() - start > 15: raise TimeoutError("measurement exceeded 15s")
            if not select.select([descriptor], [], [], .05)[0]: continue
            try: chunk = os.read(descriptor, 65536)
            except OSError as error:
                if error.errno != errno.EIO: raise
                break
            if not chunk: break
            data.extend(chunk)
        _, status, usage = os.wait4(child.pid, 0)
        child.returncode = os.waitstatus_to_exitcode(status)
        elapsed = time.perf_counter() - start
        err = child.stderr.read()
        assert child.returncode == 0 and not err, (child.returncode, err)
        return {"elapsed_ms": elapsed * 1000, "cpu_ms": (usage.ru_utime + usage.ru_stime) * 1000,
                "rss_bytes": usage.ru_maxrss * (1 if platform.system() == "Darwin" else 1024),
                "output_bytes": len(data), "images": len(protocol.images(data)), "image_bytes": sum(len(m[0]) for m in protocol.APC.finditer(data)),
                "sha256": hashlib.sha256(data).hexdigest()}
    finally:
        if child.returncode is None: child.kill(); child.wait()
        child.stderr.close()
        if child.stdout: child.stdout.close()
        if master is not None: os.close(master)


def summarize(samples):
    return {"runs": len(samples), **{f"median_{key}": round(statistics.median(s[key] for s in samples), 3)
            for key in ["elapsed_ms", "cpu_ms", "rss_bytes"]},
            **{key: samples[-1][key] for key in ["output_bytes", "image_bytes", "images", "sha256"]}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--before", type=Path)
    parser.add_argument("--runs", type=int, default=11)
    args = parser.parse_args()
    assert 3 <= args.runs <= 51
    local = ROOT / "benchmarks/local/inline"
    text = local / "text-10000"
    text.mkdir(parents=True, exist_ok=True)
    for i in range(10000):
        path = text / f"entry-{i:05}.txt"
        if not path.exists(): path.touch()
    variants = {"after": ROOT / "target/release/lsa"}
    if args.before:
        before = args.before.resolve()
        before.relative_to(ROOT)
        variants = {"before": before, **variants}
    report = {"date": datetime.now(timezone.utc).isoformat(), "os": platform.platform(), "machine": platform.machine(),
              "rust": subprocess.check_output(["rustc", "--version"], text=True).strip(),
              "conditions": "Fresh release processes; OS caches warm, not flushed. 2 warmups, paired/interleaved versions. PTY 122x40, 8x17 pixels/cell, drained without renderer; stdin /dev/null. wait4 includes child CPU/RSS. Empty means thumbnail cache cleared before each timed run. Cache setup excluded.",
              "binaries": {label: {"bytes": binary.stat().st_size, "sha256": hashlib.sha256(binary.read_bytes()).hexdigest()} for label,binary in variants.items()},
              "cases": {}}
    cases = [("text_10000_pipe", False, text), ("text_10000_tty", True, text),
             ("long_10000_pipe", False, text), ("user_images_off", True, ROOT / "img-test"),
             ("user_images_empty", True, ROOT / "img-test"), ("user_images_warm", True, ROOT / "img-test"),
             ("gallery_default", True, ROOT / "img-test/generated/many"),
             ("long_images_off", True, ROOT / "img-test"), ("long_images_empty", True, ROOT / "img-test"), ("long_images_warm", True, ROOT / "img-test")]
    for case, terminal, path in cases:
        samples = {label: [] for label in variants}
        for i in range(args.runs + 2):
            for label, binary in variants.items():
                flags = []
                if case == "long_10000_pipe" or case.startswith("long_images"): flags = ["-l"]
                if case.startswith(("user_images", "long_images")):
                    cache = local / f"cache-{label}"
                    if not case.endswith("off"):
                        flag = f"--cache-dir={cache}"
                        if case.endswith("empty") or i == 0:
                            subprocess.run([str(binary), flag, "--clear-cache"], capture_output=True, check=True)
                        flags += [flag]
                        if case.endswith("warm") and i == 0: measure(binary, [*flags, path], terminal)
                sample = measure(binary, [*flags, path], terminal)
                if i >= 2: samples[label].append(sample)
        report["cases"][case] = {label: summarize(runs) for label, runs in samples.items()}
        print(case, report["cases"][case], flush=True)
    if "before" in variants:
        plain = report["cases"]["text_10000_pipe"]
        assert plain["before"]["sha256"] == plain["after"]["sha256"]
    for label in variants:
        for family in ["user_images", "long_images"]:
            images = [report["cases"][f"{family}_{mode}"][label] for mode in ["off", "empty", "warm"]]
            assert len({record["sha256"] for record in images}) == 1
    destination = ROOT / "benchmarks/local/inline.json"
    destination.write_text(json.dumps(report, indent=2) + "\n")
    print(destination)


if __name__ == "__main__": main()

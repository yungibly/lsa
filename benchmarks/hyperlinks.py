#!/usr/bin/env python3
"""Paired hyperlink costs with plain-pipe and complete visible-output controls."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import subprocess

from inline import ROOT, measure, summarize


OSC8 = re.compile(rb"\x1b\]8;;([^\x1b\x07]*)(?:\x1b\\|\x07)")


def prepare_fixtures():
    base = ROOT / "target/hyperlink-benchmark"
    paths = {"short": base / "short-10000", "deep": base / "deep-10000"}
    for i in range(6):
        paths["deep"] /= f"project-{i:02}-" + "nested-workspace-" * 3
    for directory in paths.values():
        directory.mkdir(parents=True, exist_ok=True)
        for i in range(10000):
            name = f"entry-{i:05}"
            if i % 4 == 0:
                name += " photo #100% résumé"
            path = directory / f"{name}.txt"
            if not path.exists():
                path.touch()
            os.utime(path, (1_700_000_000, 1_700_000_000))
    return paths


def output(binary, flags, directory):
    env = os.environ.copy()
    for key in ["NO_COLOR", "LS_COLORS", "TMUX", "STY", "ZELLIJ"]:
        env.pop(key, None)
    env.update(TERM="xterm-ghostty", TERM_PROGRAM="ghostty", LC_ALL="C")
    return subprocess.check_output(
        [str(binary), *flags, str(directory)], cwd=ROOT, env=env,
        stdin=subprocess.DEVNULL, stderr=subprocess.PIPE,
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--before", type=Path, required=True)
    parser.add_argument("--runs", type=int, default=7)
    args = parser.parse_args()
    assert 3 <= args.runs <= 51
    before = args.before.resolve()
    before.relative_to(ROOT)
    after = ROOT / "target/release/lsa"
    binaries = {"before": before, "after": after}
    paths = prepare_fixtures()
    report = {
        "date": datetime.now(timezone.utc).isoformat(),
        "os": platform.platform(),
        "machine": platform.machine(),
        "rust": subprocess.check_output(["rustc", "--version"], text=True).strip(),
        "timezone": os.environ.get("TZ", "(host local timezone)"),
        "conditions": f"Two warmups then {args.runs} paired/interleaved fresh release processes; "
                      "alternating order per pair. OS caches warm, not flushed. "
                      "stdout is a drained pipe without renderer; stdin /dev/null. "
                      "wait4 child CPU/RSS. Preparation and output parsing excluded from timing. "
                      "No concurrent builds/tests. 10,000 files per fixture, repeated timestamp; "
                      "one quarter have spaces, #, %, and non-ASCII names. "
                      "Styled long cases force color/icons and disable images. "
                      "Forced-link versions differ in URI hostname bytes; every case asserts "
                      "identical complete output after removing OSC 8 framing. "
                      "Plain before/after output must also match byte-for-byte.",
        "binaries": {
            label: {
                "version": subprocess.check_output([str(path), "--version"], text=True).strip(),
                "bytes": path.stat().st_size,
                "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            } for label, path in binaries.items()
        },
        "fixtures": {
            label: {"path": str(path.relative_to(ROOT)),
                    "absolute_directory_bytes": len(os.fsencode(path)), "entries": 10000}
            for label, path in paths.items()
        },
        "cases": {},
    }
    styled = ["-l", "--color=always", "--icons=always", "--no-images"]
    for depth, directory in paths.items():
        cases = [
            (f"plain_{depth}_pipe", {
                "before": (before, []), "after": (after, []),
            }),
            (f"forced_links_{depth}_pipe", {
                "before": (before, [*styled, "--hyperlink"]),
                "after": (after, [*styled, "--hyperlink=always"]),
            }),
            (f"link_overhead_{depth}_pipe", {
                "off": (after, [*styled, "--hyperlink=never"]),
                "on": (after, [*styled, "--hyperlink=always"]),
            }),
        ]
        for case, variants in cases:
            expected = {}
            visible_hashes = set()
            details = {}
            for label, (binary, flags) in variants.items():
                data = output(binary, flags, directory)
                expected[label] = hashlib.sha256(data).hexdigest()
                visible_hashes.add(hashlib.sha256(OSC8.sub(b"", data)).hexdigest())
                links = sum(bool(match[1]) for match in OSC8.finditer(data))
                assert links == (10000 if label == "on" or case.startswith("forced_links") else 0)
                details[label] = {"flags": flags, "links": links}
            assert len(visible_hashes) == 1, (case, "visible output differs")
            if case.startswith("plain"):
                assert len(set(expected.values())) == 1, (case, "plain output differs")
            samples = {label: [] for label in variants}
            for i in range(args.runs + 2):
                order = list(variants) if i % 2 == 0 else list(reversed(variants))
                for label in order:
                    binary, flags = variants[label]
                    sample = measure(binary, [*flags, directory])
                    assert sample["sha256"] == expected[label], (case, label, "unstable output")
                    if i >= 2:
                        samples[label].append(sample)
            result = {
                "equal_output": len(set(expected.values())) == 1,
                "equal_visible_output": True,
                "visible_output_sha256": next(iter(visible_hashes)),
                **{label: {**details[label], **summarize(values)}
                   for label, values in samples.items()},
            }
            report["cases"][case] = result
            print(case, json.dumps(result), flush=True)
    destination = ROOT / "benchmarks/local/hyperlinks.json"
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(json.dumps(report, indent=2) + "\n")
    print(destination)


if __name__ == "__main__":
    main()

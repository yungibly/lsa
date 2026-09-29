#!/usr/bin/env python3
"""Paired v0.5.0 preview and listing measurements against a saved binary.

Galleries use the user's sample photo (read only, via symlinks), a HEIC copy
made with sips on macOS, and generated folders. Fixtures and the report stay
under benchmarks/local/. Two warmups, then paired runs in alternating order.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
from inline import ROOT, measure, summarize

PHOTO = ROOT / "img-test/shutterstock_1798373137.jpg"


def fixtures(local):
    photos, heic, folders, text = (local / name for name in ["photos-64", "heic-64", "folders-256", "files-10000"])
    if not photos.exists():
        photos.mkdir(parents=True)
        for i in range(64):
            (photos / f"photo-{i:02}.jpg").symlink_to(PHOTO)
    if platform.system() == "Darwin" and not heic.exists():
        source = local / "photo.heic"
        subprocess.run(["sips", "-s", "format", "heic", str(PHOTO), "--out", str(source)], check=True, capture_output=True)
        heic.mkdir()
        for i in range(64):
            (heic / f"photo-{i:02}.heic").symlink_to(source)
    if not folders.exists():
        for i in range(256):
            (folders / f"folder-{i:03}").mkdir(parents=True)
    if not text.exists():
        text.mkdir()
        for i in range(10000):
            path = text / f"entry-{i:05}.txt"
            path.touch()
            os.utime(path, (1_700_000_000, 1_700_000_000))
    return photos, heic, folders, text


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--before", type=Path, required=True)
    parser.add_argument("--runs", type=int, default=7)
    args = parser.parse_args()
    assert 3 <= args.runs <= 51
    before = args.before.resolve()
    before.relative_to(ROOT)
    variants = {"before": before, "after": ROOT / "target/release/lsa"}
    local = ROOT / "benchmarks/local/previews"
    photos, heic, folders, text = fixtures(local)
    # (name, terminal, flags, equal output expected)
    cases = [
        ("plain_names_pipe_10000", False, [text], True),
        ("long_pipe_10000", False, ["-l", text], True),
        ("long_tty_10000", True, ["-l", text], False),
        ("jpeg_grid_64", True, ["--grid", "--preview-limit=64", photos], False),
        ("jpeg_long_miniatures_64", True, ["-l", "--preview-limit=64", photos], False),
        ("folder_grid_256", True, ["--grid", folders], False),
        ("sample_images_default", True, [ROOT / "img-test"], False),
    ]
    if heic.exists():
        cases.append(("heic_grid_64", True, ["--grid", "--preview-limit=64", heic], False))
    report = {
        "date": datetime.now(timezone.utc).isoformat(),
        "os": platform.platform(),
        "machine": platform.machine(),
        "conditions": f"Two warmups, then {args.runs} paired/interleaved fresh processes per case. "
                      "OS caches warm. 122x40 PTY, 8x17 cell pixels, drained without a renderer; "
                      "stdin /dev/null. wait4 child CPU and peak RSS; elapsed ends at child exit. "
                      "Thumbnail cache off. No concurrent builds/tests.",
        "binaries": {label: {"bytes": path.stat().st_size,
                             "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
                     for label, path in variants.items()},
        "cases": {},
    }
    for case, terminal, flags, equal in cases:
        samples = {label: [] for label in variants}
        for i in range(args.runs + 2):
            order = list(variants) if i % 2 == 0 else list(reversed(variants))
            for label in order:
                sample = measure(variants[label], flags, terminal)
                if i >= 2:
                    samples[label].append(sample)
        for values in samples.values():
            assert len({v["sha256"] for v in values}) == 1, case
        result = {label: summarize(values) for label, values in samples.items()}
        if equal:
            assert result["before"]["sha256"] == result["after"]["sha256"], case
        report["cases"][case] = {"equal_output": equal, **result}
        b, a = result["before"], result["after"]
        print(f"{case:26} {b['median_elapsed_ms']:9.2f} -> {a['median_elapsed_ms']:8.2f} ms   "
              f"cpu {b['median_cpu_ms']:8.2f} -> {a['median_cpu_ms']:8.2f} ms   "
              f"rss {b['median_rss_bytes'] / 2**20:6.1f} -> {a['median_rss_bytes'] / 2**20:5.1f} MiB   "
              f"images {b['images']} -> {a['images']}   bytes {b['output_bytes']:,} -> {a['output_bytes']:,}",
              flush=True)
    destination = ROOT / "benchmarks/local/previews.json"
    destination.write_text(json.dumps(report, indent=2) + "\n")
    print(destination)


if __name__ == "__main__":
    main()

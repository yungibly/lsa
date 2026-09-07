#!/usr/bin/env python3
"""Paired equal-work galleries and rotated JPEGs, drained PTY, no renderer."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import platform
import shutil
import subprocess
from inline import ROOT, measure, summarize


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--before', type=Path, required=True)
    parser.add_argument('--runs', type=int, default=7)
    args = parser.parse_args()
    assert 3 <= args.runs <= 51
    before = args.before.resolve(); before.relative_to(ROOT)
    variants = {'before': before, 'after': ROOT / 'target/release/lsa'}
    local = ROOT / 'benchmarks/local/gallery'; local.mkdir(parents=True, exist_ok=True)
    large = local / '256'; large.mkdir(exist_ok=True)
    source = ROOT / 'img-test/generated/landscape.png'
    for i in range(256):
        path = large / f'image-{i:03}.png'
        if not path.exists(): shutil.copyfile(source, path)  # distinct source identities
    fixtures = ROOT / 'target/visual-checks/gallery-files'
    assert fixtures.exists(), 'Run target/release/examples/gallery_fixtures first'
    report = {'date': datetime.now(timezone.utc).isoformat(), 'os': platform.platform(), 'machine': platform.machine(),
              'conditions': 'Fresh release processes; two warmups then interleaved before/after. OS caches warm, not flushed. 122x40 PTY, 8x17 pixels/cell, no renderer, stdin /dev/null. Child CPU/RSS via wait4. 256 distinct copies of one synthetic 320x160 PNG. EXIF case: one generated 3200x2000 JPEG, orientation 6. Cache clearing/priming excluded; output parsing excluded from elapsed time.',
              'binaries': {label: {'bytes': path.stat().st_size, 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()} for label, path in variants.items()},
              'cases': {}}
    cases = {'gallery_256_equal_work': ['--preview-limit=256', large],
             'rotated_jpeg_off': [fixtures / 'rotated-photo.jpg'],
             'rotated_jpeg_empty': [fixtures / 'rotated-photo.jpg'],
             'rotated_jpeg_warm': [fixtures / 'rotated-photo.jpg'],
             'rotated_jpeg_long': ['-l', fixtures / 'rotated-photo.jpg']}
    for case, flags in cases.items():
        samples = {label: [] for label in variants}
        for i in range(args.runs + 2):
            for label, binary in variants.items():
                selected = list(flags)
                if case.endswith(('empty', 'warm')):
                    cache_flag = f'--cache-dir={local / ("cache-" + label)}'
                    if case.endswith('empty') or i == 0:
                        subprocess.run([str(binary), cache_flag, '--clear-cache'], capture_output=True, check=True)
                    selected.insert(0, cache_flag)
                    if case.endswith('warm') and i == 0: measure(binary, selected, True)
                sample = measure(binary, selected, True)
                if i >= 2: samples[label].append(sample)
        report['cases'][case] = {label: summarize(values) for label, values in samples.items()}
        print(case, report['cases'][case], flush=True)
    paired = report['cases']['gallery_256_equal_work']
    assert paired['before']['sha256'] == paired['after']['sha256']
    assert paired['after']['images'] == 256
    for label in variants:
        assert len({report['cases'][f'rotated_jpeg_{mode}'][label]['sha256'] for mode in ['off', 'empty', 'warm']}) == 1
    destination = ROOT / 'benchmarks/local/gallery.json'
    destination.write_text(json.dumps(report, indent=2) + '\n')
    print(destination)


if __name__ == '__main__': main()

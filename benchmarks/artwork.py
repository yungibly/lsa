#!/usr/bin/env python3
"""Equal-output measurements of the final built-in artwork rasterization pass."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
from inline import ROOT, measure, summarize


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--before', type=Path, required=True)
    parser.add_argument('--runs', type=int, default=7)
    args = parser.parse_args()
    assert 3 <= args.runs <= 51
    before = args.before.resolve(); before.relative_to(ROOT)
    variants = {'before': before, 'after': ROOT / 'target/release/lsa'}
    local = ROOT / 'benchmarks/local/artwork'; local.mkdir(parents=True, exist_ok=True)
    folders, mixed, raster = [local / name for name in ['folders', 'mixed', 'raster']]
    for directory in [folders, mixed, raster]: directory.mkdir(exist_ok=True)
    types = ['folder', 'file.weird', 'picture.HEIC', 'video.webm', 'audio.mp3', 'archive.zip',
             'code.rs', 'config.json', 'link', 'pipe', 'broken.png']
    for i in range(256):
        (folders / f'folder-{i:03}').mkdir(exist_ok=True)
        path = mixed / f'{i:03}-{types[i % len(types)]}'
        if not path.exists() and not path.is_symlink():
            if i % len(types) == 0: path.mkdir()
            elif i % len(types) == 8: path.symlink_to('001-file.weird')
            elif i % len(types) == 9: os.mkfifo(path)
            else: path.touch()
        photo = raster / f'photo-{i:03}.png'
        if not photo.exists(): photo.symlink_to(ROOT / 'img-test/generated/landscape.png')
    report = {'date': datetime.now(timezone.utc).isoformat(), 'os': platform.platform(),
              'conditions': f'Two warmups then {args.runs} paired/interleaved fresh release processes, alternating version order. OS caches warm. No thumbnail cache. 122x40 PTY with 8x17 cell pixels, drained without renderer; stdin /dev/null; wait4 child CPU/RSS. Build/tests finished before timing. Mixed rotates through all eleven artwork categories. Raster control links one synthetic 320x160 PNG. Every comparison asserts identical complete output SHA-256.',
              'binaries': {label: {'bytes': path.stat().st_size, 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()} for label, path in variants.items()},
              'cases': {}}
    for case, flags in {'folders_256': ['--grid', folders], 'mixed_artwork_256': ['--grid', mixed],
                        'raster_256': ['--grid', raster]}.items():
        samples = {label: [] for label in variants}
        for i in range(args.runs + 2):
            order = list(variants) if i % 2 == 0 else list(reversed(variants))
            for label in order:
                sample = measure(variants[label], flags, True)
                if i >= 2: samples[label].append(sample)
        result = {label: summarize(values) for label, values in samples.items()}
        assert all(len({sample['sha256'] for sample in values}) == 1
                   for values in samples.values())
        assert result['before']['sha256'] == result['after']['sha256']
        report['cases'][case] = result
        print(case, result, flush=True)
    destination = ROOT / 'benchmarks/local/artwork.json'
    destination.write_text(json.dumps(report, indent=2) + '\n')
    print(destination)


if __name__ == '__main__': main()

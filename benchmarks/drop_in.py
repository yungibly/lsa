#!/usr/bin/env python3
"""Measure the practical ls compatibility pass against the previous release."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
import platform
from pathlib import Path
from inline import ROOT, measure, summarize


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--before', type=Path, required=True)
    parser.add_argument('--runs', type=int, default=7)
    args = parser.parse_args()
    assert 3 <= args.runs <= 51
    before = args.before.resolve()
    before.relative_to(ROOT)
    binaries = {'before': before, 'after': ROOT / 'target/release/lsa'}
    fixture = ROOT / 'benchmarks/local/drop-in'
    text = fixture / 'text-10000'
    tree = fixture / 'tree'
    text.mkdir(parents=True, exist_ok=True)
    for i in range(10000):
        path = text / f'entry-{i:05}.txt'
        path.touch(exist_ok=True)
        os.utime(path, (1700000000, 1700000000))
    for d in range(50):
        directory = tree / f'dir-{d:03}'
        directory.mkdir(parents=True, exist_ok=True)
        for i in range(100):
            (directory / f'file-{i:03}').touch(exist_ok=True)
    report = {
        'date': datetime.now(timezone.utc).isoformat(),
        'os': platform.platform(),
        'conditions': 'Two warmups followed by paired/interleaved fresh release processes. '
                      'OS caches warm, no thumbnail disk cache. Fixture setup excluded. '
                      '122x40 PTY with 8x17 pixel cells, drained without renderer. '
                      'wait4 measures application CPU/RSS; terminal rendering excluded. '
                      'Explicit -l adds links/group, totals and attribute indicators, so '
                      'its outputs intentionally differ. Recursion is after-only.',
        'binaries': {label: {'bytes': binary.stat().st_size,
                            'sha256': hashlib.sha256(binary.read_bytes()).hexdigest()}
                     for label, binary in binaries.items()},
        'cases': {},
    }
    cases = [
        ('plain_pipe_10000', False, [text], True),
        ('columns_tty_10000', True, ['-C', text], True),
        ('automatic_details_tty_10000', True, ['--no-images', text], True),
        ('explicit_long_pipe_10000', False, ['-l', text], False),
        ('gallery_tty', True, [ROOT / 'img-test/generated/many'], True),
        ('recursive_pipe_5050', False, ['-R', tree], False),
    ]
    for name, terminal, flags, equal in cases:
        variants = {'after': binaries['after']} if name.startswith('recursive') else binaries
        samples = {label: [] for label in variants}
        for i in range(args.runs + 2):
            order = list(variants) if i % 2 == 0 else list(reversed(variants))
            for label in order:
                sample = measure(variants[label], flags, terminal)
                if i >= 2:
                    samples[label].append(sample)
        result = {label: summarize(values) for label, values in samples.items()}
        for values in samples.values():
            assert len({v['sha256'] for v in values}) == 1, name
        if equal:
            assert result['before']['sha256'] == result['after']['sha256'], name
        report['cases'][name] = {'equal_output': equal, **result}
        print(name, result, flush=True)
    destination = ROOT / 'benchmarks/local/drop-in.json'
    destination.write_text(json.dumps(report, indent=2) + '\n')
    print(destination)


if __name__ == '__main__':
    main()

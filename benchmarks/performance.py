#!/usr/bin/env python3
"""Paired listing CPU/RSS measurements, with explicit equal-output controls."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
from inline import ROOT, measure, summarize


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--before', type=Path, required=True)
    parser.add_argument('--runs', type=int, default=7)
    args = parser.parse_args()
    assert 3 <= args.runs <= 51
    before = args.before.resolve()
    before.relative_to(ROOT)
    variants = {'before': before, 'after': ROOT / 'target/release/lsa'}
    local = ROOT / 'benchmarks/local/performance'
    repeated, distinct = [local / name for name in ['repeated-10000', 'distinct-10000']]
    for directory in [repeated, distinct]:
        directory.mkdir(parents=True, exist_ok=True)
        for i in range(10000):
            path = directory / f'entry-{i:05}.txt'
            if not path.exists():
                path.touch()
            # The distinct case defeats the bounded exact-second cache, as do
            # ordinary directories containing more than 64 different times.
            seconds = 1_700_000_000 + (i * 64 if directory == distinct else 0)
            os.utime(path, (seconds, seconds))
    report = {
        'date': datetime.now(timezone.utc).isoformat(),
        'os': platform.platform(),
        'rust': subprocess.check_output(['rustc', '--version'], text=True).strip(),
        'timezone': os.environ.get('TZ', '(host local timezone)'),
        'conditions': f'Two warmups, then {args.runs} paired/interleaved fresh release processes. '
                      'OS caches warm, not flushed. 122x40 PTY, 8x17 cell pixels, drained '
                      'without renderer; stdin /dev/null. wait4 child CPU/RSS. '
                      'Fixture setup excluded. No concurrent builds/tests. '
                      'All cases except changed_default_tty assert identical output.',
        'binaries': {label: {'bytes': path.stat().st_size,
                            'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}
                     for label, path in variants.items()},
        'cases': {},
    }
    cases = [
        ('plain_pipe', False, repeated, [], [], True),
        ('compact_tty', True, repeated, [], ['-C'], True),
        ('long_repeated_pipe', False, repeated, ['-l'], ['-l'], True),
        ('long_distinct_pipe', False, distinct, ['-l'], ['-l'], True),
        ('numeric_without_time_pipe', False, distinct,
         ['--fields=mode,links,uid,gid,size'], ['--fields=mode,links,uid,gid,size'], True),
        ('long_repeated_tty', True, repeated, ['-l'], [], True),
        ('long_distinct_tty', True, distinct, ['-l'], [], True),
        ('changed_default_tty', True, repeated, [], [], False),
    ]
    for case, terminal, directory, old, new, equal in cases:
        samples = {label: [] for label in variants}
        for i in range(args.runs + 2):
            order = list(variants) if i % 2 == 0 else list(reversed(variants))
            for label in order:
                flags = old if label == 'before' else new
                sample = measure(variants[label], [*flags, directory], terminal)
                if i >= 2:
                    samples[label].append(sample)
        for values in samples.values():
            assert len({v['sha256'] for v in values}) == 1
        result = {label: summarize(values) for label, values in samples.items()}
        if equal:
            assert result['before']['sha256'] == result['after']['sha256'], case
        report['cases'][case] = {'equal_output': equal, **result}
        print(case, result, flush=True)
    destination = ROOT / 'benchmarks/local/performance.json'
    destination.write_text(json.dumps(report, indent=2) + '\n')
    print(destination)


if __name__ == '__main__':
    main()

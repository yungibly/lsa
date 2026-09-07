#!/usr/bin/env python3
"""Larger galleries, adjustable frames, option notices and vector previews."""
from pathlib import Path
import os
import struct
import subprocess
import tempfile
import zlib
from check_pty import APC, BIN, ROOT, capture, check_cursor, images
from check_layout import plain, run
from check_thumbnails import replay


def png():
    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
    return (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', 16, 16, 8, 6, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress((b'\0' + bytes([255, 0, 0, 255]) * 16) * 16))
            + chunk(b'IEND', b''))


def main():
    cases = 0
    with tempfile.TemporaryDirectory(dir=ROOT / 'target', prefix='gallery-') as temp:
        root = Path(temp)
        source = root / 'source.png'; source.write_bytes(png())
        many = root / 'many'; many.mkdir()
        for i in range(300): (many / f'image-{i:03}.png').symlink_to(source)
        cache = root / 'cache'
        code, data, err, _ = capture([f'--cache-dir={cache}', '--cache-stats', many], cols=122)
        assert code == 0 and b'255 hits, 1 misses, 1 writes, 0 errors' in err, err
        assert len(images(data)) == 260  # 256 previews plus four tiles finishing a row
        for i in range(300): assert plain(data).count(f'image-{i:03}.png@'.encode()) == 1
        cases += 1
        for flags in [['--preview-limit', '300'], ['--preview-limit=4096']]:
            data = run([*flags, many], cols=122)
            assert len(images(data)) == 300
            assert check_cursor(data, 122, 24, 23) == 300
            cases += 1
        data = run(['--preview-limit=2', many, many], cols=122)
        assert len(images(data)) == 5
        for i in range(300): assert plain(data).count(f'image-{i:03}.png@'.encode()) == 2
        cases += 1
        data = run(['--fields=size', many], cols=122)
        assert len(images(data)) == 256 and len(replay(data)) == 300
        offsets = [line.index(f'image-{i:03}.png@') for i, line in enumerate(replay(data))]
        assert len(set(offsets)) == 1
        cases += 1

        # Exact row increments, adaptive spacing, clamping and full label wraps.
        small = root / 'small'; small.mkdir()
        for i in range(7): (small / f'{i}-long-filename-spanning-several-cells.png').symlink_to(source)
        for cols, rows, cell in [(122, 40, (8, 17)), (80, 8, (8, 16)), (12, 6, (8, 16)), (200, 24, (32, 80))]:
            for size in range(1, 13):
                geometry = dict(cols=cols, rows=rows, pixels=(cols * cell[0], rows * cell[1]))
                data = run(['--grid', '--thumbnail-size', str(size), small], **geometry)
                frames = images(data)
                assert len(frames) == 7
                assert all(int(c[b'r']) == min(size, rows - 3) for c in frames)
                assert all(int(c[b's']) <= 320 and int(c[b'v']) <= 240 for c in frames)
                assert check_cursor(data, cols, rows, rows - 1) == 7
                # Remove layout spaces/newlines to reassemble complete wrapped names.
                text = b''.join(plain(data).split())
                if cols == 12:
                    for i in range(7): assert text.count(f'{i}-long-filename-spanning-several-cells.png@'.encode()) == 1
                cases += 1
        for selector in ['--header', '--fields=size', '-l', '-n', '-1']:
            for flags in [[selector, '--grid'], ['--grid', selector]]:
                code, data, err, _ = capture([*flags, source, source], cols=122)
                assert code == 0 and err.startswith(b'lsa: --grid ignored:') and err.count(b'\n') == 1, err
                assert b'long' in err if selector != '-1' else b'text lines' in err
                assert len(images(data)) == (0 if selector == '-1' else 2)
                cases += 1
        code, data, err, _ = capture(['--header', '--thumbnail-size=7', source], cols=122)
        assert code == 0 and b'--thumbnail-size ignored:' in err
        assert all(c[b'r'] == b'1' for c in images(data))
        cases += 1

        formats = root / 'formats'; formats.mkdir()
        svg = formats / 'drawing.SVG'
        svg.write_text('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 40 20"><rect width="40" height="20" fill="red"/></svg>')
        (formats / 'linked.svg').symlink_to(svg)
        ico = formats / 'icon.ico'
        payload = png()
        ico.write_bytes(struct.pack('<HHH', 0, 1, 1) + struct.pack('<BBBBHHII', 16, 16, 0, 0, 1, 32, len(payload), 22) + payload)
        data = run([formats], cols=122)
        frames = images(data, pixels=True)
        assert len(frames) == 3
        for c, pixels in frames:
            index = ((int(c[b'v']) // 2) * int(c[b's']) + int(c[b's']) // 2) * 4
            assert pixels[index:index + 4] == bytes([255, 0, 0, 255])
        cases += 1
        for size in [1, 3, 7, 12]:
            flags = [f'--cache-dir={root / "format-cache"}', f'--thumbnail-size={size}', formats]
            cold = run(flags, cols=122)
            warm = run(flags, cols=122)
            assert cold == warm == run([f'--thumbnail-size={size}', formats], cols=122)
            cases += 1
        # Missing resources and unsupported complex SVGs are whole-preview
        # failures, never partial drawings or access to an external FIFO.
        fifo = root / 'external.png'; os.mkfifo(fifo)
        bad = root / 'bad'; bad.mkdir()
        for name, body in [('external', f'<image href="{fifo}"/>'), ('font', '<text>hello</text>'), ('filter', '<filter/>'), ('broken', '')]:
            (bad / f'{name}.svg').write_text(f'<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20">{body}</svg>' if body else 'bad xml')
        (bad / 'oversized.svg').write_bytes(b' ' * (256 * 1024 + 1))
        data = run([bad], cols=122)
        assert len(images(data)) == 5 and len({p for c, p in images(data, pixels=True)}) == 1
        for path in bad.iterdir(): assert plain(data).count(path.name.encode()) == 1
        cases += 1
        for flags in [['-1'], ['--no-images'], ['--preview-limit=0'], ['--diagnose']]:
            untouched = root / ('unused-' + flags[0].replace('-', ''))
            data = run([f'--cache-dir={untouched}', *flags, bad])
            assert not APC.search(data) and not untouched.exists()
            cases += 1
        piped = subprocess.run([str(BIN), '--grid', '--thumbnail-size=12', str(formats)], capture_output=True, check=True)
        assert not piped.stderr and b'\x1b' not in piped.stdout and len(piped.stdout.splitlines()) == 3
        cases += 1
    print(f'{cases} gallery checks passed; large defaults, granular sizes, notices, SVG/ICO, cache and fallbacks. Ghostty appearance remains unverified.')


if __name__ == '__main__': main()

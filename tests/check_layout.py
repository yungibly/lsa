#!/usr/bin/env python3
"""Real executable through a PTY; no renderer or terminal queries."""
import os
from pathlib import Path
import subprocess
import tempfile

from check_pty import APC, CSI, BIN, ROOT, capture, check_cursor, images


def run(args, **kwargs):
    code, data, err, _ = capture(args, **kwargs)
    assert code == 0 and not err, err
    return data


def main():
    cases = 0
    with tempfile.TemporaryDirectory(dir=ROOT / "target", prefix="layouts-") as temp:
        root = Path(temp)
        text = root / "text"
        text.mkdir()
        for name in ["a", "bb", "ccc", "d", "ee"]:
            (text / name).touch()
        for width, expected in [(10, b"a    bb\nccc  d\nee\n"), (11, b"a  bb  ccc\nd  ee\n")]:
            data = run([text], cols=width)
            assert data.replace(b"\r", b"") == expected
            assert check_cursor(data, width, 24, 23) == 0
            cases += 1
        assert run(["-r", text], cols=11).split() == [b"ee", b"d", b"ccc", b"bb", b"a"]
        cases += 1
        for args in [["-1"], ["--grid", "-1"], ["-1", "--grid"]]:
            assert run([*args, text]).replace(b"\r", b"") == b"a\nbb\nccc\nd\nee\n"
            cases += 1

        mixed = root / "mixed"
        mixed.mkdir()
        image = ROOT / "img-test/generated/landscape.png"
        for i in range(4):
            (mixed / f"image-{i}.png").symlink_to(image)
            (mixed / f"text-{i}").touch()
        geometry = dict(cols=122, rows=40, pixels=(976, 680))
        auto = run([mixed], **geometry)
        assert len(images(auto)) == 4
        assert auto == run(["--grid", mixed], **geometry)
        cases += 1
        # Below half after filtering: keep every entry, with no preview payloads.
        (mixed / ".extra").touch()
        assert b"\x1b" not in run(["-a", mixed], **geometry)
        cases += 1
        expected = run(["--no-images", mixed], **geometry)
        assert b"\x1b" not in expected and len(expected.splitlines()) < 8
        for flag in ["--no-images", "--protocol=none"]:
            for args in [["--grid", flag], [flag, "--grid"]]:
                assert run([*args, mixed], **geometry) == expected
                cases += 1
        for limit in [0, 3]:
            assert run([f"--preview-limit={limit}", mixed], **geometry) == expected
            cases += 1
        diagnostic = run(["--diagnose", mixed], **geometry)
        assert b"layout=grid" in diagnostic and b"preview_candidates=4" in diagnostic
        assert b"\x1b" not in diagnostic
        cases += 1
        for args in [[], ["--grid", "--protocol=kitty"], ["--no-images"]]:
            piped = subprocess.run([str(BIN), *args, str(mixed)], capture_output=True, check=True)
            assert piped.stdout == b"image-0.png@\nimage-1.png@\nimage-2.png@\nimage-3.png@\ntext-0\ntext-1\ntext-2\ntext-3\n"
            assert not piped.stderr
            cases += 1

        for i in range(4):
            (mixed / f"text-{i}").unlink()
        # Four images occupy exactly 14 rows at 80 columns, plus two prompt rows.
        assert len(images(run([mixed], rows=16))) == 4
        assert b"\x1b" not in run([mixed], rows=15)
        cases += 1
        (mixed / "image-0.png").rename(mixed / "a-name-that-wraps-over-two-lines.png")
        assert b"\x1b" not in run([mixed], rows=16)
        assert len(images(run([mixed], rows=17))) == 4
        cases += 1
        # Selection doesn't decode or resolve target types: diagnose a FIFO link.
        os.mkfifo(root / "pipe")
        (mixed / "image-1.png").unlink()
        (mixed / "image-1.png").symlink_to(root / "pipe")
        diagnostic = run(["--diagnose", mixed], **geometry)
        assert b"layout=grid" in diagnostic and b"preview_candidates=4" in diagnostic
        assert b"\x1b" not in diagnostic
        cases += 1

        groups = root / "groups"
        groups.mkdir()
        for name in ["b-dir", "z-dir"]:
            (groups / name).mkdir()
        for name in ["a.png", "c.png", "d.png", "y.png"]:
            (groups / name).symlink_to(image)
        (groups / "x.txt").touch()
        (groups / "m-link").symlink_to("z-dir")
        forward = ["b-dir/", "z-dir/", "a.png@", "c.png@", "d.png@", "m-link@", "x.txt", "y.png@"]
        reverse = ["z-dir/", "b-dir/", "y.png@", "x.txt", "m-link@", "d.png@", "c.png@", "a.png@"]
        for args in [["-1"], ["--no-images"], [], ["--grid"]]:
            for order, extra in [(forward, []), (reverse, ["-r"])]:
                data = run(["--dirs-first", *args, *extra, groups], **geometry)
                plain = CSI.sub(b"", APC.sub(b"", data)).decode()
                positions = [plain.index(name) for name in order]
                assert positions == sorted(positions), plain
                assert all(plain.count(name) == 1 for name in order)
                assert len(images(data)) == (0 if args in [["-1"], ["--no-images"]] else 4)
                cases += 1
        for args in [["--fields=size,modified", "--grid"], ["--grid", "--fields=size,modified"], ["--long"], ["-l"]]:
            data = run(["--dirs-first", *args, groups], **geometry)
            assert b"\x1b" not in data and len(data.splitlines()) == 8
            assert b"layout=long" in run(["--diagnose", *args, groups], **geometry)
            cases += 1

    many = ROOT / "img-test/generated/many"
    data = run([many], cols=122, rows=40)
    assert not APC.search(data) and len(data.split()) == 40
    assert len(data.splitlines()) < 40
    assert b"layout_reason=grid exceeds one screen" in run(["--diagnose", many], cols=122, rows=40)
    cases += 1
    print(f"{cases} layout checks passed: default columns/grid, filtering, width/height, overrides, diagnostics, and pipes.")


if __name__ == "__main__":
    main()

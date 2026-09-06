#!/usr/bin/env python3
"""Inline defaults and styling against the real binary, with no terminal renderer."""
import os
from pathlib import Path
import re
import subprocess
import tempfile
from check_pty import APC, CSI, BIN, ROOT, capture, check_cursor, images

SGR = re.compile(rb"\x1b\[[0-9;]*m")
OSC = re.compile(rb"\x1b\]8;;[^\x1b]*\x1b\\")


def plain(data):
    return CSI.sub(b"", SGR.sub(b"", OSC.sub(b"", APC.sub(b"", data)))).replace(b"\r", b"")


def run(args, **kwargs):
    code, data, err, _ = capture(args, **kwargs)
    assert code == 0 and not err, (code, err)
    assert not any(command in data for command in [b"\x1b[?1049", b"\x1b[?25", b"\x1b[2J"]), "interactive terminal lifecycle returned"
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
            assert plain(data) == expected, data
            assert check_cursor(data, width, 24, 23) == 0
            cases += 1
        for args in [["-1"], ["--grid", "-1"], ["-1", "--grid"]]:
            assert plain(run([*args, text])) == b"a\nbb\nccc\nd\nee\n"
            cases += 1

        image = ROOT / "img-test/generated/landscape.png"
        mixed = root / "mixed"
        mixed.mkdir()
        for i in range(4):
            (mixed / f"image-{i}.png").symlink_to(image)
            (mixed / f"text-{i}").touch()
        geometry = dict(cols=122, rows=40, pixels=(976, 680))
        for tty_input in [False, True]:
            auto = run([mixed], tty_input=tty_input, **geometry)
            assert len(images(auto)) == 4
            assert auto == run(["--grid", mixed], **geometry)
            cases += 1
        for environment in [{}, {"NO_COLOR": "1"}]:
            decorated = run([mixed], decorated=True, environment=environment, **geometry)
            assert len(images(decorated)) == 4 and "\uf0c1".encode() in decorated
            # All fixture names are ASCII; each emitted Nerd icon is one cell.
            modeled = SGR.sub(b"", decorated).decode().replace("\uf0c1", "@").replace("\uf15b", "f").encode()
            assert check_cursor(modeled, 122, 40, 39) == 4
            if environment: assert not SGR.search(decorated)
            cases += 1
        (mixed / ".extra").touch()
        assert b"\x1b" not in run(["-a", mixed], **geometry)
        cases += 1
        expected = run(["--no-images", mixed], **geometry)
        for flag in ["--no-images", "--protocol=none", "--preview-limit=0"]:
            for args in [["--grid", flag], [flag, "--grid"]]:
                assert run([*args, mixed], **geometry) == expected
                cases += 1
        assert len(images(run(["--preview-limit=3", mixed], **geometry))) == 3
        cases += 1
        diagnostic = run(["--diagnose", mixed], decorated=True, **geometry)
        assert b"layout=grid" in diagnostic and b"preview_candidates=4" in diagnostic
        assert b"\x1b" not in diagnostic
        cases += 1
        for args in [[], ["--grid", "--protocol=kitty"], ["--no-images"]]:
            piped = subprocess.run([str(BIN), *args, str(mixed)], capture_output=True, check=True)
            assert piped.stdout == b"image-0.png\nimage-1.png\nimage-2.png\nimage-3.png\ntext-0\ntext-1\ntext-2\ntext-3\n"
            assert not piped.stderr
            cases += 1

        # Small mixed directories and lone operands preview without special flags.
        one = root / "one"
        one.mkdir()
        (one / "picture.png").symlink_to(image)
        (one / "notes.txt").touch()
        for path in [one, one / "picture.png"]:
            for rows in [8, 15, 16, 40]:
                data = run([path], rows=rows)
                assert len(images(data)) == 1
                assert check_cursor(data, 80, rows, rows - 1) == 1
                cases += 1
        # Many file operands share the same row, rather than a grid per argument.
        data = run([mixed / f"image-{i}.png" for i in range(4)], **geometry)
        assert len(images(data)) == 4
        assert b"\x1b[25G" in data and b"\x1b[73G" in data
        cases += 1
        diagnostic = run(["--diagnose", *[mixed / f"image-{i}.png" for i in range(4)]], **geometry)
        assert b"entries=4" in diagnostic and diagnostic.count(b"layout=grid") == 1
        cases += 1
        assert not APC.search(run([one], decorated=True, environment={"TERM": "dumb"}))
        cases += 1
        # Full wrapped labels survive narrow terminal geometry, including graphemes.
        for name in ["桃e\u0301👩‍💻.png", "a-very-long-name-that-wraps-across-many-lines.png", "line\nbreak.png"]:
            (one / name).symlink_to(image)
        data = run(["--grid", one], cols=12, rows=8)
        decoded = plain(data).decode()
        assert "👩‍💻" in decoded and "e\u0301" in decoded
        assert len(images(data)) == 4
        assert len(re.findall(rb"\x1b_Ga=T", data)) == 4
        cases += 1

        styled = root / "styled"
        styled.mkdir()
        for name in ["photo10.png", "Photo1.png", "photo2.png", "code.rs", "archive.zip", "run", "notes.txt"]:
            (styled / name).touch()
        (styled / "run").chmod(0o755)
        (styled / "folder").mkdir()
        (styled / "link").symlink_to("notes.txt")
        baseline = run(["--no-images", "-1", styled], decorated=True)
        assert b"\x1b[1;34m" in baseline and b"\x1b[32m" in baseline
        assert "\uf07b folder".encode() in baseline and "\uf489 run".encode() in baseline
        ordered = plain(baseline)
        assert ordered.index(b"Photo1.png") < ordered.index(b"photo2.png") < ordered.index(b"photo10.png")
        cases += 1
        for env in [{"NO_COLOR": "1"}, {"NO_COLOR": "yes"}]:
            data = run(["--no-images", styled], decorated=True, environment=env)
            assert not SGR.search(data) and "\uf07b".encode() in data
            assert SGR.search(run(["--color=always", "--no-images", styled], decorated=True, environment=env))
            cases += 1
        assert SGR.search(run(["--no-images", styled], decorated=True, environment={"NO_COLOR": ""}))
        cases += 1
        data = run(["--no-images", styled], decorated=True, environment={"TERM_PROGRAM": "other", "TERM": "xterm-256color"})
        assert "▸ folder".encode() in data and "\uf07b".encode() not in data
        cases += 1
        data = run(["--no-images", styled], decorated=True, environment={"TERM": "dumb"})
        assert b"\x1b" not in data and "\uf07b".encode() not in data
        cases += 1
        data = run(["--no-images", "--icons=never", "--color=never", styled], decorated=True)
        assert b"\x1b" not in data and b"folder/" not in data
        cases += 1
        data = run(["--no-images", "--icons=never", "-1", styled], decorated=True, environment={"LS_COLORS": "di=31:*.rs=38;5;123:ln=\x1b]bad"})
        assert b"\x1b[31mfolder\x1b[0m" in data
        assert b"\x1b[38;5;123mcode.rs\x1b[0m" in data
        assert b"\x1b]bad" not in data
        cases += 1
        data = run(["--hyperlink", "--no-images", "--icons=never", styled], decorated=True)
        assert b"\x1b]8;;file:///" in data and b"\x1b]8;;\x1b\\" in data
        cases += 1
        data = run(["--no-images", "--header", styled], decorated=True)
        assert b"Permissions" in data and b"Owner" in data and b"Name" in data
        assert len(plain(data).splitlines()) == 10 and not APC.search(data)
        cases += 1
        # Escape untrusted names even when decorations and clickable links are on.
        unsafe = styled / "a\x1b]bad\nname"
        unsafe.touch()
        data = run(["--hyperlink", "-1", styled], decorated=True)
        assert b"%1B%5Dbad%0Aname" in data
        assert b"a\\u{1b}]bad\\nname" in data and b"a\x1b]bad" not in data
        cases += 1
        # A zero budget skips both row reservations and lazy cache initialization.
        cache = root / "unused-cache"
        data = run(["--grid", "--preview-limit=0", f"--cache-dir={cache}", one], decorated=True)
        assert not APC.search(data) and not cache.exists()
        cases += 1
        # Overflow must exit even with stdin/stdout on the same foreground TTY.
        many_text = root / "many-text"
        many_text.mkdir()
        for i in range(1000): (many_text / f"file{i:04}").touch()
        data = run([many_text], cols=40, rows=8, tty_input=True)
        assert len(data.split()) == 1000
        cases += 1

    many = ROOT / "img-test/generated/many"
    for tty_input in [False, True]:
        data = run([many], cols=122, rows=40, tty_input=tty_input)
        assert len(images(data)) == 16
        labels = plain(data)
        for i in range(40): assert labels.count(f"image-{i:02}.png@".encode()) == 1
        # The budget's tail stays compact instead of producing 24 empty tiles.
        assert len(labels.splitlines()) < 40
        cases += 1
    print(f"{cases} inline layout/style checks passed; real terminal appearance remains unverified.")


if __name__ == "__main__": main()

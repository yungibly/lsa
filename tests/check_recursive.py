#!/usr/bin/env python3
"""Recursive listing and shared image budgets in a PTY, without drawing images."""
from pathlib import Path
import re
import struct
import tempfile
import zlib

from check_pty import ROOT, capture, check_cursor, images
from check_layout import plain

STATS = re.compile(rb"lsa: cache: (\d+) hits, (\d+) misses, (\d+) writes, (\d+) errors\n")


def png():
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    return (b"\x89PNG\r\n\x1a\n"
            + chunk(b"IHDR", struct.pack(">IIBBBBB", 16, 16, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress((b"\0" + bytes([240, 30, 80, 255]) * 16) * 16))
            + chunk(b"IEND", b""))


def check_lifecycle(data):
    assert not any(command in data for command in [
        b"\x1b[?1049", b"\x1b[?1047", b"\x1b[?47", b"\x1b[?25",
        b"\x1b[2J", b"\x1b[6n",
    ]), "interactive terminal lifecycle returned"
    assert data.endswith(b"\r\n"), "shell prompt would start mid-line"


def main():
    cases = 0
    with tempfile.TemporaryDirectory(dir=ROOT / "target", prefix="recursive-") as temp:
        root = Path(temp)
        tree = root / "tree"
        names = []
        for letter in "abc":
            directory = tree / f"{letter}-images"
            directory.mkdir(parents=True)
            for i in range(2):
                name = f"photo-{letter}-{i}.png"
                (directory / name).write_bytes(png())
                names.append(name.encode())
        operand = tree.relative_to(ROOT)
        geometry = dict(cols=122, rows=24, pixels=(976, 384))
        for mode in ["-l", "--grid"]:
            for limit in [1, 3, 6]:
                cache = root / f"cache-{mode.strip('-')}-{limit}"
                code, data, err, _ = capture([
                    mode, "-R", f"--preview-limit={limit}",
                    f"--cache-dir={cache}", "--cache-stats", operand,
                ], **geometry)
                assert code == 0, err
                stats = STATS.fullmatch(err)
                assert stats, err
                hits, misses, writes, errors = map(int, stats.groups())
                # These are distinct source files. Exactly `limit` preview
                # attempts across the whole walk proves subdirectories do not
                # each receive a fresh budget. Folder artwork uses no attempts.
                assert hits + misses == limit and writes == misses and errors == 0, err
                frames = images(data)
                if mode == "-l":
                    assert len(frames) == limit, len(frames)
                else:
                    assert limit <= len(frames) <= 9, len(frames)
                text = plain(data)
                for name in names:
                    assert text.count(name) == 1, (name, text)
                for letter in "abc":
                    heading = f"{operand}/{letter}-images:\n".encode()
                    assert text.count(heading) == 1, text
                check_lifecycle(data)
                assert check_cursor(data, 122, 24, 23) == len(frames)
                cases += 1

        # An ancestor link is listed but cannot make -L traversal loop. Its
        # failure must not drop later sibling directories or their names.
        (tree / "a-images/back").symlink_to("..")
        cache = root / "cycle-cache"
        code, data, err, _ = capture([
            "-RL", "--preview-limit=1", f"--cache-dir={cache}", "--cache-stats", operand,
        ], **geometry)
        assert code == 1 and err.count(b"recursive directory cycle") == 1, err
        assert f"{operand}/a-images/back: recursive directory cycle".encode() in err, err
        stats = STATS.search(err)
        assert stats and sum(map(int, stats.groups()[:2])) == 1, err
        text = plain(data)
        assert b"back/" in text
        for name in names:
            assert text.count(name) == 1, (name, text)
        assert f"{operand}/c-images:\n".encode() in text
        check_lifecycle(data)
        cases += 1

        # -P leaves that same link alone; -a displays dots without recursing
        # into them. capture also checks unchanged terminal modes, once with
        # a real controlling input terminal and once with stdin disconnected.
        for tty_input in [False, True]:
            code, data, err, _ = capture([
                "-RaP", "-1", "--preview-limit=0", operand,
            ], tty_input=tty_input, **geometry)
            assert code == 0 and not err, err
            assert not images(data)
            text = plain(data)
            assert b"back@\n" in text
            assert text.count(b"./\n") == 8  # . and .. in each of four directories
            assert b"/back:\n" not in text and b"/.:\n" not in text and b"/..:\n" not in text
            for name in names:
                assert text.count(name) == 1, (name, text)
            check_lifecycle(data)
            cases += 1
    print(f"Passed {cases} recursive PTY checks (bytes/cursor/budgets, not visible graphics)")


if __name__ == "__main__":
    main()

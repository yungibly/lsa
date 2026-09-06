#!/usr/bin/env python3
"""Headless PTY checks: bytes/cursor bounds, not visible graphics."""
import base64
import errno
import fcntl
import os
from pathlib import Path
import pty
import re
import select
import struct
import subprocess
import termios
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / "target/release/lsa"
APC = re.compile(rb"\x1b_G([^;]*);([^\x1b]*)\x1b\\")
CSI = re.compile(rb"\x1b\[[0-9]*[ABG]")


def capture(args, *, cols=80, rows=24, pixels=(640, 384), environment=None, prefix=(), decorated=False, tty_input=False):
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, *pixels))
    before = termios.tcgetattr(slave)
    env = os.environ.copy()
    for name in ["TMUX", "STY", "ZELLIJ", "LS_COLORS", "NO_COLOR"]:
        env.pop(name, None)
    env.update(TERM="xterm-ghostty", TERM_PROGRAM="ghostty")
    env.update(environment or {})
    start = time.perf_counter()
    def foreground():
        os.setsid()
        fcntl.ioctl(slave, termios.TIOCSCTTY, 0)
    flags = [] if decorated else ["--color=never", "--icons=never", "-F"]
    child = subprocess.Popen([*prefix, str(BIN), *flags, *map(str, args)], cwd=ROOT, env=env,
                             stdin=slave if tty_input else subprocess.DEVNULL,
                             stdout=slave, stderr=subprocess.PIPE,
                             preexec_fn=foreground if tty_input else None)
    data = bytearray()
    after = None
    try:
        while True:
            if time.perf_counter() - start > 15:
                raise TimeoutError("lsa PTY output took more than 15 seconds")
            if select.select([master], [], [], 0.001)[0]:
                try:
                    chunk = os.read(master, 65536)
                except OSError as error:
                    if error.errno != errno.EIO:
                        raise
                    break
                if not chunk:
                    break
                data.extend(chunk)
            if child.poll() is not None and slave is not None:
                try: after = termios.tcgetattr(slave)
                except termios.error as error:
                    if not tty_input or error.args[0] != errno.ENOTTY: raise
                # Let the master report EOF/EIO only after every queued byte has
                # been drained. Do not race a final write against child.poll().
                os.close(slave)
                slave = None
        code = child.wait(timeout=2)
        err = child.stderr.read()
        try:
            if slave is not None: after = termios.tcgetattr(slave)
            if after is not None: assert before == after, "terminal modes changed"
        except termios.error as error:
            # macOS revokes the controlling PTY when its session leader exits.
            # Non-controlling trials still verify unchanged termios exactly.
            if not tty_input or error.args[0] != errno.ENOTTY:
                raise
        return code, bytes(data), err, time.perf_counter() - start
    finally:
        if child.poll() is None:
            child.kill()
            child.wait()
        child.stderr.close()
        os.close(master)
        if slave is not None: os.close(slave)


def images(data, *, pixels=False):
    decoded = []
    pending = bytearray()
    first = None
    for match in APC.finditer(data):
        control = dict(part.split(b"=", 1) for part in match[1].split(b","))
        assert control[b"q"] == b"2"
        assert len(match[2]) <= 4096 and len(match[2]) % 4 == 0
        if b"a" in control:
            assert first is None
            first = control
            assert control[b"a"] == b"T" and control[b"C"] == b"1"
            assert control[b"t"] == b"d" and control[b"f"] == b"32"
            assert b"i" not in control and b"p" not in control
        assert first is not None
        pending.extend(base64.b64decode(match[2], validate=True))
        if control[b"m"] == b"0":
            assert len(pending) == int(first[b"s"]) * int(first[b"v"]) * 4
            decoded.append((first, bytes(pending)) if pixels else first)
            first = None
            pending.clear()
    assert first is None
    return decoded


def check_cursor(data, cols, rows, start_row):
    """Small model of the emitted subset, run on ASCII fixture labels only."""
    row, col, offset, placements = start_row, 0, 0, 0
    while offset < len(data):
        apc = APC.match(data, offset)
        csi = CSI.match(data, offset)
        if apc:
            control = dict(part.split(b"=", 1) for part in apc[1].split(b","))
            if b"a" in control:
                assert row + int(control[b"r"]) < rows, (row, control)
                assert col + int(control[b"c"]) < cols, (col, control)
                placements += 1
            offset = apc.end()
        elif csi:
            n, op = int(csi[0][2:-1]), csi[0][-1:]
            if op == b"A":
                assert row >= n, "cursor-up clipped above reserved row"
                row -= n
            elif op == b"B":
                assert row + n < rows, "cursor-down clipped"
                row += n
            elif op == b"G":
                col = n - 1
            offset = csi.end()
        else:
            ch = data[offset]
            assert ch != 27, "unexpected terminal command"
            if ch == 13:
                col = 0
            elif ch == 10:
                row = min(rows - 1, row + 1)
            else:
                assert 32 <= ch < 127, "ASCII fixtures required for cursor model"
                col += 1
                assert col < cols, "right-edge autowrap"
            offset += 1
    assert col == 0, "prompt would begin mid-line"
    return placements


def main():
    many = ROOT / "img-test/generated/many"
    assert many.is_dir(), "Run target/release/examples/fixtures first"
    runs = 0
    for cols, rows in [(80, 24), (80, 8), (12, 8), (200, 50)]:
        code, data, err, _ = capture(["--grid", "--preview-limit=64", many], cols=cols, rows=rows, pixels=(cols * 8, rows * 16))
        assert code == 0 and not err, err
        assert len(images(data)) == 40
        for start in [0, rows - 1]:
            assert check_cursor(data, cols, rows, start) == 40
        runs += 1
    code, data, err, _ = capture(["--grid", "--preview-limit=2", many])
    assert code == 0 and len(images(data)) == 3 and not err
    text = CSI.sub(b"", APC.sub(b"", data))
    for i in range(40):
        assert f"image-{i:02}.png@".encode() in text
    runs += 1
    # At 32x80 cells, 40 full tiles exceed the 8 MiB image-output cap.
    code, data, err, _ = capture(["--grid", "--preview-limit=64", many], pixels=(2560, 1920))
    assert code == 0 and 0 < len(images(data)) < 40 and not err
    assert sum(len(m[0]) for m in APC.finditer(data)) <= 8 * 1024 * 1024
    runs += 1
    for args, env, cols, rows in [
        (["--no-images"], {}, 80, 24), (["-1"], {}, 80, 24), (["-l", "--no-images"], {}, 80, 24),
        (["--protocol=none"], {}, 80, 24), ([], {"TMUX": "test"}, 80, 24),
        ([], {"TERM": "dumb", "TERM_PROGRAM": "unknown"}, 80, 24),
        ([], {}, 11, 24), ([], {}, 80, 5),
    ]:
        code, data, err, _ = capture(["--grid", *args, many], environment=env, cols=cols, rows=rows)
        assert code == 0 and b"\x1b" not in data and not err
        runs += 1
    code, data, err, _ = capture(["--grid", ROOT / "img-test/generated"])
    assert code == 0 and len(images(data)) == 15 and not err, err
    runs += 1
    with tempfile.TemporaryDirectory(dir=ROOT / "target", prefix="pty-inputs-") as temp:
        temp = Path(temp)
        (temp / "oversized.png").touch()
        with (temp / "oversized.png").open("wb") as file:
            file.truncate(32 * 1024 * 1024 + 1)
        os.mkfifo(temp / "fifo.png")
        (temp / "linked-fifo.png").symlink_to("fifo.png")
        code, data, err, _ = capture(["--grid", temp])
        assert code == 0 and len(images(data)) == 3 and not err, err
        assert b"fifo.png|" in data and b"oversized.png" in data
        runs += 1
    print(f"{runs} PTY checks passed; frames, budgets, fallback, cursor bounds, modes, and special files. Visual rendering unverified.")


if __name__ == "__main__":
    main()

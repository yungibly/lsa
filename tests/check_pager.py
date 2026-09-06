#!/usr/bin/env python3
"""Interactive executable in a foreground controlling PTY; no terminal renderer.

Scratch files stay under target/. Checks screen bounds, interaction and exact
termios restoration, including kernel-generated signals and suspend/resume.
"""
import errno
import fcntl
import os
from pathlib import Path
import pty
import re
import resource
import select
import signal
import struct
import subprocess
import tempfile
import termios
import time

from check_pty import ROOT, BIN, APC, capture

CSI = re.compile(rb"\x1b\[([?0-9;]*)([A-Za-z])")
ENTER = b"\x1b[?1049h"
LEAVE = b"\x1b[0m\x1b[?2004l\x1b[?25h\x1b[?1049l"
FRAME = b"\x1b[0m\x1b[2J\x1b[H"
GRID_FRAME = b"\x1b[0m\x1b[H"


class Pager:
    def __init__(self, path, *options, cols=100, rows=12, environment=None, graphics=False, pixels=(0, 0)):
        self.master, self.slave = pty.openpty()
        self.cols, self.rows = cols, rows
        self.graphics = graphics
        self.resize(cols, rows, notify=False, pixels=pixels)
        self.before = termios.tcgetattr(self.slave)
        self.before_flags = fcntl.fcntl(self.slave, fcntl.F_GETFL)
        self.data = bytearray()

        def foreground():
            os.setsid()
            fcntl.ioctl(0, termios.TIOCSCTTY, 0)

        env = os.environ.copy()
        for name in ["TMUX", "STY", "ZELLIJ"]:
            env.pop(name, None)
        env.update(TERM="xterm-ghostty", TERM_PROGRAM="ghostty")
        env.update(environment or {})
        self.child = subprocess.Popen(
            [str(BIN), "--page", *([] if graphics else ["--no-images"]), *map(str, options), str(path)], cwd=ROOT,
            stdin=self.slave, stdout=self.slave, stderr=subprocess.PIPE,
            env=env, preexec_fn=foreground,
        )

    def __enter__(self):
        return self

    def __exit__(self, *_):
        if self.child.poll() is None:
            self.child.kill()
            self.wait_exit()
        self.child.stderr.close()
        os.close(self.master)
        os.close(self.slave)

    def read(self, duration=0.08):
        result = bytearray()
        deadline = time.monotonic() + duration
        while time.monotonic() < deadline:
            if select.select([self.master], [], [], max(0, deadline - time.monotonic()))[0]:
                try:
                    chunk = os.read(self.master, 65536)
                except OSError as error:
                    if error.errno != errno.EIO:
                        raise
                    break
                result.extend(chunk)
        self.data.extend(result)
        return bytes(result)

    def frame(self):
        result = bytearray()
        deadline = time.monotonic() + 3
        while time.monotonic() < deadline:
            result.extend(self.read())
            if (FRAME in result or GRID_FRAME in result) and result.endswith(b"\x1b[0m"):
                if not self.graphics:
                    assert b"\x1b_G" not in result, "text pager emitted image commands"
                return bytes(result)
            if self.child.poll() is not None:
                raise AssertionError((self.child.returncode, self.child.stderr.read(), result))
        raise TimeoutError(("no complete pager frame", result))

    def send(self, keys):
        os.write(self.master, keys)
        return self.frame()

    def resize(self, cols, rows, *, notify=True, pixels=(0, 0)):
        self.cols, self.rows = cols, rows
        fcntl.ioctl(self.slave, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, *pixels))
        if notify:
            os.kill(self.child.pid, signal.SIGWINCH)

    def screen(self, data):
        """Model the emitted subset using ASCII fixtures; reject offscreen writes."""
        data = APC.sub(b"", data)
        rows, cols = min(self.rows, 256), min(self.cols, 512)
        screen = [[" "] * cols for _ in range(rows)]
        row = col = 0
        cursor = 0
        while cursor < len(data):
            match = CSI.match(data, cursor)
            if match:
                parameters, op = match.groups()
                if op == b"H":
                    row, col = ([int(n) - 1 for n in parameters.split(b";")]
                                if parameters else [0, 0])
                    assert 0 <= row < rows and 0 <= col < cols
                elif (parameters, op) == (b"2", b"J"):
                    screen = [[" "] * cols for _ in range(rows)]
                elif (parameters, op) == (b"2", b"K"):
                    screen[row] = [" "] * cols
                else:
                    assert ((op == b"m" and parameters in [b"0", b"7"])
                            or (op in [b"h", b"l"] and parameters in [b"?1049", b"?25", b"?2004"])), match[0]
                cursor = match.end()
                continue
            byte = data[cursor]
            assert 32 <= byte < 127, ("unexpected byte", byte)
            assert 0 <= row < rows and col < cols - 1, ("autowrap or out of bounds", row, col)
            screen[row][col] = chr(byte)
            col += 1
            cursor += 1
        return ["".join(line).rstrip() for line in screen]

    def selected(self, data):
        return next((line[2:] for line in self.screen(data) if line.startswith("> ")), None)

    def finish(self, code=0, keys=b"q", cache_stats=False):
        if keys:
            os.write(self.master, keys)
        self.wait_exit()
        self.read()
        assert self.child.returncode == code, (self.child.returncode, self.child.stderr.read())
        error = self.child.stderr.read()
        if cache_stats:
            assert re.fullmatch(rb"lsa: cache: \d+ hits, \d+ misses, \d+ writes, \d+ errors\n", error), error
        else:
            assert not error, error
        assert bytes(self.data).endswith(LEAVE), bytes(self.data)[-200:]
        assert termios.tcgetattr(self.master) == self.before, "terminal modes not restored"
        assert self.data.count(ENTER) == self.data.count(b"\x1b[?1049l")
        return error

    def wait_exit(self):
        # Drain the PTY while waiting, including restoration output. This also
        # avoids relying on Python's platform-specific timed wait implementation.
        deadline = time.monotonic() + 3
        while self.child.poll() is None:
            self.read(0.02)
            if time.monotonic() > deadline:
                raise TimeoutError(("pager failed to exit", bytes(self.data)[-200:]))


def main():
    cases = 0
    with tempfile.TemporaryDirectory(dir=ROOT / "target", prefix="pager-") as temp:
        root = Path(temp)
        mixed = root / "mixed"
        mixed.mkdir()
        (mixed / ".hidden").touch()
        (mixed / "a.txt").write_text("small")
        (mixed / "b.png").write_bytes(b"invalid image contents")
        (mixed / "folder").mkdir()
        (mixed / "folder/inside").touch()
        (mixed / "link-dir").symlink_to("folder")
        (mixed / "lost-link").symlink_to("missing")
        os.mkfifo(mixed / "pipe.png")
        (mixed / "z-large").write_bytes(b"x" * 100)

        for options in [[], ["-a"], ["--dirs-first", "-r"], ["--dirs-first", "-S"], ["-t"]]:
            expected = subprocess.check_output([str(BIN), "-1", *options, str(mixed)]).decode().splitlines()
            with Pager(mixed, *options, "--cache-dir", root / "unused-cache") as pager:
                data = pager.frame()
                screen = pager.screen(data)
                listed = [line[2:] for line in screen[1:-2] if line]
                assert listed == expected, (listed, expected)
                assert pager.selected(data) == expected[0]
                assert termios.tcgetattr(pager.slave) != pager.before
                current_flags = fcntl.fcntl(pager.slave, fcntl.F_GETFL)
                assert current_flags & os.O_NONBLOCK == pager.before_flags & os.O_NONBLOCK
                pager.finish()
            assert not (root / "unused-cache").exists()
            cases += 1

        # Directories, links and dangling links are inspected, never entered.
        with Pager(mixed) as pager:
            pager.frame()
            assert pager.selected(pager.send(b"\x1b[Bj")) == "folder/"
            header = pager.screen(pager.send(b"\x1b[C\x1b[Dhl"))[0]
            for name in ["folder", "link-dir", "lost-link"]:
                data = pager.send(b"\r")
                assert f"Name: {name}" in " ".join(pager.screen(data))
                assert pager.screen(data)[0] == header
                assert "inside" not in " ".join(pager.screen(data))
                assert pager.selected(pager.send(b"\x7f")) == name + ("/" if name == "folder" else "@")
                pager.send(b"j")
            pager.finish()
        cases += 1

        with Pager(mixed / "link-dir") as pager:
            assert pager.selected(pager.frame()) == "inside"
            assert pager.selected(pager.send(b"h\x7f")) == "inside"
            pager.finish()
        cases += 1

        many = root / "many"
        many.mkdir()
        for i in range(60):
            (many / f"entry-{i:02}").touch()
        with Pager(many, rows=9) as pager:
            data = pager.frame()
            assert pager.selected(data) == "entry-00"
            # Incomplete arrow input does not leak bytes into actions.
            for byte in [b"\x1b", b"["]:
                os.write(pager.master, byte)
                assert pager.read(0.02) == b""
            assert pager.selected(pager.send(b"B")) == "entry-01"
            assert pager.selected(pager.send(b"\x1b[6~")) == "entry-07"
            assert pager.selected(pager.send(b"\x1b[5~")) == "entry-01"
            assert pager.selected(pager.send(b"G")) == "entry-59"
            assert pager.selected(pager.send(b"j")) == "entry-59"
            assert pager.selected(pager.send(b"\x1bOH")) == "entry-00"
            assert pager.selected(pager.send(b"k")) == "entry-00"
            assert pager.selected(pager.send(b"\x1bOF")) == "entry-59"
            pager.resize(37, 6)
            assert pager.selected(pager.frame()) == "entry-59"
            pager.resize(1, 1)
            pager.screen(pager.frame())
            pager.resize(100, 12)
            assert pager.selected(pager.frame()) == "entry-59"
            pager.resize(1000, 1000)
            assert pager.selected(pager.frame()) == "entry-59"
            pager.resize(100, 12)
            pager.frame()
            # Unknown CSI and bracketed paste are ignored, including action bytes.
            os.write(pager.master, b"\x1b[1;5A\x1b[200~q\rjj\x1b[201~")
            assert pager.read(0.15) == b""
            assert pager.selected(pager.send(b"k")) == "entry-58"
            # Listing membership/order stays fixed despite filesystem changes.
            (many / "000-new").touch()
            (many / "entry-58").unlink()
            os.write(pager.master, b"r")
            assert pager.read(0.1) == b"", "refresh is no longer a pager action"
            assert pager.selected(pager.send(b"Gk")) == "entry-58"
            assert pager.selected(pager.send(b"g ")) == "entry-09"
            assert pager.selected(pager.send(b"b")) == "entry-00"
            pager.finish()
        cases += 1

        full = root / "full"
        full.mkdir()
        long_name = "long-" + "0123456789" * 20 + "-end"
        (full / long_name).touch()
        with Pager(full, cols=25, rows=7) as pager:
            data = pager.frame()
            assert pager.selected(data).endswith(">")
            data = pager.send(b"\r")
            expected = []
            for line in [f"Name: {long_name}", "Kind: [file]", f"Path: {full / long_name}"]:
                expected.extend(line[i:i + 24] for i in range(0, len(line), 24))
            for top in range(len(expected) - 4 + 1):
                assert pager.screen(data)[1:-2] == expected[top:top + 4]
                data = pager.send(b"j")
            data = pager.send(b"G")
            assert "-end" in "".join(pager.screen(data)[1:-2])
            data = pager.send(b"\x1b")
            assert pager.selected(data).startswith("long-")
            pager.finish()
        cases += 1

        odd = root / "odd"
        odd.mkdir()
        (odd / "bad\n\x1b[31m").touch()
        (odd / "桃e\u0301👩‍💻.txt").touch()
        with Pager(odd) as pager:
            data = pager.frame()
            assert b"bad\\n\\u{1b}[31m" in data
            assert "桃e\u0301👩‍💻.txt".encode() in data
            assert b"\x1b[31m" not in data
            pager.finish()
        cases += 1

        empty = root / "empty"
        empty.mkdir()
        with Pager(empty) as pager:
            assert "(empty directory)" in pager.screen(pager.frame())
            assert pager.selected(pager.send(b"jG\r")) is None
            pager.finish(keys=b"\x04")
        cases += 1

        locked = mixed / "locked"
        locked.mkdir()
        locked.chmod(0)
        try:
            if os.geteuid() != 0:
                with Pager(locked) as pager:
                    pager.wait_exit()
                    assert pager.child.returncode == 1
                    assert ENTER not in pager.read()
                    assert termios.tcgetattr(pager.master) == pager.before
                cases += 1
        finally:
            locked.chmod(0o700)

        for path in [root / "missing", mixed / "a.txt"]:
            with Pager(path) as pager:
                pager.wait_exit()
                assert pager.child.returncode == 1
                assert ENTER not in pager.read()
                assert termios.tcgetattr(pager.master) == pager.before
                assert pager.child.stderr.read()
            cases += 1

        # Renaming the source directory leaves its listing usable.
        transient = root / "transient"
        transient.mkdir()
        (transient / "kept").touch()
        with Pager(transient) as pager:
            pager.frame()
            transient.rename(root / "renamed")
            assert pager.selected(pager.send(b"G")) == "kept"
            pager.finish()
        cases += 1

        for sig in [signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGQUIT]:
            with Pager(mixed) as pager:
                pager.frame()
                os.kill(pager.child.pid, sig)
                pager.finish(code=128 + sig, keys=None)
            cases += 1
        with Pager(mixed) as pager:
            pager.frame()
            pager.finish(code=130, keys=b"\x03") # TTY generates SIGINT via ISIG.
        cases += 1

        with Pager(mixed) as pager:
            pager.frame()
            pager.send(b"j")
            os.write(pager.master, b"\x1a") # TTY generates SIGTSTP.
            deadline = time.monotonic() + 3
            while time.monotonic() < deadline:
                pid, status = os.waitpid(pager.child.pid, os.WNOHANG | os.WUNTRACED)
                if pid:
                    assert os.WIFSTOPPED(status), status
                    break
                pager.read(0.02)
            else:
                raise TimeoutError("pager did not suspend")
            pager.read()
            assert bytes(pager.data).endswith(LEAVE)
            stopped_attributes = termios.tcgetattr(pager.master)
            # Darwin sets PENDIN when returning to canonical mode while the
            # controlling process is still alive. It is pending retype state,
            # not a failure to restore an input/output/local mode or control byte.
            stopped_attributes[3] &= ~getattr(termios, "PENDIN", 0)
            assert stopped_attributes == pager.before, (status, pager.before, stopped_attributes, bytes(pager.data)[-200:])
            os.kill(pager.child.pid, signal.SIGCONT)
            assert pager.selected(pager.frame()) == "b.png"
            pager.finish()
        cases += 1

        # Idle uses a blocking wait: no emitted frames and negligible process CPU.
        before = resource.getrusage(resource.RUSAGE_CHILDREN)
        with Pager(empty) as pager:
            pager.frame()
            start = time.monotonic()
            assert pager.read(1.0) == b""
            pager.finish()
            elapsed = time.monotonic() - start
        after = resource.getrusage(resource.RUSAGE_CHILDREN)
        cpu = after.ru_utime + after.ru_stime - before.ru_utime - before.ru_stime
        assert cpu < 0.10, ("idle process consumed excessive CPU", cpu)
        print(f"Idle trial: {elapsed:.3f}s observed, {cpu * 1000:.2f}ms child CPU (includes startup/quit)")
        cases += 1

        result = subprocess.run([str(BIN), "--page", str(mixed)], input=b"do not consume",
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        assert result.returncode == 1 and not result.stdout and b"requires terminal" in result.stderr
        code, data, err, _ = capture(["--page", mixed]) # stdout TTY, stdin /dev/null
        assert code == 1 and not data and b"requires terminal" in err
        cases += 2

        master, slave = pty.openpty()
        try:
            before = termios.tcgetattr(slave)
            os.write(master, b"unconsumed input\n")
            result = subprocess.run([str(BIN), "--page", str(mixed)], stdin=slave,
                                    stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            assert result.returncode == 1 and not result.stdout
            assert termios.tcgetattr(slave) == before
            assert select.select([slave], [], [], 0)[0]
            assert os.read(slave, 1024) == b"unconsumed input\n"
        finally:
            os.close(master)
            os.close(slave)
        cases += 1

    print(f"{cases} pager PTY/CLI scenarios passed; no renderer or Ghostty visual claim")


if __name__ == "__main__":
    main()

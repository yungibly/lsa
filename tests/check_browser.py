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


class Browser:
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
            [str(BIN), "--browse", *([] if graphics else ["--no-images"]), *map(str, options), str(path)], cwd=ROOT,
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
                    assert b"\x1b_G" not in result, "text browser emitted image commands"
                return bytes(result)
            if self.child.poll() is not None:
                raise AssertionError((self.child.returncode, self.child.stderr.read(), result))
        raise TimeoutError(("no complete browser frame", result))

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
                raise TimeoutError(("browser failed to exit", bytes(self.data)[-200:]))


def main():
    cases = 0
    with tempfile.TemporaryDirectory(dir=ROOT / "target", prefix="browser-") as temp:
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
            with Browser(mixed, *options, "--cache-dir", root / "unused-cache") as browser:
                data = browser.frame()
                screen = browser.screen(data)
                listed = [line[2:] for line in screen[1:-2] if line]
                assert listed == expected, (listed, expected)
                assert browser.selected(data) == expected[0]
                assert termios.tcgetattr(browser.slave) != browser.before
                current_flags = fcntl.fcntl(browser.slave, fcntl.F_GETFL)
                assert current_flags & os.O_NONBLOCK == browser.before_flags & os.O_NONBLOCK
                browser.finish()
            assert not (root / "unused-cache").exists()
            cases += 1

        with Browser(mixed) as browser:
            browser.frame()
            assert browser.selected(browser.send(b"\x1b[Bj")) == "folder/"
            data = browser.send(b"\r")
            assert browser.selected(data) == "inside"
            assert "/folder" in browser.screen(data)[0]
            assert browser.selected(browser.send(b"\x1b[D")) == "folder/"
            assert browser.selected(browser.send(b"j")) == "link-dir@"
            assert browser.selected(browser.send(b"l")) == "inside"
            assert browser.selected(browser.send(b"h")) == "link-dir@"
            browser.send(b"j")
            data = browser.send(b"\r")
            assert browser.selected(data) == "lost-link@"
            assert "Cannot navigate" in " ".join(browser.screen(data))
            browser.finish()
        cases += 1

        with Browser(mixed / "link-dir") as browser:
            assert browser.selected(browser.frame()) == "inside"
            assert browser.selected(browser.send(b"h")) == "folder/"
            browser.finish()
        cases += 1

        many = root / "many"
        many.mkdir()
        for i in range(60):
            (many / f"entry-{i:02}").touch()
        with Browser(many, rows=9) as browser:
            data = browser.frame()
            assert browser.selected(data) == "entry-00"
            # Incomplete arrow input does not leak bytes into actions.
            for byte in [b"\x1b", b"["]:
                os.write(browser.master, byte)
                assert browser.read(0.02) == b""
            assert browser.selected(browser.send(b"B")) == "entry-01"
            assert browser.selected(browser.send(b"\x1b[6~")) == "entry-07"
            assert browser.selected(browser.send(b"\x1b[5~")) == "entry-01"
            assert browser.selected(browser.send(b"G")) == "entry-59"
            assert browser.selected(browser.send(b"j")) == "entry-59"
            assert browser.selected(browser.send(b"\x1bOH")) == "entry-00"
            assert browser.selected(browser.send(b"k")) == "entry-00"
            assert browser.selected(browser.send(b"\x1bOF")) == "entry-59"
            browser.resize(37, 6)
            assert browser.selected(browser.frame()) == "entry-59"
            browser.resize(1, 1)
            browser.screen(browser.frame())
            browser.resize(100, 12)
            assert browser.selected(browser.frame()) == "entry-59"
            browser.resize(1000, 1000)
            assert browser.selected(browser.frame()) == "entry-59"
            browser.resize(100, 12)
            browser.frame()
            # Unknown CSI and bracketed paste are ignored, including action bytes.
            os.write(browser.master, b"\x1b[1;5A\x1b[200~q\rjj\x1b[201~")
            assert browser.read(0.15) == b""
            assert browser.selected(browser.send(b"k")) == "entry-58"
            # Refresh preserves the name even when a new entry shifts its index.
            (many / "000-new").touch()
            assert browser.selected(browser.send(b"r")) == "entry-58"
            (many / "entry-58").unlink()
            assert browser.selected(browser.send(b"r")) == "entry-59"
            browser.finish()
        cases += 1

        full = root / "full"
        full.mkdir()
        long_name = "long-" + "0123456789" * 20 + "-end"
        (full / long_name).touch()
        with Browser(full, cols=25, rows=7) as browser:
            data = browser.frame()
            assert browser.selected(data).endswith(">")
            data = browser.send(b" ")
            expected = []
            for line in [f"Name: {long_name}", "Kind: [file]", f"Path: {full / long_name}"]:
                expected.extend(line[i:i + 24] for i in range(0, len(line), 24))
            for top in range(len(expected) - 4 + 1):
                assert browser.screen(data)[1:-2] == expected[top:top + 4]
                data = browser.send(b"j")
            data = browser.send(b"G")
            assert "-end" in "".join(browser.screen(data)[1:-2])
            data = browser.send(b"\x1b")
            assert browser.selected(data).startswith("long-")
            browser.finish()
        cases += 1

        odd = root / "odd"
        odd.mkdir()
        (odd / "bad\n\x1b[31m").touch()
        (odd / "桃e\u0301👩‍💻.txt").touch()
        with Browser(odd) as browser:
            data = browser.frame()
            assert b"bad\\n\\u{1b}[31m" in data
            assert "桃e\u0301👩‍💻.txt".encode() in data
            assert b"\x1b[31m" not in data
            browser.finish()
        cases += 1

        empty = root / "empty"
        empty.mkdir()
        with Browser(empty) as browser:
            assert "(empty directory)" in browser.screen(browser.frame())
            assert browser.selected(browser.send(b"jG\r")) is None
            browser.finish(keys=b"\x04")
        cases += 1

        locked = mixed / "locked"
        locked.mkdir()
        locked.chmod(0)
        try:
            if os.geteuid() != 0:
                with Browser(locked) as browser:
                    browser.wait_exit()
                    assert browser.child.returncode == 1
                    assert ENTER not in browser.read()
                    assert termios.tcgetattr(browser.master) == browser.before
                cases += 1
        finally:
            locked.chmod(0o700)

        for path in [root / "missing", mixed / "a.txt"]:
            with Browser(path) as browser:
                browser.wait_exit()
                assert browser.child.returncode == 1
                assert ENTER not in browser.read()
                assert termios.tcgetattr(browser.master) == browser.before
                assert browser.child.stderr.read()
            cases += 1

        # A failed refresh keeps the previous usable screen/selection.
        transient = root / "transient"
        transient.mkdir()
        (transient / "kept").touch()
        with Browser(transient) as browser:
            browser.frame()
            transient.rename(root / "renamed")
            data = browser.send(b"r")
            assert browser.selected(data) == "kept"
            assert "Cannot navigate/refresh" in " ".join(browser.screen(data))
            browser.finish()
        cases += 1

        for sig in [signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGQUIT]:
            with Browser(mixed) as browser:
                browser.frame()
                os.kill(browser.child.pid, sig)
                browser.finish(code=128 + sig, keys=None)
            cases += 1
        with Browser(mixed) as browser:
            browser.frame()
            browser.finish(code=130, keys=b"\x03") # TTY generates SIGINT via ISIG.
        cases += 1

        with Browser(mixed) as browser:
            browser.frame()
            browser.send(b"j")
            os.write(browser.master, b"\x1a") # TTY generates SIGTSTP.
            deadline = time.monotonic() + 3
            while time.monotonic() < deadline:
                pid, status = os.waitpid(browser.child.pid, os.WNOHANG | os.WUNTRACED)
                if pid:
                    assert os.WIFSTOPPED(status), status
                    break
                browser.read(0.02)
            else:
                raise TimeoutError("browser did not suspend")
            browser.read()
            assert bytes(browser.data).endswith(LEAVE)
            stopped_attributes = termios.tcgetattr(browser.master)
            # Darwin sets PENDIN when returning to canonical mode while the
            # controlling process is still alive. It is pending retype state,
            # not a failure to restore an input/output/local mode or control byte.
            stopped_attributes[3] &= ~getattr(termios, "PENDIN", 0)
            assert stopped_attributes == browser.before, (status, browser.before, stopped_attributes, bytes(browser.data)[-200:])
            os.kill(browser.child.pid, signal.SIGCONT)
            assert browser.selected(browser.frame()) == "b.png"
            browser.finish()
        cases += 1

        # Idle uses a blocking wait: no emitted frames and negligible process CPU.
        before = resource.getrusage(resource.RUSAGE_CHILDREN)
        with Browser(empty) as browser:
            browser.frame()
            start = time.monotonic()
            assert browser.read(1.0) == b""
            browser.finish()
            elapsed = time.monotonic() - start
        after = resource.getrusage(resource.RUSAGE_CHILDREN)
        cpu = after.ru_utime + after.ru_stime - before.ru_utime - before.ru_stime
        assert cpu < 0.10, ("idle process consumed excessive CPU", cpu)
        print(f"Idle trial: {elapsed:.3f}s observed, {cpu * 1000:.2f}ms child CPU (includes startup/quit)")
        cases += 1

        result = subprocess.run([str(BIN), "--browse", str(mixed)], input=b"do not consume",
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        assert result.returncode == 1 and not result.stdout and b"requires terminal" in result.stderr
        code, data, err, _ = capture(["--browse", mixed]) # stdout TTY, stdin /dev/null
        assert code == 1 and not data and b"requires terminal" in err
        cases += 2

        master, slave = pty.openpty()
        try:
            before = termios.tcgetattr(slave)
            os.write(master, b"unconsumed input\n")
            result = subprocess.run([str(BIN), "--browse", str(mixed)], stdin=slave,
                                    stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            assert result.returncode == 1 and not result.stdout
            assert termios.tcgetattr(slave) == before
            assert select.select([slave], [], [], 0)[0]
            assert os.read(slave, 1024) == b"unconsumed input\n"
        finally:
            os.close(master)
            os.close(slave)
        cases += 1

    print(f"{cases} browser PTY/CLI scenarios passed; no renderer or Ghostty visual claim")


if __name__ == "__main__":
    main()

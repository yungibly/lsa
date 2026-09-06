#!/usr/bin/env python3
"""Browser Kitty byte/ownership/viewport checks, without a terminal renderer."""
import base64
import hashlib
import os
from pathlib import Path
import re
import signal
import struct
import tempfile
import time
import zlib

from check_browser import Browser, CSI, ENTER, LEAVE
from check_pty import APC, ROOT


def png(path, red=20):
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    rows = (b"\0" + bytes([red, 50, 100, 255]) * 32) * 16
    path.write_bytes(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 32, 16, 8, 6, 0, 0, 0))
                     + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b""))


def settle(browser):
    # Generated sources are tiny; also allow the real-image and worker startup cases.
    quiet = time.monotonic()
    deadline = quiet + 5
    while time.monotonic() < deadline:
        if browser.read(0.05):
            quiet = time.monotonic()
        elif time.monotonic() - quiet >= 0.2:
            return
    raise TimeoutError("browser preview work did not settle")


class Model:
    def __init__(self, cols=80, rows=24):
        self.offset = 0
        self.total = 0
        self.uploads = []
        self.active = {}
        self.pending = None
        self.pixels = bytearray()
        self.max_active = 0
        self.deletions = 0
        self.moves = 0
        self.row = self.col = 0
        self.resize(cols, rows)

    def resize(self, cols, rows):
        self.cols, self.rows = min(cols, 512), min(rows, 256)
        self.text = [[" "] * self.cols for _ in range(self.rows)]

    def take(self, browser):
        data = bytes(browser.data)[self.offset:]
        self.offset = len(browser.data)
        cursor = 0
        while cursor < len(data):
            apc = APC.match(data, cursor)
            csi = CSI.match(data, cursor)
            if apc:
                self.command(apc)
                cursor = apc.end()
            elif csi:
                params, op = csi.groups()
                if op == b"H":
                    self.row, self.col = ([int(n) - 1 for n in params.split(b";")] if params else [0, 0])
                    assert 0 <= self.row < self.rows and 0 <= self.col < self.cols
                elif (params, op) == (b"2", b"K"):
                    self.text[self.row] = [" "] * self.cols
                elif (params, op) == (b"2", b"J"):
                    assert not self.active, "full-screen erase used with browser images resident"
                    self.text = [[" "] * self.cols for _ in range(self.rows)]
                elif op in [b"h", b"l"] and params == b"?1049":
                    assert not self.active, "images survived alternate-screen transition"
                else:
                    assert ((op == b"m" and params in [b"0", b"7"])
                            or (op in [b"h", b"l"] and params in [b"?25", b"?2004"])), csi[0]
                cursor = csi.end()
            else:
                byte = data[cursor]
                assert 32 <= byte < 127, ("ASCII fixture expected", data[cursor:cursor + 50])
                assert self.col < self.cols - 1 and self.row < self.rows, "text autowrap/offscreen"
                self.text[self.row][self.col] = chr(byte)
                self.col += 1
                cursor += 1
        assert self.pending is None
        return data

    def command(self, apc):
        self.total += len(apc[0])
        assert self.total <= 8 * 1024 * 1024, "session exceeded image command budget"
        controls = dict(part.split(b"=", 1) for part in apc[1].split(b","))
        assert controls[b"q"] == b"2"
        if controls.get(b"a") == b"d":
            assert set(controls) == {b"a", b"d", b"I", b"q"}
            assert controls[b"d"] == b"N", "cleanup is not scoped to an image number"
            number = int(controls[b"I"])
            assert number in self.active, "cleanup targeted an unowned image"
            del self.active[number]
            self.deletions += 1
            return
        if controls.get(b"a") == b"p":
            number = int(controls[b"I"])
            assert number in self.active and controls[b"p"] == b"1" and controls[b"C"] == b"1"
            self.active[number]["row"] = self.row
            self.active[number]["col"] = self.col
            self.moves += 1
            self.bounds(self.active[number])
            return
        if controls.get(b"a") == b"T":
            assert self.pending is None
            number = int(controls[b"I"])
            assert number != 0 and number not in self.active
            assert b"i" not in controls and controls[b"p"] == b"1" and controls[b"C"] == b"1"
            assert controls[b"t"] == b"d" and controls[b"f"] == b"32"
            self.pending = {"number": number, "row": self.row, "col": self.col,
                            "cols": int(controls[b"c"]), "rows": int(controls[b"r"]),
                            "width": int(controls[b"s"]), "height": int(controls[b"v"])}
        assert self.pending is not None
        assert len(apc[2]) <= 4096 and len(apc[2]) % 4 == 0
        self.pixels.extend(base64.b64decode(apc[2], validate=True))
        if controls[b"m"] == b"0":
            image = self.pending
            assert len(self.pixels) == image["width"] * image["height"] * 4 <= 320 * 240 * 4
            center = ((image["height"] // 2) * image["width"] + image["width"] // 2) * 4
            image["red"] = self.pixels[center]
            image["digest"] = hashlib.sha256(self.pixels).hexdigest()
            self.active[image["number"]] = image
            self.uploads.append(image.copy())
            self.max_active = max(self.max_active, len(self.active))
            assert len(self.active) <= 32, "terminal residency exceeded 32 images"
            self.bounds(image)
            self.pending = None
            self.pixels.clear()

    def bounds(self, image):
        assert 1 <= image["row"] and image["row"] + image["rows"] < self.rows - 1
        assert image["col"] >= 0 and image["col"] + image["cols"] < self.cols

    def names_match_pixels(self):
        for image in self.active.values():
            label = "".join(self.text[image["row"] + 5][image["col"]:image["col"] + image["cols"]])
            assert f"image-{image['red']:03}.png" in label, (image, label)


def main():
    cases = 0
    with tempfile.TemporaryDirectory(dir=ROOT / "target", prefix="browser-images-") as temp:
        root = Path(temp)
        many = root / "many"
        many.mkdir()
        for i in range(100):
            png(many / f"image-{i:03}.png", i)

        with Browser(many, "--cache-dir", root / "viewport-cache", "--cache-stats", graphics=True, cols=80, rows=24) as b:
            first = b.frame()
            settle(b)
            m = Model()
            m.take(b)
            assert {p["red"] for p in m.active.values()} == set(range(9))
            m.names_match_pixels()
            assert first.index(b"image-008.png") < first.index(b"\x1b_G"), "names did not precede decoding"
            # Selection-only redraw retains images without additional commands.
            before = m.total
            b.send(b"j")
            m.take(b)
            assert m.total == before
            # Scroll one tile row; overlapping images move instead of re-uploading.
            b.send(b"jjjjjjjj")
            settle(b)
            m.take(b)
            m.names_match_pixels()
            assert m.moves == 6 and {p["red"] for p in m.active.values()} == set(range(3, 12))
            b.send(b"G")
            settle(b)
            m.take(b)
            assert {p["red"] for p in m.active.values()} == set(range(93, 100))
            m.names_match_pixels()
            error = b.finish(cache_stats=True)
            m.take(b)
            assert not m.active
            # Initial 9 visible + 1 after, then 3 new, then 7 visible + 1 before.
            assert b"21 misses, 21 writes, 0 errors" in error, error
        cases += 1

        for cols, rows in [(12, 10), (80, 24), (200, 50), (1000, 1000)]:
            with Browser(many, graphics=True, cols=cols, rows=rows) as b:
                b.frame()
                settle(b)
                m = Model(cols, rows)
                m.take(b)
                assert 1 <= len(m.active) <= 32
                b.finish()
                m.take(b)
                assert not m.active
            cases += 1

        with Browser(many, graphics=True, cols=80, rows=24) as b:
            b.frame()
            settle(b)
            m = Model()
            m.take(b)
            b.send(b"jj ")
            m.take(b)
            assert not m.active
            assert b"Name: image-002.png" in bytes(b.data)
            b.send(b"\x1b")
            settle(b)
            m.take(b)
            m.names_match_pixels()
            b.resize(40, 10, pixels=(400, 200))
            b.frame()
            settle(b)
            m.resize(40, 10)
            m.take(b)
            assert len(m.active) == 1 and next(iter(m.active.values()))["red"] == 2
            b.resize(10, 4)
            b.frame()
            m.resize(10, 4)
            m.take(b)
            assert not m.active
            b.finish()
            m.take(b)
        cases += 1

        for options, env in [(["--no-images"], {}), (["--protocol=none"], {}),
                             ([], {"TERM": "xterm", "TERM_PROGRAM": "unknown"}),
                             ([], {"TMUX": "test"})]:
            with Browser(many, *options, "--cache-dir", root / "never-open", graphics=True, environment=env) as b:
                assert b"\x1b_G" not in b.frame()
                settle(b)
                b.finish()
                assert b"\x1b_G" not in b.data
                assert not (root / "never-open").exists()
            cases += 1

        with Browser(many, "--preview-limit=0", "--cache-dir", root / "zero", graphics=True) as b:
            assert b"[limit]" in b.frame()
            settle(b)
            b.finish()
            assert b"\x1b_G" not in b.data and not (root / "zero").exists()
        cases += 1

        with Browser(many, "--preview-limit=2", graphics=True, cols=80, rows=24) as b:
            b.frame()
            settle(b)
            m = Model()
            m.take(b)
            assert len(m.uploads) == 2 and b"[limit]" in b.data
            b.send(b"G")
            settle(b)
            b.finish()
            m.take(b)
            assert len(m.uploads) == 2 and not m.active
        cases += 1

        with Browser(many, "--preview-limit=256", graphics=True, cols=80, rows=24, pixels=(3840, 1152)) as b:
            b.frame()
            settle(b)
            m = Model()
            m.take(b)
            for _ in range(10):
                b.send(b"\x1b[6~")
                settle(b)
                m.take(b)
            assert 10 < len(m.uploads) < 100 and b"[limit]" in b.data
            b.finish()
            m.take(b)
            assert not m.active and m.total <= 8 * 1024 * 1024
        cases += 1

        mixed = root / "mixed"
        mixed.mkdir()
        (mixed / "a-folder").mkdir()
        png(mixed / "a-folder/image-077.png", 77)
        (mixed / "b-file").touch()
        png(mixed / "c-image.png", 30)
        (mixed / "d-broken.png").write_bytes(b"bad image")
        (mixed / "e-link.png").symlink_to("c-image.png")
        os.mkfifo(mixed / "f-pipe.png")
        (mixed / "g-link.png").symlink_to("f-pipe.png")
        with Browser(mixed, graphics=True, cols=80, rows=24) as b:
            b.frame()
            settle(b)
            m = Model()
            m.take(b)
            assert len(m.active) == 2 and b"[no preview]" in b.data
            for label in [b"a-folder/", b"b-file", b"c-image.png", b"d-broken.png", b"e-link.png@", b"f-pipe.png|", b"g-link.png@"]:
                assert label in b.data
            b.send(b"\r")
            settle(b)
            m.take(b)
            assert len(m.active) == 1 and next(iter(m.active.values()))["red"] == 77
            b.send(b"h")
            settle(b)
            m.take(b)
            assert len(m.active) == 2
            png(mixed / "c-image.png", 60)
            b.send(b"r")
            settle(b)
            m.take(b)
            assert {p["red"] for p in m.active.values()} == {60}
            b.finish()
            m.take(b)
            assert not m.active
        cases += 1

        # Exact cached pixels match, while the CLI reports real completed cache work.
        digests = []
        for expected_hits in [0, 10]:
            with Browser(many, "--cache-dir", root / "warm", "--cache-stats", graphics=True, cols=80, rows=24) as b:
                b.frame()
                settle(b)
                m = Model()
                m.take(b)
                digests.append([image["digest"] for image in m.uploads])
                error = b.finish(cache_stats=True)
                m.take(b)
                assert f"{expected_hits} hits".encode() in error, error
        assert digests[0] == digests[1]
        cases += 1

        for sig in [signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGQUIT]:
            with Browser(many, graphics=True, cols=80, rows=24) as b:
                b.frame()
                settle(b)
                m = Model()
                m.take(b)
                os.kill(b.child.pid, sig)
                b.finish(code=128 + sig, keys=None)
                m.take(b)
                assert not m.active
            cases += 1

        with Browser(many, graphics=True, cols=80, rows=24) as b:
            b.frame()
            settle(b)
            m = Model()
            m.take(b)
            os.write(b.master, b"\x1a")
            deadline = time.monotonic() + 3
            while time.monotonic() < deadline:
                pid, status = os.waitpid(b.child.pid, os.WNOHANG | os.WUNTRACED)
                b.read(0.02)
                if pid:
                    assert os.WIFSTOPPED(status)
                    break
            else:
                raise TimeoutError("browser did not suspend")
            m.take(b)
            assert not m.active and bytes(b.data).endswith(LEAVE)
            os.kill(b.child.pid, signal.SIGCONT)
            b.frame()
            settle(b)
            m.take(b)
            assert len(m.active) == 9
            b.finish()
            m.take(b)
            assert not m.active
        cases += 1

        with Browser(many, graphics=True, cols=80, rows=24) as b:
            b.frame()
            settle(b)
            assert b.read(0.5) == b"", "completed previews caused idle redraws"
            b.finish()
        cases += 1

    print(f"{cases} browser image scenarios passed: viewport, ownership, limits, cache, signals, idle; no renderer")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Optional: render lsa's output with Ghostty's own terminal core (libghostty-vt)
through microsoft/tui-test, and check the text layer that terminal actually
shows: aligned names, colors, parsed OSC 8 links, grid labels, column order,
budget fallbacks, and no leaked escape payloads. Kitty images are parsed but not
drawn by this backend, so pixels remain a real-terminal check.

Skips unless the tui-test CLI is found at $TUI_TEST_BINARY or
target/tools/tui-test/tui-test (a GitHub release binary). Its daemon state is
kept under target/ by pointing HOME there.
"""
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BIN = Path(os.environ.get("LSA_TEST_BINARY", ROOT / "target/release/lsa"))
TOOL = Path(os.environ.get("TUI_TEST_BINARY", ROOT / "target/tools/tui-test/tui-test"))
STATE = ROOT / "target/tools/tui-test/home"
SESSION = "lsa-check"
# Base64 image data on screen would mean an APC sequence was not consumed.
LEAK = re.compile(r"[A-Za-z0-9+/]{48,}")


def tt(*args):
    env = dict(os.environ, HOME=str(STATE))
    result = subprocess.run([str(TOOL), *map(str, args)], env=env, capture_output=True, text=True, timeout=60)
    assert result.returncode == 0, (args, result.stdout, result.stderr)
    return result.stdout


def run(argv, *, cols=122, rows=40, env=None):
    """Run lsa to exit in a Ghostty-backed PTY; return the full text."""
    environment = {"TERM": "xterm-ghostty", "TERM_PROGRAM": "ghostty", "TZ": "UTC0", **(env or {})}
    tt("run", "--backend", "ghostty", "--session", SESSION, "--cols", cols, "--rows", rows,
       "--cwd", ROOT, *[f"--env={k}={v}" for k, v in environment.items()], "--restart", "--",
       BIN, *argv)
    tt("wait", "exit", "--session", SESSION)
    text = tt("text", "--full", "--session", SESSION)
    assert not LEAK.search(text), "escape payload leaked onto the screen"
    return text


def starts(name):
    found = json.loads(tt("find", "text", name, "--json", "--session", SESSION))["data"]["matches"]
    return [(m["start"]["row"], m["start"]["column"]) for m in found]


def cell(column, row):
    return json.loads(tt("cells", column, row, "1", "1", "--json", "--session", SESSION))["data"]["cells"][0]


def main():
    if not TOOL.exists():
        print(f"Skipped: tui-test not found at {TOOL} (set TUI_TEST_BINARY).")
        return
    STATE.mkdir(parents=True, exist_ok=True)
    cases = 0
    fixture = ROOT / "img-test/generated"
    names = ["broken.png", "dangling.png", "landscape.bmp", "landscape.jpg", "linked.png",
             "many", "notes.txt", "portrait.png", "still.gif", "transparent.png"]
    try:
        # Long view: miniatures and icons share one name column.
        for argv in [["-l", fixture], ["-l", "--icons=never", fixture], ["-n", "--header", fixture]]:
            run(argv)
            columns = {column for name in names for _, column in starts(name)}
            assert len(columns) == 1, (argv, columns)
            cases += 1
        with tempfile.TemporaryDirectory(dir=ROOT / "target", prefix="ghostty-vt-") as temp:
            root = Path(temp)
            styled = root / "styled"
            styled.mkdir()
            for name in ["notes.txt", "archive.zip", "code.rs"]:
                (styled / name).touch()
            (styled / "folder").mkdir()
            (styled / "gone").symlink_to("missing")
            # Colors as Ghostty resolves them: plain files keep the default
            # foreground; dangling links are red; directories bold blue.
            text = run(["-1", "--no-images", styled])
            expected = {"notes.txt": ("default", False), "archive.zip": ("1", False),
                        "code.rs": ("6", False), "folder": ("4", True), "gone": ("1", False)}
            for name, (fg, bold) in expected.items():
                (row, column), = starts(name)
                attributes = cell(column, row)
                assert (str(attributes["fg"]), attributes["bold"]) == (fg, bold), (name, attributes)
                # Links carry the absolute path, without `.` components.
                assert attributes["link"].endswith(str(styled / name).replace(" ", "%20")), attributes["link"]
                assert "/./" not in attributes["link"]
            cases += 1
            # -C fills columns down, -x rows across, like ls.
            fruit = root / "fruit"
            fruit.mkdir()
            for name in ["apple", "banana", "cherry", "date", "elder", "fig", "grape", "honeydew", "kiwi", "lemon"]:
                (fruit / name).touch()
            text = run(["-C", "--icons=never", fruit], cols=30, rows=10)
            assert text.splitlines()[:4] == ["apple   elder     kiwi", "banana  fig       lemon",
                                             "cherry  grape", "date    honeydew"], text
            text = run(["-x", "--icons=never", fruit], cols=30, rows=10)
            assert [line.split() for line in text.splitlines()[:2]] == [
                ["apple", "banana", "cherry"], ["date", "elder", "fig"]], text
            cases += 2
            # Wrapped grid labels in a narrow terminal reassemble to full names.
            narrow = root / "narrow"
            narrow.mkdir()
            long_name = "a-very-long-image-name-that-wraps.png"
            (narrow / long_name).symlink_to(fixture / "landscape.png")
            (narrow / "b.png").symlink_to(fixture / "landscape.png")
            text = run(["--grid", narrow], cols=12, rows=8)
            assert long_name in "".join(text.split()), text
            cases += 1
        # A tall gallery keeps every label in scrollback, each exactly once.
        text = run(["--grid", fixture / "many"], cols=122, rows=24)
        for i in range(40):
            assert text.count(f"image-{i:02}.png") == 1, i
        cases += 1
        # Once the budget is spent, the rest continue as compact text.
        text = run(["--grid", "--preview-limit=2", fixture / "many"], cols=80, rows=24)
        assert all(text.count(f"image-{i:02}.png") == 1 for i in range(40)), text
        cases += 1
    finally:
        subprocess.run([str(TOOL), "close", "--session", SESSION], env=dict(os.environ, HOME=str(STATE)),
                       capture_output=True)
    print(f"{cases} Ghostty VT (libghostty-vt via tui-test) text-layer checks passed; images are not drawn by this backend.")


if __name__ == "__main__":
    main()

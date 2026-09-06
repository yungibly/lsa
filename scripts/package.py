#!/usr/bin/env python3
"""Build and smoke-test a native release archive. All artifacts stay under target/."""
import argparse
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", help="Rust target; its binary must run on this machine")
    args = parser.parse_args()
    env = os.environ.copy()
    env.update(CARGO_HOME=str(ROOT / ".cargo-home"), CARGO_TARGET_DIR=str(ROOT / "target"))
    version = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]["version"]
    rust = subprocess.check_output(["rustc", "-vV"], text=True, cwd=ROOT, env=env)
    host = next(line.removeprefix("host: ") for line in rust.splitlines() if line.startswith("host: "))
    target = args.target or host
    subprocess.run(["cargo", "build", "--release", "--locked", "--bin", "lsa", "--target", target],
                   cwd=ROOT, env=env, check=True)
    binary = ROOT / "target" / target / "release/lsa"
    output = ROOT / "target/packages"
    output.mkdir(parents=True, exist_ok=True)
    name = f"lsa-{version}-{target}"
    with tempfile.TemporaryDirectory(dir=output, prefix="stage-") as temp:
        stage = Path(temp) / name
        (stage / "bin").mkdir(parents=True)
        shutil.copy2(binary, stage / "bin/lsa")
        (stage / "README.txt").write_text(
            f"lsa {version} ({target})\n\n"
            "Run bin/lsa [PATH ...]. Colored, icon-decorated listings with bounded\n"
            "inline thumbnails. Output stays in scrollback and returns to the shell.\n"
            "Use bin/lsa --help for options and controls (also in HELP.txt).\n\n"
            "To install, copy bin/lsa to a directory on your PATH. No data files,\n"
            "image programs, cache directory, or background service are required.\n"
            "This archive targets the operating system and architecture named above.\n"
        )
        (stage / "BUILD.txt").write_text(f"package target: {target}\n{rust}")
        (stage / "HELP.txt").write_bytes(subprocess.check_output([str(binary), "--help"], cwd=ROOT))
        archive = output / f"{name}.tar.gz"
        with tarfile.open(archive, "w:gz") as bundle:
            bundle.add(stage, arcname=name)
        # Exercise the artifact itself, after extraction to a different prefix.
        unpacked = Path(temp) / "unpacked"
        with tarfile.open(archive) as bundle:
            bundle.extractall(unpacked, filter="data")
        installed = unpacked / name / "bin/lsa"
        actual = subprocess.check_output([str(installed), "--version"], cwd=stage, env=env)
        assert actual == f"lsa {version}\n".encode(), actual
        fixture = Path(temp) / "listing"
        fixture.mkdir()
        (fixture / "photo.png").write_bytes(b"text fallback")
        (fixture / "folder").mkdir()
        actual = subprocess.check_output([str(installed), str(fixture)], cwd=stage, env=env)
        assert actual == b"folder\nphoto.png\n", actual
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    archive.with_suffix(archive.suffix + ".sha256").write_text(f"{digest}  {archive.name}\n")
    print(f"{archive}\nSHA-256 {digest}\nExtracted binary: version and piped mixed-listing checks passed.")


if __name__ == "__main__":
    main()

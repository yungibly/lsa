#!/usr/bin/env python3
"""Validate release inputs and generate metadata from smoke-tested archives."""
import argparse
import hashlib
from pathlib import Path
import re
import tomllib

ROOT = Path(__file__).resolve().parents[1]
TARGETS = {
    "macos": {"arm": "aarch64-apple-darwin", "intel": "x86_64-apple-darwin"},
    "linux": {"arm": "aarch64-unknown-linux-musl", "intel": "x86_64-unknown-linux-musl"},
}


def validate_tag(tag, version):
    if not re.fullmatch(r"v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", tag):
        raise ValueError("release tags must have the form vMAJOR.MINOR.PATCH (no prereleases)")
    if tag != f"v{version}":
        raise ValueError(f"tag {tag!r} does not match Cargo.toml version {version!r}")


def prepare(tag, version, repository, packages, output):
    validate_tag(tag, version)
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository):
        raise ValueError("repository must be OWNER/REPO")
    homepage = f"https://github.com/{repository}"
    digests = {}
    sums = []
    for targets in TARGETS.values():
        for target in targets.values():
            name = f"lsa-{version}-{target}.tar.gz"
            archive = packages / name
            # Recompute every checksum. Missing, renamed, or modified artifacts
            # must fail before either a release or a formula can be published.
            with archive.open("rb") as stream:
                digest = hashlib.file_digest(stream, "sha256").hexdigest()
            checksum = packages / f"{name}.sha256"
            expected = f"{digest}  {name}"
            if checksum.read_text().strip() != expected:
                raise ValueError(f"checksum mismatch: {name}")
            digests[target] = digest
            sums.append(expected)

    lines = [
        "class Lsa < Formula",
        '  desc "ls, augmented: colors, icons, and inline image thumbnails"',
        f'  homepage "{homepage}"',
        f'  version "{version}"',
    ]
    for system, targets in TARGETS.items():
        lines += ["", f"  on_{system} do"]
        if system == "macos":
            lines += ['    depends_on macos: :sonoma', ""]
        for arch, target in targets.items():
            name = f"lsa-{version}-{target}.tar.gz"
            lines += [
                f"    on_{arch} do",
                f'      url "{homepage}/releases/download/{tag}/{name}"',
                f'      sha256 "{digests[target]}"',
                "    end",
            ]
        lines += ["  end"]
    lines += [
        "",
        "  def install",
        '    bin.install "bin/lsa"',
        '    doc.install "README.txt", "HELP.txt", "BUILD.txt"',
        "  end",
        "",
        "  test do",
        '    assert_equal "lsa #{version}\\n", shell_output("#{bin}/lsa --version")',
        '    (testpath/"listing/folder").mkpath',
        '    (testpath/"listing/photo.png").write "text fallback"',
        '    assert_equal "folder\\nphoto.png\\n", shell_output("#{bin}/lsa #{testpath}/listing")',
        "  end",
        "end",
        "",
    ]
    output.mkdir(parents=True, exist_ok=True)
    (output / "lsa.rb").write_text("\n".join(lines))
    (output / "SHA256SUMS").write_text("\n".join(sums) + "\n")
    (output / "notes.md").write_text(
        f"**lsa {version} — ls, augmented.**\n\n"
        "Colored listings, file icons, and bounded inline image thumbnails. "
        "Output stays in terminal scrollback and returns to the shell.\n\n"
        "Native binaries for Apple Silicon and Intel macOS (Sonoma 14 or later), "
        "and ARM64 and x86-64 Linux (static musl). Each archive was built, tested, "
        "extracted, and smoke-tested on its matching architecture.\n\n"
        "Install from Homebrew once the release workflow's tap update completes:\n\n"
        "```sh\nbrew install yungibly/tap/lsa\n```\n\n"
        "Or download the matching `.tar.gz` and `SHA256SUMS`, verify with "
        "`shasum -a 256 --ignore-missing -c SHA256SUMS` on macOS or "
        "`sha256sum --ignore-missing -c SHA256SUMS` on Linux, then extract and "
        "copy `bin/lsa` onto your PATH.\n\n"
        "Automatic thumbnails target direct Ghostty/Kitty terminals. "
        "Headless CI checks protocol bytes, not terminal rendering; "
        "see [compatibility notes](" + homepage + f"/blob/{tag}/docs/compatibility.md).\n"
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["validate", "prepare"])
    parser.add_argument("--tag", required=True)
    parser.add_argument("--repository", default="yungibly/lsa")
    parser.add_argument("--packages", type=Path, default=ROOT / "target/packages")
    parser.add_argument("--output", type=Path, default=ROOT / "target/release-metadata")
    args = parser.parse_args()
    version = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]["version"]
    try:
        validate_tag(args.tag, version)
        if args.command == "prepare":
            prepare(args.tag, version, args.repository, args.packages, args.output)
    except (ValueError, OSError) as error:
        parser.exit(1, f"release: {error}\n")
    print(f"Validated {args.tag}" + (f"; metadata: {args.output}" if args.command == "prepare" else ""))


if __name__ == "__main__":
    main()

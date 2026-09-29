"""Release guards: all architectures, exact tags, and checksums before publication."""
import hashlib
import importlib.util
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("release", ROOT / "scripts/release.py")
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)


class ReleaseTests(unittest.TestCase):
    def test_tag_must_match_manifest_and_be_stable(self):
        release.validate_tag("v0.1.0", "0.1.0")
        for tag in ["0.1.0", "v0.2.0", "v0.1.0-rc.1", "v0.1.0/extra", "v00.1.0"]:
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                release.validate_tag(tag, "0.1.0")

    def test_notes_come_from_exactly_one_changelog_section(self):
        (ROOT / "target").mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=ROOT / "target", prefix="release-test-") as temp:
            changelog = Path(temp) / "CHANGELOG.md"
            changelog.write_text("# Changelog\n\n## v0.2.0 — later\n\nNewer.\n\n"
                                 "## v0.1.0 — 2026-01-01\n\n**Title.**\n\n- Item.\n\n## v0.0.1\n\nOld.\n")
            self.assertEqual(release.release_notes("0.1.0", changelog), "**Title.**\n\n- Item.")
            self.assertEqual(release.release_notes("0.0.1", changelog), "Old.")
            for version in ["0.3.0", "0.1"]:
                with self.subTest(version=version), self.assertRaisesRegex(ValueError, "section"):
                    release.release_notes(version, changelog)
            changelog.write_text("## v0.1.0\n\n## v0.0.1\n")
            with self.assertRaisesRegex(ValueError, "empty"):
                release.release_notes("0.1.0", changelog)
        # The repository's own changelog has a section for the current version.
        import tomllib
        version = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]["version"]
        self.assertTrue(release.release_notes(version))

    def test_formula_requires_all_unchanged_archives(self):
        (ROOT / "target").mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=ROOT / "target", prefix="release-test-") as temp:
            root = Path(temp)
            output = root / "metadata"
            changelog = root / "CHANGELOG.md"
            changelog.write_text("## v0.1.0\n\n**Reviewed notes.**\n")
            targets = [target for group in release.TARGETS.values() for target in group.values()]
            for target in targets:
                name = f"lsa-0.1.0-{target}.tar.gz"
                content = target.encode()
                (root / name).write_bytes(content)
                digest = hashlib.sha256(content).hexdigest()
                (root / f"{name}.sha256").write_text(f"{digest}  {name}\n")
            release.prepare("v0.1.0", "0.1.0", "yungibly/lsa", root, output, changelog)
            notes = (output / "notes.md").read_text()
            self.assertTrue(notes.startswith("**Reviewed notes.**\n\n## Install"))
            self.assertIn("SHA256SUMS", notes)
            formula = (output / "lsa.rb").read_text()
            self.assertEqual(formula.count("      sha256 "), 4)
            for target in targets:
                self.assertIn(f"lsa-0.1.0-{target}.tar.gz", formula)
            self.assertIn('bin.install "bin/lsa"', formula)
            self.assertEqual(len((output / "SHA256SUMS").read_text().splitlines()), 4)

            archive = root / f"lsa-0.1.0-{targets[0]}.tar.gz"
            archive.write_bytes(b"changed after build")
            with self.assertRaisesRegex(ValueError, "checksum mismatch"):
                release.prepare("v0.1.0", "0.1.0", "yungibly/lsa", root, root / "rejected", changelog)
            self.assertFalse((root / "rejected").exists())
            archive.unlink()
            with self.assertRaises(FileNotFoundError):
                release.prepare("v0.1.0", "0.1.0", "yungibly/lsa", root, root / "missing", changelog)

    def test_repository_cannot_inject_formula_code(self):
        with self.assertRaisesRegex(ValueError, "OWNER/REPO"):
            release.prepare("v0.1.0", "0.1.0", 'owner/repo";system("bad")', ROOT, ROOT)


if __name__ == "__main__":
    unittest.main()

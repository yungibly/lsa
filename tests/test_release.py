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

    def test_formula_requires_all_unchanged_archives(self):
        (ROOT / "target").mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=ROOT / "target", prefix="release-test-") as temp:
            root = Path(temp)
            output = root / "metadata"
            targets = [target for group in release.TARGETS.values() for target in group.values()]
            for target in targets:
                name = f"lsa-0.1.0-{target}.tar.gz"
                content = target.encode()
                (root / name).write_bytes(content)
                digest = hashlib.sha256(content).hexdigest()
                (root / f"{name}.sha256").write_text(f"{digest}  {name}\n")
            release.prepare("v0.1.0", "0.1.0", "yungibly/lsa", root, output)
            formula = (output / "lsa.rb").read_text()
            self.assertEqual(formula.count("      sha256 "), 4)
            for target in targets:
                self.assertIn(f"lsa-0.1.0-{target}.tar.gz", formula)
            self.assertIn('bin.install "bin/lsa"', formula)
            self.assertEqual(len((output / "SHA256SUMS").read_text().splitlines()), 4)

            archive = root / f"lsa-0.1.0-{targets[0]}.tar.gz"
            archive.write_bytes(b"changed after build")
            with self.assertRaisesRegex(ValueError, "checksum mismatch"):
                release.prepare("v0.1.0", "0.1.0", "yungibly/lsa", root, root / "rejected")
            self.assertFalse((root / "rejected").exists())
            archive.unlink()
            with self.assertRaises(FileNotFoundError):
                release.prepare("v0.1.0", "0.1.0", "yungibly/lsa", root, root / "missing")

    def test_repository_cannot_inject_formula_code(self):
        with self.assertRaisesRegex(ValueError, "OWNER/REPO"):
            release.prepare("v0.1.0", "0.1.0", 'owner/repo";system("bad")', ROOT, ROOT)


if __name__ == "__main__":
    unittest.main()

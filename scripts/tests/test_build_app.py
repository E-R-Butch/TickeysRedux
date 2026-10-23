import os
from pathlib import Path
import re
import shutil
import sys
import tempfile
import unittest
from unittest.mock import patch
from urllib.parse import urlsplit

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import build_app
from release_policy import verify_bundle_metadata


class BundledAudioNotesTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="tickeys-audio-notes-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        source = Path(__file__).resolve().parents[2]
        for name in ("assets", "docs", "licenses"):
            shutil.copytree(source / name, self.root / name)
        for name in ("LICENSE", "THIRD_PARTY_NOTICES.md"):
            shutil.copy2(source / name, self.root / name)
        (self.root / "scripts").mkdir()
        shutil.copy2(source / "scripts/generate_audio_assets.py", self.root / "scripts/generate_audio_assets.py")
        self.source_notes = (self.root / "docs/audio-assets.md").read_text()

        # Exercise actual resource assembly without a macOS compiler or signer.
        binary = self.root / "target/aarch64-apple-darwin/release/tickeys-redux"
        binary.parent.mkdir(parents=True)
        binary.write_bytes(b"test binary")
        sysroot = self.root / "rust"
        runtime_docs = sysroot / "share/doc/rust"
        (runtime_docs / "licenses").mkdir(parents=True)
        (runtime_docs / "COPYRIGHT-library.html").write_text("<html>runtime credits</html>")
        (runtime_docs / "licenses/MIT.txt").write_text("runtime license fixture")
        stamp = {"version": "1.0.8", "commit": "a" * 40, "dirty": False,
                 "target": "aarch64-apple-darwin"}

        def command(*args):
            return str(sysroot) if args == ("rustc", "--print", "sysroot") else "test toolchain"

        with patch.object(build_app, "ROOT", self.root), \
                patch.object(build_app, "source_stamp", return_value=stamp), \
                patch.object(build_app, "run"), \
                patch.object(build_app, "command", side_effect=command), \
                patch.object(build_app, "verify_bundle", side_effect=lambda bundle, version, commit, **kwargs:
                             verify_bundle_metadata(bundle, version, commit)), \
                patch.dict(os.environ, {}, clear=True), patch("builtins.print"):
            bundle, _ = build_app.build_app()
        self.resources = bundle / "Contents/Resources"
        self.notes = (self.resources / "AUDIO_ASSETS.md").read_text()

    def links(self, notes):
        return re.findall(r"\]\(([^)]+)\)", notes)

    def test_every_relative_link_resolves_inside_bundle_resources(self):
        relative_links = [link for link in self.links(self.notes) if not urlsplit(link).scheme]
        self.assertIn("LICENSE", relative_links)
        self.assertIn("scripts/generate_audio_assets.py", relative_links)
        self.assertIn("data/audio-manifest.json", relative_links)
        for link in relative_links:
            with self.subTest(link=link):
                target = (self.resources / link).resolve()
                self.assertTrue(target.is_relative_to(self.resources.resolve()), link)
                self.assertTrue(target.is_file(), link)

    def test_generator_source_and_license_are_available_offline(self):
        for name in ("scripts/generate_audio_assets.py", "LICENSE"):
            with self.subTest(name=name):
                self.assertEqual((self.resources / name).read_bytes(), (self.root / name).read_bytes())

    def test_external_links_and_source_document_are_unchanged(self):
        external = lambda notes: [link for link in self.links(notes) if urlsplit(link).scheme]
        self.assertEqual(external(self.notes), external(self.source_notes))
        self.assertEqual((self.root / "docs/audio-assets.md").read_text(), self.source_notes)


if __name__ == "__main__":
    unittest.main()

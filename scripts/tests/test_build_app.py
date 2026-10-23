import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
from urllib.parse import urlsplit

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import build_app
from release_policy import verify_bundle_metadata


class DependencyPreparationTests(unittest.TestCase):
    def run_until_build(self, official=False, debug=False, failure=None):
        commands = []
        fetched = False
        stamp = {"version": "1.0.8", "commit": "a" * 40, "dirty": False}

        class BuildReached(Exception):
            pass

        def run(*args, **kwargs):
            nonlocal fetched
            commands.append(args)
            if args == ("cargo", "fetch", "--locked"):
                if failure == "fetch":
                    raise subprocess.CalledProcessError(101, args)
                fetched = True
            elif args == ("python3", "scripts/generate_notices.py", "--check"):
                # The notice generator uses offline, locked Cargo metadata.
                # Model a fresh Cargo cache rather than CI's pre-fetched cache.
                if not fetched:
                    self.fail("offline notice verification ran before locked dependency fetch")
                if failure == "notices":
                    raise subprocess.CalledProcessError(1, args)
            elif args[:2] == ("cargo", "build"):
                raise BuildReached
            elif args[0] == "codesign":
                self.fail("preparation tests must never reach signing")

        with patch.object(build_app, "source_stamp", return_value=stamp), \
                patch.object(build_app, "release_source", return_value=stamp), \
                patch.object(build_app, "run", side_effect=run), \
                patch.dict(os.environ, {}, clear=True):
            expected = subprocess.CalledProcessError if failure else BuildReached
            with self.assertRaises(expected):
                build_app.build_app(official=official, debug=debug)
        return commands

    def test_cold_cache_fetches_locked_sources_before_offline_notices_and_build(self):
        for official, debug in ((False, False), (False, True), (True, False)):
            with self.subTest(official=official, debug=debug):
                commands = self.run_until_build(official, debug)
                self.assertEqual(commands[:3], [
                    ("python3", "scripts/generate_audio_assets.py", "--check"),
                    ("cargo", "fetch", "--locked"),
                    ("python3", "scripts/generate_notices.py", "--check"),
                ])
                self.assertEqual(commands[3][:3], ("cargo", "build", "--locked"))
                self.assertEqual(len(commands), 4)

    def test_fetch_failure_stops_before_notice_verification_build_or_signing(self):
        for official in (False, True):
            with self.subTest(official=official):
                commands = self.run_until_build(official=official, failure="fetch")
                self.assertEqual(commands[-1], ("cargo", "fetch", "--locked"))
                self.assertEqual(len(commands), 2)

    def test_notice_failure_stops_before_build_or_signing(self):
        for official in (False, True):
            with self.subTest(official=official):
                commands = self.run_until_build(official=official, failure="notices")
                self.assertEqual(commands[-1], ("python3", "scripts/generate_notices.py", "--check"))
                self.assertEqual(len(commands), 3)


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

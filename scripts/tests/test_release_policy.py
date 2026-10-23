import hashlib
import json
from pathlib import Path
import plistlib
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from release_policy import (BUNDLE_ID, RELEASE_CERT_SHA1, release_source,
                            validate_signing_environment, verify_bundle_metadata)


class ReleaseSourceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="tickeys-policy-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.git("init", "-q")
        (self.root / "Cargo.toml").write_text('[package]\nname = "fixture"\nversion = "1.0.8"\n')
        (self.root / ".gitignore").write_text("dist/\n")
        self.git("add", ".")
        self.git("-c", "user.name=Release Test", "-c", "user.email=release-test@example.invalid", "commit", "-qm", "fixture")

    def git(self, *args):
        return subprocess.check_output(["git", *args], cwd=self.root, text=True, stderr=subprocess.STDOUT).strip()

    def tag(self):
        self.git("-c", "user.name=Release Test", "-c", "user.email=release-test@example.invalid", "tag", "-a", "v1.0.8", "-m", "release fixture")

    def test_clean_annotated_tag_resolves_to_exact_build_commit(self):
        self.tag()
        (self.root / "dist").mkdir()
        (self.root / "dist/ignored-output").write_text("build artifact")
        stamp = release_source(self.root, {})
        self.assertEqual(stamp["commit"], self.git("rev-parse", "HEAD"))
        self.assertEqual(stamp["tag"], "v1.0.8")
        self.assertFalse(stamp["dirty"])

    def test_untracked_and_modified_sources_are_both_rejected(self):
        self.tag()
        untracked = self.root / "untracked.rs"
        untracked.write_text("new source")
        with self.assertRaisesRegex(ValueError, "clean worktree"):
            release_source(self.root, {})
        untracked.unlink()
        with (self.root / "Cargo.toml").open("a") as stream:
            stream.write("# local modification\n")
        with self.assertRaisesRegex(ValueError, "clean worktree"):
            release_source(self.root, {})

    def test_missing_lightweight_and_wrong_commit_tags_are_rejected(self):
        with self.assertRaisesRegex(ValueError, "missing release tag"):
            release_source(self.root, {})
        self.git("tag", "v1.0.8")
        with self.assertRaisesRegex(ValueError, "annotated"):
            release_source(self.root, {})
        self.git("tag", "-d", "v1.0.8")
        self.tag()
        (self.root / "change").write_text("later change")
        self.git("add", "change")
        self.git("-c", "user.name=Release Test", "-c", "user.email=release-test@example.invalid", "commit", "-qm", "later")
        with self.assertRaisesRegex(ValueError, "current build commit"):
            release_source(self.root, {})

    def test_overriding_both_signing_values_cannot_bypass_pin(self):
        with self.assertRaisesRegex(ValueError, "pinned"):
            validate_signing_environment({"TICKEYS_SIGNING_IDENTITY": "A" * 40,
                                          "TICKEYS_EXPECTED_SIGNING_CERT_SHA1": "A" * 40})
        with self.assertRaisesRegex(ValueError, "pinned"):
            validate_signing_environment({"TICKEYS_SIGNING_IDENTITY": "-"})
        validate_signing_environment({"TICKEYS_SIGNING_IDENTITY": RELEASE_CERT_SHA1})


class BundleMetadataTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="tickeys-bundle-test-")
        self.addCleanup(self.temp.cleanup)
        self.bundle = Path(self.temp.name) / "Fixture.app"
        self.resources = self.bundle / "Contents/Resources"
        self.resources.mkdir(parents=True)
        self.stamp = {"version": "1.0.8", "commit": "a" * 40, "dirty": False, "target": "aarch64-apple-darwin"}
        self.info = {"CFBundleShortVersionString": "1.0.8", "CFBundleVersion": "1.0.8",
                     "CFBundleIdentifier": BUNDLE_ID, "CFBundleDevelopmentRegion": "en",
                     "CFBundleLocalizations": ["en", "zh-Hans"], "LSMinimumSystemVersion": "13.0",
                     "TickeysBuildCommit": "a" * 40, "TickeysBuildDirty": False}
        for name in ["LICENSE", "THIRD_PARTY_NOTICES.md", "AUDIO_ASSETS.md", "data/fixture/key.wav",
                     "licenses/rust-runtime/COPYRIGHT.html", "licenses/dependencies/fixture/LICENSE"]:
            file = self.resources / name
            file.parent.mkdir(parents=True, exist_ok=True)
            file.write_text("fixture")
        for language in ("Base", "en", "zh-Hans"):
            directory = self.resources / f"{language}.lproj"
            directory.mkdir()
            (directory / "Localizable.strings").write_text('"a" = "b";')
        digest = hashlib.sha256(b"fixture").hexdigest()
        (self.resources / "data/schemes.json").write_text(json.dumps([{"name": "fixture", "files": ["key.wav"]}]))
        (self.resources / "data/audio-manifest.json").write_text(json.dumps({"assets": [
            {"path": "fixture/key.wav", "sha256": digest}]}))
        (self.resources / "licenses/dependencies.json").write_text(json.dumps([
            {"name": "fixture", "files": [{"path": "dependencies/fixture/LICENSE", "sha256": digest}]}]))
        self.write_metadata()

    def write_metadata(self):
        with (self.bundle / "Contents/Info.plist").open("wb") as stream:
            plistlib.dump(self.info, stream)
        (self.resources / "build-info.json").write_text(json.dumps(self.stamp))

    def verify(self):
        return verify_bundle_metadata(self.bundle, "1.0.8", "a" * 40, require_clean=True)

    def test_complete_metadata_is_accepted(self):
        self.assertEqual(self.verify(), self.stamp)

    def test_dirty_or_different_commit_metadata_is_rejected(self):
        self.stamp["dirty"] = True
        self.write_metadata()
        with self.assertRaisesRegex(ValueError, "dirty"):
            self.verify()
        self.stamp["dirty"] = False
        self.stamp["commit"] = "b" * 40
        self.write_metadata()
        with self.assertRaisesRegex(ValueError, "different commit"):
            self.verify()

    def test_missing_or_changed_license_is_rejected(self):
        (self.resources / "licenses/dependencies/fixture/LICENSE").write_text("altered")
        with self.assertRaisesRegex(ValueError, "license text mismatch"):
            self.verify()
        (self.resources / "THIRD_PARTY_NOTICES.md").unlink()
        with self.assertRaisesRegex(ValueError, "missing THIRD_PARTY"):
            self.verify()

    def test_mismatched_localization_and_version_are_rejected(self):
        self.info["CFBundleDevelopmentRegion"] = "zh_CN"
        self.write_metadata()
        with self.assertRaisesRegex(ValueError, "localization"):
            self.verify()
        self.info["CFBundleDevelopmentRegion"] = "en"
        self.info["CFBundleVersion"] = "1.0.7"
        self.write_metadata()
        with self.assertRaisesRegex(ValueError, "CFBundleVersion"):
            self.verify()

    def test_modified_audio_and_missing_inventory_are_rejected(self):
        (self.resources / "data/fixture/key.wav").write_bytes(b"changed")
        with self.assertRaisesRegex(ValueError, "audio file mismatch"):
            self.verify()
        (self.resources / "data/audio-manifest.json").write_text('{"assets": []}')
        with self.assertRaisesRegex(ValueError, "does not cover"):
            self.verify()


class RuntimeLicenseTests(unittest.TestCase):
    def test_rust_185_and_modern_toolchain_layouts(self):
        from build_app import copy_runtime_licenses
        for modern in (False, True):
            with self.subTest(modern=modern), tempfile.TemporaryDirectory() as folder:
                root = Path(folder)
                source = root / "rust"
                source.mkdir()
                if modern:
                    (source / "COPYRIGHT-library.html").write_text("<html>runtime credits</html>")
                    (source / "licenses").mkdir()
                    (source / "licenses/MIT.txt").write_text("MIT fixture")
                else:
                    (source / "COPYRIGHT").write_text("runtime <credits> & authors")
                    (source / "LICENSE-MIT").write_text("MIT fixture")
                    (source / "LICENSE-APACHE").write_text("Apache fixture")
                output = root / "collected"
                copy_runtime_licenses(source, output)
                self.assertTrue((output / "COPYRIGHT.html").is_file())
                self.assertEqual((output / "licenses/MIT.txt").read_text(), "MIT fixture")
                if not modern:
                    self.assertIn("&lt;credits&gt; &amp;", (output / "COPYRIGHT.html").read_text())


if __name__ == "__main__":
    unittest.main()

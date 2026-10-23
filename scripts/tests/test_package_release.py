"""Release publication integration with build/signing/mounting mocked out."""
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import package_release
from release_policy import RELEASE_CERT_SHA1


class ReleasePublicationTests(unittest.TestCase):
    def run_release(self, collide=False):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            bundle = root / "Fixture.app"
            resources = bundle / "Contents/Resources"
            resources.mkdir(parents=True)
            stamp = {"version": "1.0.8", "commit": "a" * 40, "dirty": False,
                     "tag": "v1.0.8", "target": "aarch64-apple-darwin"}
            (resources / "build-info.json").write_text(json.dumps(stamp))
            dist = root / "dist"
            zip_path = dist / "Tickeys.Redux.v1.0.8.zip"
            source_checks = 0

            def release_source(_):
                nonlocal source_checks
                source_checks += 1
                if collide and source_checks == 2:
                    dist.mkdir()
                    zip_path.write_bytes(b"previous release")
                return stamp

            def run(*args):
                if args[:3] == ("ditto", "-c", "-k"):
                    Path(args[-1]).write_bytes(b"verified archive")

            def create_dmg(_, destination, stamp, official):
                Path(destination).write_bytes(b"verified image")

            with patch.object(package_release, "ROOT", root), \
                    patch.object(package_release, "release_source", side_effect=release_source), \
                    patch.object(package_release, "command", return_value=RELEASE_CERT_SHA1), \
                    patch.object(package_release, "build_app", return_value=(bundle, stamp)), \
                    patch.object(package_release, "run", side_effect=run), \
                    patch.object(package_release, "create_dmg", side_effect=create_dmg), \
                    patch.object(package_release, "verify_bundle"), \
                    patch.object(sys, "argv", ["package_release.py"]), patch("builtins.print"):
                if collide:
                    with self.assertRaises(SystemExit) as error:
                        package_release.main()
                    self.assertEqual(error.exception.code, 1)
                else:
                    package_release.main()
            if collide:
                self.assertEqual(zip_path.read_bytes(), b"previous release")
                self.assertEqual(list(dist.iterdir()), [zip_path])
            else:
                self.assertEqual(len(list(dist.iterdir())), 4)
                checksums = (dist / "Tickeys.Redux.v1.0.8.sha256.txt").read_text()
                for line in checksums.splitlines():
                    checksum, name = line.split("  ")
                    self.assertEqual(hashlib.sha256((dist / name).read_bytes()).hexdigest(), checksum)

    def test_late_destination_collision_keeps_existing_release(self):
        self.run_release(collide=True)

    def test_complete_release_has_matching_checksums(self):
        self.run_release()


if __name__ == "__main__":
    unittest.main()

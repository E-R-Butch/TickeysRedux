"""Exercise publication races without building, signing, or mounting a real app."""
import errno
import os
from concurrent.futures import ThreadPoolExecutor
from threading import Barrier
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import build_app
import artifact_publication
from artifact_publication import publish_artifact


class DiskImagePublicationTests(unittest.TestCase):
    def test_destination_created_during_packaging_is_not_overwritten(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            bundle = root / "Fixture.app"
            bundle.mkdir()
            destination = root / "output.dmg"

            def run(*args):
                if args[0] == "create-dmg":
                    Path(args[-2]).write_bytes(b"new image")
                elif args[:2] == ("hdiutil", "attach"):
                    mount = Path(args[-1])
                    mount.mkdir()
                    (mount / "Applications").symlink_to("/Applications")
                    (mount / ".background").mkdir()
                    (mount / ".background/dmg_bg.png").write_bytes(b"background")
                    (mount / ".DS_Store").write_bytes(b"layout")
                elif args[:2] == ("hdiutil", "detach"):
                    # Another invocation wins after the initial exists() check.
                    destination.write_bytes(b"existing image")

            with patch.object(build_app.shutil, "which", return_value="create-dmg"), \
                    patch.object(build_app, "run", side_effect=run), \
                    patch.object(build_app, "verify_bundle"), patch("builtins.print"):
                with self.assertRaises((FileExistsError, ValueError)):
                    build_app.create_dmg(bundle, destination, {"version": "1.0.8", "commit": "a" * 40})
            self.assertEqual(destination.read_bytes(), b"existing image")


class AtomicPublicationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source = self.root / "source"
        self.source.write_bytes(b"complete artifact")
        self.destination = self.root / "artifact.dmg"

    def assert_no_staging_files(self):
        self.assertEqual(list(self.root.glob(".tickeys-artifact-*")), [])

    def test_new_destination_is_complete_and_source_is_retained(self):
        publish_artifact(self.source, self.destination)
        self.assertEqual(self.destination.read_bytes(), self.source.read_bytes())
        self.assert_no_staging_files()

    def test_existing_file_directory_and_symlinks_are_never_changed(self):
        target = self.root / "target"
        target.write_bytes(b"original")
        for kind in ("file", "directory", "symlink", "dangling"):
            with self.subTest(kind=kind):
                destination = self.root / kind
                if kind == "file":
                    destination.write_bytes(b"original")
                elif kind == "directory":
                    destination.mkdir()
                else:
                    destination.symlink_to(target if kind == "symlink" else self.root / "missing")
                with self.assertRaises(FileExistsError):
                    publish_artifact(self.source, destination)
                if kind == "file":
                    self.assertEqual(destination.read_bytes(), b"original")
                elif kind == "directory":
                    self.assertEqual(list(destination.iterdir()), [])
                else:
                    self.assertTrue(destination.is_symlink())
                self.assertEqual(target.read_bytes(), b"original")
                self.assertFalse((self.root / "missing").exists())
                self.assert_no_staging_files()

    def test_simultaneous_publishers_have_exactly_one_winner(self):
        second = self.root / "second"
        second.write_bytes(b"second complete artifact")
        ready = Barrier(2)
        link = os.link

        def simultaneous_link(source, destination):
            ready.wait(timeout=5)
            link(source, destination)

        def publish(source):
            try:
                publish_artifact(source, self.destination)
                return source.read_bytes()
            except FileExistsError:
                return None

        with patch.object(artifact_publication.os, "link", side_effect=simultaneous_link), \
                ThreadPoolExecutor(max_workers=2) as executor:
            results = list(executor.map(publish, [self.source, second]))
        winners = [result for result in results if result is not None]
        self.assertEqual(len(winners), 1)
        self.assertEqual(self.destination.read_bytes(), winners[0])
        self.assertTrue(self.source.is_file())
        self.assertTrue(second.is_file())
        self.assert_no_staging_files()

    def test_copy_failure_leaves_no_destination_or_staging_file(self):
        def fail_after_partial_copy(source, staged):
            staged.write(b"partial")
            raise OSError("simulated full disk")

        with patch.object(artifact_publication.shutil, "copyfileobj", side_effect=fail_after_partial_copy):
            with self.assertRaisesRegex(OSError, "full disk"):
                publish_artifact(self.source, self.destination)
        self.assertFalse(self.destination.exists())
        self.assert_no_staging_files()

    def test_unsupported_filesystem_fails_closed(self):
        with patch.object(artifact_publication.os, "link", side_effect=OSError(errno.EOPNOTSUPP, "unsupported")):
            with self.assertRaises(OSError):
                publish_artifact(self.source, self.destination)
        self.assertFalse(self.destination.exists())
        self.assert_no_staging_files()


if __name__ == "__main__":
    unittest.main()

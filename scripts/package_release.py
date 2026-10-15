#!/usr/bin/env python3
"""Package an immutable tagged release using the existing official identity."""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

from build_app import ROOT, build_app, create_dmg, run
from release_policy import RELEASE_CERT_SHA1, RELEASE_CERT_SHA256, command, release_source, verify_bundle


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="check source, tag and signing identity without creating artifacts")
    args = parser.parse_args()
    try:
        stamp = release_source(ROOT)
        identities = command("security", "find-identity", "-p", "codesigning")
        if RELEASE_CERT_SHA1 not in identities.upper():
            raise ValueError("the pinned release certificate/private key is not in this Mac's keychain; use the original signing Mac")
        if args.check:
            print(json.dumps(stamp, indent=2, sort_keys=True))
            return
        # No artifacts are removed if preflight fails, and existing releases
        # cannot be overwritten by accidentally rerunning the packaging command.
        dist = ROOT / "dist"
        names = [f'Tickeys.Redux.v{stamp["version"]}.{ext}' for ext in ("zip", "dmg", "sha256.txt", "build.json")]
        if any((dist / name).exists() for name in names):
            raise ValueError("release artifacts already exist; refusing to replace them")
        run("cargo", "fmt", "--all", "--", "--check")
        run("cargo", "test", "--all-targets", "--locked")
        run("cargo", "clippy", "--all-targets", "--locked", "--", "-D", "warnings")
        run("python3", "-m", "unittest", "discover", "-s", "scripts/tests")
        bundle, built_stamp = build_app(official=True)
        if built_stamp != stamp:
            raise ValueError("build source changed after release preflight")
        with tempfile.TemporaryDirectory(prefix="tickeys-release-") as folder:
            folder = Path(folder)
            zip_path, dmg_path, checksum_path, metadata_path = [folder / name for name in names]
            create_dmg(bundle, dmg_path, stamp, official=True)
            run("ditto", "-c", "-k", "--sequesterRsrc", "--keepParent", bundle, zip_path)
            run("unzip", "-t", zip_path)
            unpacked = folder / "unpacked"
            run("ditto", "-x", "-k", zip_path, unpacked)
            verify_bundle(unpacked / bundle.name, stamp["version"], stamp["commit"], official=True)
            build_info = json.loads((bundle / "Contents/Resources/build-info.json").read_text())
            metadata = dict(build_info, signing_certificate_sha1=RELEASE_CERT_SHA1, signing_certificate_sha256=RELEASE_CERT_SHA256)
            metadata_path.write_text(json.dumps(metadata, indent=2, sort_keys=True) + "\n")
            checksums = "".join(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}\n" for path in (zip_path, dmg_path, metadata_path))
            checksum_path.write_text(checksums)
            if release_source(ROOT) != stamp:
                raise ValueError("source/tag changed during packaging; artifacts were not published")
            dist.mkdir(exist_ok=True)
            for path in (zip_path, dmg_path, checksum_path, metadata_path):
                shutil.move(str(path), dist / path.name)
        print(f"Verified release {stamp['tag']} at {stamp['commit']}: {dist}")
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"Release blocked: {error}\n")


if __name__ == "__main__":
    main()

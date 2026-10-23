#!/usr/bin/env python3
"""Build a local app, or a source/tag/certificate-verified official app."""

import argparse
import html
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import tempfile

from artifact_publication import publish_artifact
from release_policy import BUNDLE_ID, RELEASE_CERT_SHA1, command, release_source, source_stamp, verify_bundle

ROOT = Path(__file__).resolve().parent.parent
APP_NAME = "Tickeys Redux"


def run(*args, **kwargs):
    subprocess.run([str(arg) for arg in args], check=True, cwd=ROOT, **kwargs)


def copy_runtime_licenses(runtime_docs, destination):
    """Support both Rust 1.85's text files and newer toolchain inventories."""
    destination.mkdir()
    copyright_path = next((runtime_docs / name for name in
                           ("COPYRIGHT-library.html", "COPYRIGHT.html", "COPYRIGHT")
                           if (runtime_docs / name).is_file()), None)
    if copyright_path is None:
        raise ValueError("Rust toolchain copyright inventory is missing")
    if copyright_path.suffix == ".html":
        shutil.copy2(copyright_path, destination / "COPYRIGHT.html")
    else:
        text = copyright_path.read_text()
        shutil.copy2(copyright_path, destination / "COPYRIGHT.txt")
        (destination / "COPYRIGHT.html").write_text(
            '<!doctype html><meta charset="utf-8"><title>Rust copyright notices</title><pre>'
            + html.escape(text) + '</pre>\n')
    if (runtime_docs / "licenses").is_dir():
        shutil.copytree(runtime_docs / "licenses", destination / "licenses")
    else:
        (destination / "licenses").mkdir()
        for source, target in [("LICENSE-MIT", "MIT.txt"), ("LICENSE-APACHE", "Apache-2.0.txt")]:
            shutil.copy2(runtime_docs / source, destination / "licenses" / target)


def build_app(official=False, debug=False):
    if not official and os.environ.get("TICKEYS_EXPECTED_SIGNING_CERT_SHA1"):
        raise ValueError("expected release certificate checks require --official; local builds must not silently ignore them")
    stamp = release_source(ROOT) if official else source_stamp(ROOT)
    if official and debug:
        raise ValueError("official builds cannot use the debug profile")
    run("python3", "scripts/generate_audio_assets.py", "--check")
    run("python3", "scripts/generate_notices.py", "--check")
    environment = os.environ.copy()
    environment["MACOSX_DEPLOYMENT_TARGET"] = "13.0"
    environment["CARGO_TARGET_DIR"] = str(ROOT / "target")
    profile = "debug" if debug else "release"
    args = ["cargo", "build", "--locked", "--target", "aarch64-apple-darwin", "--bin", "tickeys-redux"]
    if not debug:
        args.append("--release")
    run(*args, env=environment)
    after = release_source(ROOT) if official else source_stamp(ROOT)
    if after != stamp:
        raise ValueError("source revision or worktree state changed during the build")
    builds = ROOT / "dist/builds"
    builds.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="tickeys-bundle-") as folder:
        bundle = Path(folder) / f"{APP_NAME}.app"
        contents = bundle / "Contents"
        resources = contents / "Resources"
        (contents / "MacOS").mkdir(parents=True)
        resources.mkdir()
        shutil.copy2(ROOT / f"target/aarch64-apple-darwin/{profile}/tickeys-redux", contents / "MacOS/tickeys-redux")
        shutil.copytree(ROOT / "assets/data", resources / "data", ignore=shutil.ignore_patterns("*.bak", ".DS_Store"))
        shutil.copy2(ROOT / "assets/tickeys_redux.icns", resources / "tickeys.icns")
        for source, language in [("Base", "Base"), ("Base", "en"), ("zh-Hans", "zh-Hans")]:
            shutil.copytree(ROOT / f"assets/lproj/{source}.lproj", resources / f"{language}.lproj")
        for name in ("LICENSE", "THIRD_PARTY_NOTICES.md"):
            shutil.copy2(ROOT / name, resources / name)
        shutil.copytree(ROOT / "licenses", resources / "licenses")
        # Include the generator source for offline provenance inspection.
        (resources / "scripts").mkdir()
        shutil.copy2(ROOT / "scripts/generate_audio_assets.py", resources / "scripts/generate_audio_assets.py")
        # Documentation links are relative to Resources in the offline bundle.
        audio_notes = (ROOT / "docs/audio-assets.md").read_text()
        for source, target in [("../assets/data/", "data/"), ("../LICENSE", "LICENSE"),
                               ("../scripts/generate_audio_assets.py", "scripts/generate_audio_assets.py")]:
            audio_notes = audio_notes.replace(f"]({source}", f"]({target}")
        (resources / "AUDIO_ASSETS.md").write_text(audio_notes)
        runtime_docs = Path(command("rustc", "--print", "sysroot")) / "share/doc/rust"
        runtime_target = resources / "licenses/rust-runtime"
        copy_runtime_licenses(runtime_docs, runtime_target)
        build_info = dict(stamp, rustc=command("rustc", "--version"), cargo=command("cargo", "--version"))
        (resources / "build-info.json").write_text(json.dumps(build_info, indent=2, sort_keys=True) + "\n")
        info = {
            "CFBundleDevelopmentRegion": "en",
            "CFBundleLocalizations": ["en", "zh-Hans"],
            "CFBundleExecutable": "tickeys-redux",
            "CFBundleIconFile": "tickeys",
            "CFBundleIdentifier": BUNDLE_ID,
            "CFBundleInfoDictionaryVersion": "6.0",
            "CFBundleName": APP_NAME,
            "CFBundlePackageType": "APPL",
            "CFBundleShortVersionString": stamp["version"],
            "CFBundleVersion": stamp["version"],
            "LSMinimumSystemVersion": "13.0",
            "LSUIElement": True,
            "NSHighResolutionCapable": True,
            "NSSupportsAutomaticGraphicsSwitching": True,
            "TickeysBuildCommit": stamp["commit"],
            "TickeysBuildDirty": stamp["dirty"],
        }
        with (contents / "Info.plist").open("wb") as stream:
            plistlib.dump(info, stream, sort_keys=True)
        identity = RELEASE_CERT_SHA1 if official else os.environ.get("TICKEYS_SIGNING_IDENTITY", "-")
        signing = ["codesign", "--force", "--deep", "--sign", identity]
        if identity != "-":
            signing += ["--options", "runtime", "--timestamp=none"]
        run(*signing, bundle)
        verify_bundle(bundle, stamp["version"], stamp["commit"], official=official)
        if official and release_source(ROOT) != stamp:
            raise ValueError("source/tag changed during signing or resource assembly")
        # Keep previous local or signed apps intact. Each successful build gets
        # its own directory; failures never remove an earlier build.
        mode = "official" if official else "local"
        output = Path(tempfile.mkdtemp(prefix=f'v{stamp["version"]}-{stamp["commit"][:8]}-{profile}-{mode}-', dir=builds))
        destination = output / f"{APP_NAME}.app"
        shutil.move(str(bundle), destination)
    print(f"Verified {'official' if official else 'local'} app: {destination}", flush=True)
    return destination, stamp


def create_dmg(bundle, destination, stamp, official=False):
    destination = Path(destination).absolute()
    if os.path.lexists(destination):
        raise ValueError(f"refusing to overwrite existing disk image: {destination}")
    tool = shutil.which("create-dmg")
    if not tool:
        raise ValueError("create-dmg is required; install it with brew install create-dmg")
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="tickeys-dmg-") as folder:
        folder = Path(folder)
        source = folder / "source"
        source.mkdir()
        run("ditto", bundle, source / bundle.name)
        image = folder / "Tickeys.dmg"
        run(tool, "--volname", APP_NAME, "--background", ROOT / "assets/dmg_bg.png",
            "--window-size", "660", "480", "--icon-size", "96", "--icon", bundle.name,
            "175", "240", "--app-drop-link", "505", "240", image, source)
        mount = folder / "mounted"
        run("hdiutil", "attach", image, "-nobrowse", "-readonly", "-mountpoint", mount)
        try:
            if not (mount / "Applications").is_symlink() or os.readlink(mount / "Applications") != "/Applications":
                raise ValueError("DMG has no valid Applications drop link")
            for name in (".background/dmg_bg.png", ".DS_Store"):
                if not (mount / name).is_file():
                    raise ValueError(f"DMG layout is missing {name}")
            verify_bundle(mount / bundle.name, stamp["version"], stamp["commit"], official=official)
        finally:
            run("hdiutil", "detach", mount)
        publish_artifact(image, destination)
    print(f"Verified disk image: {destination}", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--debug", action="store_true")
    parser.add_argument("--official", action="store_true")
    parser.add_argument("--dmg", "-d", nargs="?", const="")
    args = parser.parse_args()
    try:
        bundle, stamp = build_app(args.official, args.debug)
        if args.dmg is not None:
            path = args.dmg or str(ROOT / f'dist/Tickeys.Redux.v{stamp["version"]}.local.dmg')
            create_dmg(bundle, path, stamp, args.official)
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"Build failed: {error}\n")


if __name__ == "__main__":
    main()

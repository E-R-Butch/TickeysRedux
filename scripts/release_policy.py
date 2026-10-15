#!/usr/bin/env python3
"""Shared source and bundle checks for local builds and official releases."""

import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import subprocess
import tempfile

BUNDLE_ID = "com.sinclair.tickeys-redux"
RELEASE_CERT_SHA1 = "C35F68EA9702560D619BDA339ED7E735511F6A85"
RELEASE_CERT_SHA256 = "152B9E7DCA7778BB36909EC5DB85CEF96430CB26135333084F526CC13A644839"


def command(*args, cwd=None):
    return subprocess.check_output(args, cwd=cwd, text=True, stderr=subprocess.STDOUT).strip()


def project_version(root):
    match = re.search(r'^version\s*=\s*"([^"]+)"', (Path(root) / "Cargo.toml").read_text(), re.M)
    if not match:
        raise ValueError("Cargo.toml has no package version")
    return match.group(1)


def source_stamp(root):
    return {
        "version": project_version(root),
        "commit": command("git", "rev-parse", "HEAD", cwd=root),
        "dirty": bool(command("git", "status", "--porcelain", "--untracked-files=all", cwd=root)),
        "target": "aarch64-apple-darwin",
        "minimum_macos": "13.0",
    }


def validate_signing_environment(environment):
    # These environment variables may select a local identity in package_app.sh,
    # but they cannot change the identity of an official release.
    for name in ("TICKEYS_SIGNING_IDENTITY", "TICKEYS_EXPECTED_SIGNING_CERT_SHA1"):
        value = environment.get(name)
        if value is not None and value.upper() != RELEASE_CERT_SHA1:
            raise ValueError(f"{name} cannot override the pinned official certificate")


def release_source(root, environment=None):
    validate_signing_environment(os.environ if environment is None else environment)
    stamp = source_stamp(root)
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", stamp["version"]):
        raise ValueError("official releases require a stable x.y.z package version")
    if stamp["dirty"]:
        raise ValueError("official release requires a clean worktree, including untracked files")
    tag = f'v{stamp["version"]}'
    ref = f"refs/tags/{tag}"
    try:
        kind = command("git", "cat-file", "-t", ref, cwd=root)
        tagged_commit = command("git", "rev-parse", f"{ref}^{{commit}}", cwd=root)
    except subprocess.CalledProcessError as error:
        raise ValueError(f"missing release tag {tag}") from error
    if kind != "tag":
        raise ValueError(f"{tag} must be an annotated release tag")
    if tagged_commit != stamp["commit"]:
        raise ValueError(f"{tag} does not identify the current build commit")
    stamp["tag"] = tag
    return stamp


def verify_bundle_metadata(bundle, expected_version, expected_commit=None, require_clean=False):
    bundle = Path(bundle)
    contents = bundle / "Contents"
    resources = contents / "Resources"
    with (contents / "Info.plist").open("rb") as stream:
        info = plistlib.load(stream)
    for key in ("CFBundleShortVersionString", "CFBundleVersion"):
        if info.get(key) != expected_version:
            raise ValueError(f"{key} does not match {expected_version}")
    if info.get("CFBundleIdentifier") != BUNDLE_ID:
        raise ValueError("incorrect bundle identifier")
    if info.get("CFBundleDevelopmentRegion") != "en" or info.get("CFBundleLocalizations") != ["en", "zh-Hans"]:
        raise ValueError("bundle localization metadata must match English and Simplified Chinese resources")
    if info.get("LSMinimumSystemVersion") != "13.0":
        raise ValueError("incorrect minimum macOS version")
    required = ["LICENSE", "THIRD_PARTY_NOTICES.md", "AUDIO_ASSETS.md", "licenses/dependencies.json",
                "licenses/rust-runtime/COPYRIGHT.html", "data/schemes.json", "data/audio-manifest.json"]
    required += [f"{language}.lproj/Localizable.strings" for language in ("Base", "en", "zh-Hans")]
    for name in required:
        if not (resources / name).is_file():
            raise ValueError(f"bundle is missing {name}")
    if (resources / "Base.lproj/Localizable.strings").read_bytes() != (resources / "en.lproj/Localizable.strings").read_bytes():
        raise ValueError("Base and English localization must be identical")
    stamp = json.loads((resources / "build-info.json").read_text())
    if stamp.get("version") != expected_version or stamp.get("target") != "aarch64-apple-darwin":
        raise ValueError("build metadata does not match version/target")
    if not re.fullmatch(r"[0-9a-f]{40}", str(stamp.get("commit", ""))):
        raise ValueError("build metadata must contain the source commit")
    if expected_commit is not None and stamp["commit"] != expected_commit:
        raise ValueError("bundle was built from a different commit")
    if require_clean and stamp.get("dirty") is not False:
        raise ValueError("official bundle was built from a dirty worktree")
    if info.get("TickeysBuildCommit") != stamp["commit"] or info.get("TickeysBuildDirty") != stamp.get("dirty"):
        raise ValueError("Info.plist and build-info.json disagree")
    dependencies = json.loads((resources / "licenses/dependencies.json").read_text())
    if not dependencies:
        raise ValueError("dependency license inventory is empty")
    for dependency in dependencies:
        if not dependency.get("files"):
            raise ValueError(f"missing license text for {dependency.get('name')}")
        for entry in dependency["files"]:
            path = resources / "licenses" / entry["path"]
            if not path.resolve().is_relative_to((resources / "licenses").resolve()):
                raise ValueError("license path escapes bundle")
            if hashlib.sha256(path.read_bytes()).hexdigest() != entry["sha256"]:
                raise ValueError(f"license text mismatch: {entry['path']}")
    data = resources / "data"
    schemes = json.loads((data / "schemes.json").read_text())
    referenced = {f"{scheme['name']}/{file}" for scheme in schemes for file in scheme["files"]}
    audio = json.loads((data / "audio-manifest.json").read_text())["assets"]
    if not referenced or {entry["path"] for entry in audio} != referenced:
        raise ValueError("audio manifest does not cover all scheme files")
    for entry in audio:
        path = data / entry["path"]
        if not path.resolve().is_relative_to(data.resolve()):
            raise ValueError("audio path escapes bundle")
        if hashlib.sha256(path.read_bytes()).hexdigest() != entry["sha256"]:
            raise ValueError(f"audio file mismatch: {entry['path']}")
    return stamp


def verify_bundle(bundle, version, commit=None, official=False):
    stamp = verify_bundle_metadata(bundle, version, commit, require_clean=official)
    command("codesign", "--verify", "--deep", "--strict", str(bundle))
    binary = str(Path(bundle) / "Contents/MacOS/tickeys-redux")
    if command("lipo", "-archs", binary) != "arm64":
        raise ValueError("bundle must contain only arm64 code")
    if not re.search(r"\bminos 13\.0\b", command("xcrun", "vtool", "-show-build", binary)):
        raise ValueError("binary deployment target must be macOS 13.0")
    if "/ServiceManagement.framework/" not in command("otool", "-L", binary):
        raise ValueError("binary does not load ServiceManagement")
    if official:
        with tempfile.TemporaryDirectory(prefix="tickeys-cert-") as folder:
            prefix = str(Path(folder) / "certificate")
            command("codesign", "-d", "--extract-certificates", prefix, str(bundle))
            certificate = Path(prefix + "0").read_bytes()
            if hashlib.sha1(certificate).hexdigest().upper() != RELEASE_CERT_SHA1:
                raise ValueError("bundle is not signed with the official certificate")
            if hashlib.sha256(certificate).hexdigest().upper() != RELEASE_CERT_SHA256:
                raise ValueError("official certificate SHA-256 mismatch")
        requirement = command("codesign", "-d", "--requirements", "-", str(bundle)).lower()
        if f'identifier "{BUNDLE_ID}"' not in requirement or f'certificate root = h"{RELEASE_CERT_SHA1.lower()}"' not in requirement:
            raise ValueError("designated requirement is not bound to the pinned identity")
    return stamp

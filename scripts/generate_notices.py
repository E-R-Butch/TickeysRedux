#!/usr/bin/env python3
"""Generate/check bundled dependency notices from the offline, locked macOS graph.

Run cargo fetch once before using this script on a new machine. This generator
never fetches license text or resolves unlocked dependencies over the network.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import io
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
import tarfile


ROOT = Path(__file__).resolve().parents[1]
LICENSES = ROOT / "licenses"
TARGET = "aarch64-apple-darwin"
LICENSE_NAME = re.compile(r"^(?:LICEN[CS]E|COPYING|NOTICE|UNLICENSE)(?:$|[-_.])", re.I)
CODE_SUFFIXES = {".rs", ".c", ".h", ".cpp", ".py", ".js", ".ts"}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def json_bytes(value):
    return (json.dumps(value, ensure_ascii=False, indent=2) + "\n").encode("utf-8")


def safe_path(base, relative):
    path = PurePosixPath(relative)
    if path.is_absolute() or ".." in path.parts or not path.parts:
        raise ValueError(f"Unsafe notice path: {relative}")
    result = base.joinpath(*path.parts)
    if result.is_symlink() or not result.resolve().is_relative_to(base.resolve()):
        raise ValueError(f"Notice path escapes its source directory: {relative}")
    return result


def load_metadata(metadata_path):
    if metadata_path:
        return json.loads(Path(metadata_path).read_text(encoding="utf-8"))
    command = [
        "cargo", "metadata", "--offline", "--locked", "--format-version", "1",
        "--filter-platform", TARGET,
    ]
    result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
    if result.returncode:
        raise ValueError("Offline cargo metadata failed:\n" + result.stderr.strip())
    return json.loads(result.stdout)


def target_packages(metadata):
    resolve = metadata.get("resolve") or {}
    root = resolve.get("root")
    if not root:
        raise ValueError("Cargo metadata must describe this package with its resolved graph")
    nodes = {node["id"]: node for node in resolve["nodes"]}
    pending, selected = [root], {root}
    while pending:
        for dep in nodes[pending.pop()]["deps"]:
            if any(kind["kind"] in (None, "build") for kind in dep["dep_kinds"]):
                if dep["pkg"] not in selected:
                    selected.add(dep["pkg"])
                    pending.append(dep["pkg"])
    packages = [p for p in metadata["packages"] if p["id"] in selected and p["id"] != root]
    if not packages:
        raise ValueError("Refusing to generate an empty dependency notice set")
    return sorted(packages, key=lambda p: (p["name"], p["version"]))


def locked_checksums():
    # Cargo emits these four scalar fields in each registry package stanza.
    # Package names, versions and registry URLs do not need TOML escape decoding.
    entries = {}
    for block in (ROOT / "Cargo.lock").read_text(encoding="utf-8").split("[[package]]")[1:]:
        fields = dict(re.findall(r'^(name|version|source|checksum) = "([^"\\]+)"$', block, re.M))
        if "source" in fields:
            key = (fields["name"], fields["version"], fields["source"])
            entries[key] = fields.get("checksum")
    return entries


def load_upstream():
    lock = json.loads((LICENSES / "upstream/sources.json").read_text(encoding="utf-8"))
    if lock.get("schema_version") != 1:
        raise ValueError("Unsupported upstream license lock schema")
    contents = {}
    for relative, record in lock["files"].items():
        if not relative.startswith("upstream/") or not record["url"].startswith("https://"):
            raise ValueError(f"Invalid upstream license source: {relative}")
        data = safe_path(LICENSES, relative).read_bytes()
        if not data or digest(data) != record["sha256"]:
            raise ValueError(f"Pinned upstream license was changed: {relative}")
        contents[relative] = data
    packages = {(p["name"], p["version"]): p for p in lock["packages"]}
    if len(packages) != len(lock["packages"]):
        raise ValueError("Duplicate package in upstream license lock")
    return packages, contents


def verified_crate_file(directory, relative, checksums):
    data = safe_path(directory, relative).read_bytes()
    expected = checksums.get(relative)
    if not expected or digest(data) != expected:
        raise ValueError(f"Crate source checksum mismatch: {directory.name}/{relative}")
    if not data.strip():
        raise ValueError(f"Empty license source: {directory.name}/{relative}")
    return data


def source_notices(package, lock_checksums, upstream, upstream_contents):
    key = (package["name"], package["version"], package["source"])
    checksum = lock_checksums.get(key)
    if not checksum:
        raise ValueError(f"Dependency is not pinned to a registry checksum in Cargo.lock: {key}")
    directory = Path(package["manifest_path"]).parent
    archive = directory.parents[2] / "cache" / directory.parent.name / (directory.name + ".crate")
    archive_data = archive.read_bytes()
    if digest(archive_data) != checksum:
        raise ValueError(f"Cached crate archive does not match Cargo.lock: {directory.name}")
    checksums = {}
    prefix = directory.name + "/"
    # Read only license/provenance members, without unpacking the archive. These
    # digests also detect local edits to the extracted Cargo source directory.
    with tarfile.open(fileobj=io.BytesIO(archive_data), mode="r:gz") as crate:
        for member in crate.getmembers():
            if not member.name.startswith(prefix):
                raise ValueError(f"Unexpected crate archive path: {member.name}")
            relative = member.name[len(prefix):]
            name = PurePosixPath(relative)
            relevant = (
                relative == ".cargo_vcs_info.json" or relative == package.get("license_file")
                or (LICENSE_NAME.match(name.name) and name.suffix.lower() not in CODE_SUFFIXES)
            )
            if relevant:
                if not member.isfile():
                    raise ValueError(f"Non-file license member in crate: {member.name}")
                checksums[relative] = digest(crate.extractfile(member).read())
    notices = {}
    for relative in sorted(checksums):
        path = PurePosixPath(relative)
        if LICENSE_NAME.match(path.name) and path.suffix.lower() not in CODE_SUFFIXES:
            notices[relative] = verified_crate_file(directory, relative, checksums)
    if package.get("license_file"):
        relative = package["license_file"]
        notices[relative] = verified_crate_file(directory, relative, checksums)
    if notices:
        return notices

    supplement = upstream.get((package["name"], package["version"]))
    if not supplement or supplement["license"] != package["license"]:
        raise ValueError(f"Missing reviewed upstream license material for {directory.name}")
    vcs = json.loads(verified_crate_file(directory, ".cargo_vcs_info.json", checksums))
    if (vcs["git"]["sha1"] != supplement["revision"]
            or vcs.get("path_in_vcs", "") != supplement["path_in_vcs"]):
        raise ValueError(f"Upstream license revision differs from published crate: {directory.name}")
    for dest, source in sorted(supplement["files"].items()):
        safe_path(LICENSES, dest)
        if source not in upstream_contents:
            raise ValueError(f"Unpinned supplemental license source: {source}")
        notices[dest] = upstream_contents[source]

    if package["name"].startswith("objc2") or package["name"] == "dispatch2":
        attribution = [
            "Supplemental license provenance (generated by Tickeys Redux)", "",
            f"Package: {package['name']} {package['version']}",
            f"Declared license: {package['license']}",
            f"Upstream repository: {supplement['repository']}",
            f"Published source revision: {supplement['revision']}",
            f"Path in upstream repository: {supplement['path_in_vcs']}", "",
            "The published crate omits license files. LICENSE.md is copied unchanged",
            "from its exact source revision. The *-REFERENCE.txt files reproduce the",
            "official license texts linked by that declaration. These references do",
            "not supply an additional copyright grant or identify new rightsholders.",
            "The upstream declaration supplies no separate copyright-holder notice;",
            "none has been inferred. MIT template placeholders and the Zlib page's",
            "own attribution are preserved, not reassigned to this crate.", "",
            "Authors listed in published Cargo metadata (not inferred copyright holders):",
        ]
        attribution.extend(package["authors"] or ["No authors field supplied for this crate."])
        attribution.extend(["", "Source URLs and SHA-256 values: ../../upstream/sources.json", ""])
        notices["SOURCE-NOTICE.txt"] = "\n".join(attribution).encode("utf-8")
    return notices


def render_notice(entries):
    lines = [
        "# Third-party notices", "",
        "Tickeys Redux includes the following dependencies and sound assets.",
        "The project's own license is in [LICENSE](LICENSE).", "",
        "## Dependency licenses", "",
        f"This inventory covers the {len(entries)} transitive normal and build dependencies",
        f"resolved by Cargo.lock for `{TARGET}`. Build dependencies are included",
        "for completeness; this table does not claim that all are linked into the executable.",
        "License expressions below preserve the package metadata, including alternative",
        "licenses and additional conditions. Full supplied texts, copyright notices and",
        "third-party notices are retained in the linked files.", "",
        "| Dependency | Declared license | License and notice files |",
        "| --- | --- | --- |",
    ]
    for entry in entries:
        url = f"https://crates.io/crates/{entry['name']}/{entry['version']}"
        links = ", ".join(
            f"[{PurePosixPath(file['path']).name}](licenses/{file['path']})"
            for file in entry["files"]
        )
        lines.append(f"| [{entry['name']} {entry['version']}]({url}) | {entry['license']} | {links} |")
    lines.extend([
        "", "### Source provenance", "",
        "[dependencies.json](licenses/dependencies.json) records the exact package versions,",
        "registry sources and SHA-256 digest of every bundled notice. Texts supplied by",
        "crates are checked against the fetched crate's Cargo checksums. Supplemental",
        "texts for crates which omitted license files are pinned in",
        "[upstream/sources.json](licenses/upstream/sources.json).", "",
        "The objc2 family declares its licenses in an upstream LICENSE.md rather than",
        "shipping complete license texts in each crate. We preserve that declaration,",
        "including its Apple SDK licensing discussion, and the official license texts",
        "it links. No missing copyright holder or year has been invented. The included",
        "SOURCE-NOTICE files distinguish package author metadata from copyright notices.", "",
        "## Rust standard library and runtime", "",
        "The standard library and runtime supplied by the actual build toolchain are",
        "documented in [Rust copyright and notices](licenses/rust-runtime/COPYRIGHT.html).",
        "The bundle also contains that toolchain's full license texts under",
        "[licenses/rust-runtime/licenses](licenses/rust-runtime/licenses/). These files",
        "are copied from the Rust sysroot during packaging, and the rustc/cargo versions",
        "are recorded in build-info.json. They are separate from the Cargo dependency",
        "manifest above because their contents depend on the selected build toolchain.", "",
        "## Sound assets", "",
        "Sound recordings are licensed separately from the Rust program. The origin and",
        "licensing evidence for the files actually shipped in this version is recorded in",
        "[AUDIO_ASSETS.md](AUDIO_ASSETS.md). Each installed scheme carries its own notice:", "",
    ])
    # Use the shipped scheme inventory so a newly added pack cannot be silently
    # omitted. Drum retains its historical combined readme/license filename.
    audio_root = ROOT / "assets/data"
    for scheme in json.loads((audio_root / "schemes.json").read_text(encoding="utf-8")):
        name = scheme["name"]
        filename = "_readme_and_license.txt" if name == "drum" else "license.txt"
        relative = f"{name}/{filename}"
        notice = safe_path(audio_root, relative)
        if not notice.is_file() or not notice.read_bytes().strip():
            raise ValueError(f"Missing sound-asset notice: {relative}")
        lines.append(f"- [{name}](data/{relative})")
    lines.extend([
        "", "A software repository's license is not treated as proof of permission for",
        "third-party recordings. Asset provenance and any replacement sounds are",
        "documented separately; this dependency inventory makes no additional audio",
        "licensing claims.", "",
        "## Reproducing this inventory", "",
        "After fetching the locked Cargo dependencies, run",
        "`python3 scripts/generate_notices.py` from the source checkout.",
        "`python3 scripts/generate_notices.py --check` verifies cached source text digests",
        "and checks that the manifest, every generated notice and this document agree.",
        "Both commands use offline, locked Cargo metadata and never download license text.", "",
    ])
    return "\n".join(lines).encode("utf-8")


def expected_outputs(metadata):
    upstream, upstream_contents = load_upstream()
    lock_checksums = locked_checksums()
    entries, outputs = [], {}
    for package in target_packages(metadata):
        if not package.get("license"):
            raise ValueError(f"Missing declared dependency license: {package['name']}")
        notices = source_notices(package, lock_checksums, upstream, upstream_contents)
        if not notices:
            raise ValueError(f"No license text for {package['name']}")
        files = []
        for relative, data in sorted(notices.items()):
            target = f"dependencies/{package['name']}-{package['version']}/{relative}"
            outputs[safe_path(LICENSES, target)] = data
            files.append({"path": target, "sha256": digest(data)})
        entries.append({
            "name": package["name"], "version": package["version"],
            "license": package["license"], "source": package["source"], "files": files,
        })
    outputs[LICENSES / "dependencies.json"] = json_bytes(entries)
    outputs[ROOT / "THIRD_PARTY_NOTICES.md"] = render_notice(entries)
    return entries, outputs


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Check without changing files")
    parser.add_argument("--metadata", help=f"Use saved cargo metadata filtered for {TARGET}")
    args = parser.parse_args()
    try:
        entries, outputs = expected_outputs(load_metadata(args.metadata))
        existing = {path for path in (LICENSES / "dependencies").rglob("*") if path.is_file()}
        stale = existing - set(outputs)
        changed = [path for path, data in outputs.items() if not path.is_file() or path.read_bytes() != data]
        if args.check:
            if stale or changed:
                names = sorted(str(path.relative_to(ROOT)) for path in stale | set(changed))
                raise ValueError("Dependency notices need regeneration:\n" + "\n".join(names))
        else:
            for path, data in outputs.items():
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(data)
            for path in stale:
                safe_path(LICENSES, str(path.relative_to(LICENSES))).unlink()
        count = sum(len(entry["files"]) for entry in entries)
        print(f"{'Verified' if args.check else 'Generated'} {len(entries)} dependency notices ({count} files), offline for {TARGET}.")
        return 0
    except (OSError, ValueError, KeyError, TypeError, tarfile.TarError) as error:
        print(f"Notice generation failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())

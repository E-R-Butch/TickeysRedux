"""Publish complete files atomically without replacing any existing destination."""

import os
from pathlib import Path
import shutil
import tempfile


def publish_artifact(source, destination):
    """Stage on the destination filesystem, then claim its name without clobbering.

    Unlike rename/move, link() fails if any file, directory or symlink already
    occupies the name, even if it appeared after packaging began. Unsupported
    filesystems fail closed; there is no overwrite-capable fallback. Only our
    private staging file is removed, never an existing or published artifact.
    """
    source, destination = Path(source), Path(destination)
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(prefix=".tickeys-artifact-", dir=destination.parent,
                                     delete=False) as staged:
        staging_path = Path(staged.name)
        try:
            with source.open("rb") as stream:
                shutil.copyfileobj(stream, staged)
            staged.flush()
            os.fsync(staged.fileno())
            shutil.copymode(source, staging_path)
            os.link(staging_path, destination)
        finally:
            staging_path.unlink()

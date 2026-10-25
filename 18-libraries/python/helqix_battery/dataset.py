"""Pinned public dataset registry and checksum-verified download.

Data files are never committed to git. They are fetched from the published Zenodo record
and accepted only if their MD5 matches the value published by Zenodo.
"""
from __future__ import annotations

import hashlib
import os
import tempfile
import urllib.request
from pathlib import Path
from typing import Callable, Mapping

DATASET = {
    "doi": "10.5281/zenodo.4032561",
    "title": 'Experimental data for "Development of Experimental Techniques for Parameterization of Multi-scale Lithium-ion Battery Models"',
    "authors": "Chen, Brosa Planella, O'Regan, Gastol, Widanage, Kendrick",
    "licence": "CC-BY-4.0",
    "url_template": "https://zenodo.org/records/4032561/files/{name}?download=1",
    "files": {
        "LGM50_cell02.csv": "0bbf959cbfc367296ec69d7a74d2f5c1",
        "LGM50_cell03.csv": "a09cbd8eff0af55dfe9bc8728fa89908",
        "LGM50_cell04.csv": "f17615476283d2e15087165697ff9d7c",
    },
    "reference": "Chen et al. 2020, J. Electrochem. Soc. 167 080534, doi:10.1149/1945-7111/ab9050",
}


class DatasetIntegrityError(RuntimeError):
    """A data file is missing or does not match its published checksum."""


def _digest(path: Path, algorithm: str) -> str:
    h = hashlib.new(algorithm)
    with open(path, "rb") as handle:
        for block in iter(lambda: handle.read(1 << 16), b""):
            h.update(block)
    return h.hexdigest()


def file_md5(path: Path) -> str:
    return _digest(path, "md5")


def file_sha256(path: Path) -> str:
    return _digest(path, "sha256")


def verify_file(path: Path, expected_md5: str) -> None:
    if not Path(path).is_file():
        raise DatasetIntegrityError(f"{path} is missing")
    actual = file_md5(Path(path))
    if actual != expected_md5:
        raise DatasetIntegrityError(f"{Path(path).name}: MD5 {actual} does not match published {expected_md5}")


def fetch(dest_dir: Path, files: Mapping[str, str] | None = None, opener: Callable = urllib.request.urlopen) -> dict[str, Path]:
    """Downloads each file (or reuses a verified local copy). A corrupt download is deleted."""
    files = dict(DATASET["files"] if files is None else files)
    dest_dir = Path(dest_dir)
    dest_dir.mkdir(parents=True, exist_ok=True)
    paths = {}
    for name, md5 in files.items():
        target = dest_dir / name
        if target.is_file() and file_md5(target) == md5:
            paths[name] = target
            continue
        fd, tmp = tempfile.mkstemp(dir=dest_dir, suffix=".part")
        try:
            with os.fdopen(fd, "wb") as out, opener(DATASET["url_template"].format(name=name)) as response:
                for block in iter(lambda: response.read(1 << 16), b""):
                    out.write(block)
            verify_file(Path(tmp), md5)
            os.replace(tmp, target)
        finally:
            if os.path.exists(tmp):
                os.remove(tmp)
        paths[name] = target
    return paths

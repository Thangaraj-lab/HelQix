"""Package a package result as an electronics-result/0.1 bundle.

A bundle is the hand-off folder the platform reads: the package's
``results.json`` and CSVs copied byte for byte, plus ``manifest.json`` with a
SHA-256 per file and the labels and units the result actually used, plus a
plain-word ``README.md``.  The same source always produces the same bundle:
no timestamps, no paths outside the folder, no re-formatting of the payload.
"""

from __future__ import annotations

import hashlib
import json
import math
import shutil
from pathlib import Path

VERSION = "electronics-result/0.1"
BUNDLE_FILES = ("README.md", "manifest.json", "results.json", "series.csv", "values.csv")
LABEL_NOTES = {
    "GIVEN": "supplied in the input and used as is",
    "CALCULATED": "computed from the input by the package",
    "MEASURED": "observed on a simulated or recorded series",
    "REPORTED": "transcribed from a published table or document, not re-derived here",
    "UNKNOWN": "not known; the value is null and a reason is given",
    "TABLE": "read from a cited table row",
    "ASSUMPTION": "depends on a stated modelling assumption",
}


class BundleError(ValueError):
    """The source is not a result, or the output folder cannot hold a bundle."""


def _reject_non_finite(node, path: str = "") -> None:
    if isinstance(node, float):
        if not math.isfinite(node):
            raise BundleError(f"results.json has a non-finite number at {path or 'the top level'}")
        return
    if isinstance(node, dict):
        for key, value in node.items():
            _reject_non_finite(value, f"{path}.{key}".lstrip("."))
    elif isinstance(node, list):
        for index, value in enumerate(node):
            _reject_non_finite(value, f"{path}[{index}]")


def _load_results(source: Path) -> dict:
    path = source / "results.json"
    if not path.is_file():
        raise BundleError(f"source folder {source} has no results.json")
    try:
        body = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise BundleError(f"results.json is not valid JSON: {exc}") from exc
    if not isinstance(body, dict):
        raise BundleError("results.json must hold a JSON object")
    status = body.get("status")
    if status not in ("ok", "solver_error"):
        raise BundleError(f"results.json status must be 'ok' or 'solver_error', got {status!r}")
    solver = body.get("solver")
    if (
        not isinstance(solver, dict)
        or not isinstance(solver.get("name"), str)
        or not solver.get("name")
        or not isinstance(solver.get("version"), str)
        or not solver.get("version")
    ):
        raise BundleError("results.json needs a solver object with non-empty name and version")
    _reject_non_finite(body)
    if status == "ok" and not isinstance(body.get("values"), list):
        raise BundleError("results.json has no values list")
    return body


def _payload(body: dict) -> list[str]:
    names = []
    if body["status"] == "ok":
        names.append("values.csv")
        probes = body.get("probes")
        series = body.get("series_csv")
        if probes and not series:
            raise BundleError("results.json has probes but no series_csv")
        if series:
            if series != "series.csv":
                raise BundleError(f"results.json names {series!r}; the bundle only carries 'series.csv'")
            names.append("series.csv")
    return ["results.json"] + sorted(names)


def _prepare_out(out: Path) -> None:
    if out.exists():
        if not out.is_dir():
            raise BundleError(f"out path {out} is not a folder")
        foreign = sorted(p.name for p in out.iterdir() if p.name not in BUNDLE_FILES)
        if foreign:
            raise BundleError(
                f"out folder {out} is not fresh; it already contains {foreign}; choose an empty folder"
            )
        for name in BUNDLE_FILES:
            path = out / name
            if path.exists():
                path.unlink()
    else:
        try:
            out.mkdir(parents=True)
        except OSError as exc:
            raise BundleError(f"cannot create out folder {out}: {exc.strerror or exc}") from exc


def _collect(body: dict) -> tuple[list[str], list[str]]:
    units: set[str] = set()
    labels: set[str] = set()

    def walk(node) -> None:
        if isinstance(node, dict):
            if isinstance(node.get("unit"), str) and node["unit"]:
                units.add(node["unit"])
            if isinstance(node.get("label"), str) and node["label"]:
                labels.add(node["label"])
            for value in node.values():
                walk(value)
        elif isinstance(node, list):
            for value in node:
                walk(value)

    walk(body)
    return sorted(labels), sorted(units)


def _readme(manifest: dict) -> str:
    source = manifest["source"]
    lines = [
        "# electronics-result/0.1 bundle",
        "",
        f"Produced by: {source['package']} {source['version']}",
        f"Status: {manifest['status']}",
    ]
    if manifest.get("analysis") is not None:
        lines.append(f"Analysis: {manifest['analysis']}")
    if manifest.get("validation_label") is not None:
        lines.append(f"Validation: {manifest['validation_label']}")
    lines += [
        "",
        "This folder is the electronics-result/0.1 hand-off: results.json is the",
        "package output unchanged, the CSVs carry the same tables, and manifest.json",
        "lists every payload file with its size and SHA-256 so a reader can prove it",
        "was not edited.",
    ]
    if manifest["status"] == "solver_error" and manifest.get("message"):
        lines += ["", f"Message: {manifest['message']}"]
    lines += ["", "## Files", ""]
    for entry in manifest["files"]:
        lines.append(f"- {entry['name']} — {entry['bytes']} bytes, sha256 {entry['sha256']}")
    lines += ["", "## Labels", ""]
    for label in manifest["labels"]:
        lines.append(f"- {label} — {LABEL_NOTES.get(label, 'defined by the producing package')}")
    lines += ["", "## Units", "", ", ".join(manifest["units"]) or "none", ""]
    return "\n".join(lines)


def build_bundle(source, out) -> dict:
    """Copy a package result into a fresh bundle folder; return the manifest."""
    src = Path(source)
    dst = Path(out)
    if not src.is_dir():
        raise BundleError(f"source folder {src} does not exist")
    if src.resolve() == dst.resolve():
        raise BundleError("out folder must be different from the source folder")
    body = _load_results(src)
    payload = _payload(body)
    for name in payload:
        if not (src / name).is_file():
            raise BundleError(f"results.json needs {name} but {src} has no {name}")

    _prepare_out(dst)
    files = []
    for name in payload:
        shutil.copyfile(src / name, dst / name)
        data = (dst / name).read_bytes()
        files.append({"name": name, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()})

    labels, units = _collect(body)
    manifest = {
        "version": VERSION,
        "source": {"package": body["solver"]["name"], "version": body["solver"]["version"]},
        "status": body["status"],
        "analysis": body.get("analysis"),
        "validation_label": body.get("validation_label"),
        "labels": labels,
        "units": units,
        "files": files,
    }
    if body["status"] == "solver_error":
        manifest["code"] = body.get("code")
        manifest["message"] = body.get("message")
    (dst / "manifest.json").write_text(
        json.dumps(manifest, indent=2, allow_nan=False) + "\n", encoding="utf-8"
    )
    (dst / "README.md").write_text(_readme(manifest), encoding="utf-8")
    return manifest

"""Download a small set of CC0 assets from Poly Haven into assets-local/ (git-ignored) and record them.

Poly Haven states: "Our assets are all licensed as CC0" (https://polyhaven.com/license, checked when this
script was written). Files are verified against the MD5 the API publishes and recorded, with source URL,
author and download date, in assets-local/ASSETS.json. Nothing downloaded is committed.

Usage: python -I scripts/fetch_cc0_assets.py [output-dir]
"""
import datetime
import hashlib
import json
import pathlib
import sys
import urllib.request

API = "https://api.polyhaven.com"
HDRIS = ["brown_photostudio_02", "blocky_photo_studio"]
MODELS = ["bench_vice_01", "barrel_03", "Drill_01"]


def get_json(url: str):
    with urllib.request.urlopen(urllib.request.Request(url, headers={"User-Agent": "helqix-asset-fetch/1"}), timeout=60) as r:
        return json.load(r)


def download(url: str, dest: pathlib.Path, md5: str | None) -> str:
    dest.parent.mkdir(parents=True, exist_ok=True)
    with urllib.request.urlopen(urllib.request.Request(url, headers={"User-Agent": "helqix-asset-fetch/1"}), timeout=300) as r:
        data = r.read()
    if md5 and hashlib.md5(data).hexdigest() != md5:  # noqa: S324 (the API publishes MD5; SHA-256 is recorded too)
        raise SystemExit(f"checksum mismatch for {url}")
    dest.write_bytes(data)
    return hashlib.sha256(data).hexdigest()


def main() -> None:
    out = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "assets-local")
    today = datetime.date.today().isoformat()
    record = []
    info = get_json(f"{API}/assets?t=all")
    for aid in HDRIS:
        f = get_json(f"{API}/files/{aid}")["hdri"]["2k"]["hdr"]
        sha = download(f["url"], out / "hdri" / f"{aid}_2k.hdr", f["md5"])
        record.append({"id": aid, "kind": "hdri", "file": f"hdri/{aid}_2k.hdr", "licence": "CC0", "source": f"https://polyhaven.com/a/{aid}",
                       "authors": info[aid]["authors"], "downloaded": today, "sha256": sha})
    for aid in MODELS:
        files = get_json(f"{API}/files/{aid}")["gltf"]["2k"]["gltf"]
        base = out / "models" / aid
        download(files["url"], base / f"{aid}_2k.gltf", files["md5"])
        for rel, inc in files.get("include", {}).items():
            download(inc["url"], base / rel, inc["md5"])
        record.append({"id": aid, "kind": "model", "file": f"models/{aid}/{aid}_2k.gltf", "licence": "CC0", "source": f"https://polyhaven.com/a/{aid}",
                       "authors": info[aid]["authors"], "downloaded": today, "note": "multi-file glTF; pack to one .glb before import"})
    (out / "ASSETS.json").write_text(json.dumps(record, indent=2) + "\n")
    print(f"{len(record)} assets recorded in {out / 'ASSETS.json'}")


if __name__ == "__main__":
    main()

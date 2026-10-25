"""K6 gates that need no Blender: the licence register and the content-hash cache key.
Run: python -m pytest -q tools/assets/tests"""
import json
import shutil
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(HERE))
from check_licenses import check_asset  # noqa: E402
from pipeline import cache_key  # noqa: E402

EXAMPLE = HERE.parents[1] / "examples" / "assets" / "example-bracket"


def copy(tmp_path):
    d = tmp_path / "example-bracket"
    shutil.copytree(EXAMPLE, d)
    return d


def test_the_committed_example_passes():
    assert check_asset(EXAMPLE) == []


def test_a_file_without_a_record_fails(tmp_path):
    d = copy(tmp_path)
    (d / "texture.png").write_bytes(b"\x89PNG")
    assert any("texture.png: no licence record" in p for p in check_asset(d))


def test_a_disallowed_or_incomplete_record_fails(tmp_path):
    d = copy(tmp_path)
    reg = json.loads((d / "LICENSES.json").read_text())
    reg["files"][0]["license"] = "CC-BY-NC-4.0"
    reg["files"][1]["date_checked"] = ""
    (d / "LICENSES.json").write_text(json.dumps(reg))
    problems = check_asset(d)
    assert any("CC-BY-NC-4.0 is not allowed" in p for p in problems)
    assert any("lacks ['date_checked']" in p for p in problems)


def test_a_missing_register_or_a_listed_missing_file_fails(tmp_path):
    d = copy(tmp_path)
    (d / "source.obj").unlink()
    assert any("listed but missing" in p for p in check_asset(d))
    (d / "LICENSES.json").unlink()
    assert check_asset(d) == ["example-bracket: no LICENSES.json"]


def test_the_cache_key_changes_with_the_source_tools_and_flags(tmp_path):
    d = copy(tmp_path)
    versions, flags = {"blender": "5.2.1", "gltf_transform": "4.5.1", "toktx": None}, {"ktx2": False, "hero": False}
    k = cache_key(d, versions, flags)
    assert cache_key(d, versions, flags) == k
    assert cache_key(d, versions | {"blender": "5.3.0"}, flags) != k
    assert cache_key(d, versions, flags | {"ktx2": True}) != k
    (d / "source.obj").write_text((d / "source.obj").read_text() + "# changed\n")
    assert cache_key(d, versions, flags) != k

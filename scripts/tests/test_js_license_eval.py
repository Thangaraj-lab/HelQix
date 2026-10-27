"""The JS licence gate judges whole SPDX expressions: AND needs every part, OR needs one."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from js_license_eval import allowed, evaluate  # noqa: E402


def test_simple_and_compound_expressions():
    assert allowed("MIT", "a")
    assert allowed("(MIT OR GPL-3.0)", "a"), "one acceptable alternative is enough"
    assert not allowed("MIT AND GPL-3.0", "a"), "AND needs every part allowed"
    assert not allowed("Apache-2.0 AND LGPL-3.0-or-later", "some-lib"), "LGPL only for listed packages"
    assert allowed("Apache-2.0 AND LGPL-3.0-or-later", "@img/sharp-linux-x64")
    assert allowed("(MIT AND Zlib) OR GPL-2.0", "a")
    assert not allowed("AGPL-3.0", "a")
    assert allowed("MIT*", "a"), "a guessed MIT is still MIT"


def test_evaluate_reports_every_failure():
    pkgs = {"ok@1": {"licenses": "MIT"}, "bad@1": {"licenses": "MIT AND GPL-3.0"}, "none@1": {"licenses": "UNKNOWN"},
            "list@1": {"licenses": ["MIT", "SSPL-1.0"]}, "@helqix/x@1": {"licenses": "UNLICENSED"}}
    failures = evaluate(pkgs)
    assert len(failures) == 3, failures
    assert any(f.startswith("bad@1") for f in failures) and any(f.startswith("none@1") for f in failures) and any(f.startswith("list@1") for f in failures)


def test_ofl_is_allowed_only_for_font_packages():
    assert allowed("OFL-1.1", "@fontsource/sora")
    assert not allowed("OFL-1.1", "some-library")
    assert not allowed("MIT AND OFL-1.1", "some-library")

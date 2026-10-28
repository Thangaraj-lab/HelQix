"""The secret scan finds planted secrets and leaves ordinary code alone."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from secret_scan import scan_text  # noqa: E402

BEGIN = "-----BEGIN " + "PRIVATE KEY-----"  # built in two parts so this file does not trip its own scan


def test_planted_secrets_are_found():
    assert scan_text("a", BEGIN)
    assert scan_text("a", "key = AKIA" + "ABCDEFGHIJKLMNOP")
    assert scan_text("a", "t = 'ghp_" + "a" * 36 + "'")
    assert scan_text("a", 'API_KEY = "' + "x" * 24 + '"')
    assert scan_text("a", "password: '" + "Ab1" * 8 + "'")


def test_ordinary_code_is_not_flagged():
    for line in ["const token = useToken();", "let api_key = std::env::var(\"HELQIX_API_KEY\");", "password: string;",
                 "HELQIX_API_KEY=stub-key-not-a-secret", 'secret = ""', "// the key is never in the bundle"]:
        assert not scan_text("a", line), line


def test_a_documented_test_value_can_be_allowed():
    assert not scan_text("a", 'API_KEY = "' + "x" * 24 + '"  # secret-scan: allow')

"""Validates the shared contract fixtures against the generated JSON Schemas (Draft 2020-12).
Run: python -m pytest -q 15-sdk/platform/contracts/tests_py"""
import json
from pathlib import Path

import jsonschema
import pytest

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = json.loads((ROOT / "fixtures" / "contracts.json").read_text(encoding="utf-8"))
TYPES = [k for k in FIXTURES if not k.startswith("_")]


def validator(name):
    schema = json.loads((ROOT / "schema" / f"{name}.schema.json").read_text(encoding="utf-8"))
    cls = jsonschema.validators.validator_for(schema)
    cls.check_schema(schema)
    return cls(schema)


@pytest.mark.parametrize("name", TYPES)
def test_valid_fixtures_pass_the_schema(name):
    v = validator(name)
    for i, doc in enumerate(FIXTURES[name]["valid"]):
        errors = list(v.iter_errors(doc))
        assert not errors, f"{name} valid[{i}]: {errors[0].message}"


@pytest.mark.parametrize("name", TYPES)
def test_invalid_fixtures_fail_the_schema(name):
    v = validator(name)
    for i, doc in enumerate(FIXTURES[name]["invalid"]):
        assert not v.is_valid(doc), f"{name} invalid[{i}] was accepted: {doc}"


def test_every_schema_file_has_fixtures_or_is_listed():
    names = {p.name.removesuffix(".schema.json") for p in (ROOT / "schema").glob("*.schema.json")}
    assert set(TYPES) <= names

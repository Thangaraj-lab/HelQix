"""Verify machine integration keys cannot bypass account project isolation.

Run against a throwaway gateway configured with HELQIX_INTEGRATION_API_KEY.
This script creates two disposable accounts and saved projects.
"""

from __future__ import annotations

import argparse
import http.cookiejar
import json
import urllib.error
import urllib.parse
import urllib.request
import uuid


def request(opener: urllib.request.OpenerDirector, base: str, path: str,
            method: str = "GET", body: dict | None = None,
            headers: dict[str, str] | None = None) -> tuple[int, dict | None]:
    data = None if body is None else json.dumps(body, allow_nan=False).encode("utf-8")
    req = urllib.request.Request(base + path, data=data, method=method,
        headers={"Content-Type": "application/json", **(headers or {})})
    try:
        with opener.open(req, timeout=30) as response:
            raw = response.read()
            return response.status, json.loads(raw) if raw else None
    except urllib.error.HTTPError as error:
        raw = error.read()
        return error.code, json.loads(raw) if raw else None


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def account(base: str, suffix: str, name: str) -> tuple[urllib.request.OpenerDirector, str]:
    jar = http.cookiejar.CookieJar()
    opener = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))
    status, signup = request(opener, base, "/v1/auth/signup", "POST", {
        "name": name,
        "email": f"{name.lower().replace(' ', '-')}-{suffix}@example.invalid",
        "password": f"Project-scope-{suffix}-Long!",
        "organization": "Local security validation",
    })
    require(status == 201 and signup and signup.get("user"), f"Could not create validation account: {signup}")
    status, project = request(opener, base, "/v1/projects", "POST", {
        "payload": {"name": f"{name} project", "engineering_reality": {
            "schema": "helqix.engineering-reality.v1", "name": f"{name} project",
            "domain": "validation", "components": [], "interfaces": [], "connections": [],
            "requirements": [], "model_bindings": [],
        }},
    })
    require(status == 201 and project, f"Could not create validation project: {project}")
    return opener, project["data"]["id"]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-url", default="http://127.0.0.1:8093")
    parser.add_argument("--api-key", required=True)
    args = parser.parse_args()
    base = args.base_url.rstrip("/")
    parsed = urllib.parse.urlparse(base)
    require(parsed.scheme == "http" and parsed.hostname in {"127.0.0.1", "localhost"},
            "Refusing to create test data outside a loopback gateway")

    key = {"Authorization": f"Bearer {args.api_key}"}
    machine = urllib.request.build_opener()
    status, _ = request(machine, base, "/v1/integrations/runs?limit=10&project_id=11111111-1111-1111-1111-111111111111", headers=key)
    require(status == 401, f"API key without an account session must not access project runs, got {status}")
    status, _ = request(machine, base, "/v1/integrations/helqix_reality/execute", "POST",
        {"operation": "validate_reality", "inputs": {"project_id": "11111111-1111-1111-1111-111111111111"}}, key)
    require(status == 401, f"API key without an account session must not execute against a project, got {status}")

    suffix = uuid.uuid4().hex
    owner, owner_project = account(base, suffix, "Validation owner")
    other, other_project = account(base, suffix, "Validation other")
    owner_key_headers = {**key}
    status, run_response = request(owner, base, "/v1/integrations/helqix_reality/execute", "POST",
        {"operation": "validate_reality", "inputs": {"project_id": owner_project}}, owner_key_headers)
    require(status == 200 and run_response and run_response.get("run"),
            f"Authenticated owner could not run structural validation with the integration key: {run_response}")
    run_id = run_response["run"]["id"]

    status, own_runs = request(owner, base, f"/v1/integrations/runs?limit=10&project_id={owner_project}", headers=key)
    require(status == 200 and any(run["id"] == run_id for run in own_runs["runs"]),
            f"Owner cannot read their own project run with the configured key: {own_runs}")
    status, hidden_runs = request(other, base, f"/v1/integrations/runs?limit=10&project_id={owner_project}", headers=key)
    require(status == 404, f"A different account must not read project runs: {hidden_runs}")

    status, machine_runs = request(machine, base, "/v1/integrations/runs?limit=100", headers=key)
    require(status == 200 and all(run["id"] != run_id and run.get("project_id") is None
                                  for run in machine_runs["runs"]),
            "Machine-only run history must contain ownerless runs only, not user project runs")
    print("PASS: API-key-only requests cannot access project runs or execute saved projects; authenticated owners retain access; cross-account reads are denied; machine-only history excludes user runs.")


if __name__ == "__main__":
    main()

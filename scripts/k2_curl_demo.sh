#!/usr/bin/env bash
# K2 demonstration against a running helqix-api: prints each curl command and its output.
# Usage: HELQIX_API_KEY=... scripts/k2_curl_demo.sh [base-url]   (default http://127.0.0.1:8080)
# Exits non-zero if any expected result is missing.
set -euo pipefail
BASE="${1:-http://127.0.0.1:8080}"
AUTH="Authorization: Bearer ${HELQIX_API_KEY:?set HELQIX_API_KEY}"
JSON="Content-Type: application/json"
# The command echo goes to stderr: req's stdout is captured and parsed as JSON.
show() { printf '\n$ %s\n' "$*" | sed "s/${HELQIX_API_KEY}/\$HELQIX_API_KEY/g" >&2; }
req() { local method="$1" path="$2" body="${3:-}"; show curl -s -X "$method" "$BASE$path" -H "\"$AUTH\"" ${body:+-H "\"$JSON\"" -d "'$body'"}
  if [ -n "$body" ]; then curl -s -X "$method" "$BASE$path" -H "$AUTH" -H "$JSON" -d "$body"; else curl -s -X "$method" "$BASE$path" -H "$AUTH"; fi; }
py() { python3 -c "import json,sys; d=json.load(sys.stdin); print($1)"; }

R=$(req POST /v1/projects '{"payload":{"name":"Untitled project"}}'); echo "$R"
PID=$(echo "$R" | py 'd["data"]["id"]')
R=$(req POST "/v1/projects/$PID/builds"); echo "$R"
BID=$(echo "$R" | py 'd["buildId"]')
OPS='{"ops":[{"op":"part.add","id":"base","name":"Base","geometry":{"kind":"primitive","shape":"box","dims":[{"v":0.2,"unit":"m","origin":"user_stated","status":"unverified"}]},"maturity":"user_defined","prov":{"kind":"user_stated"}},{"op":"part.add","id":"body","name":"Body","geometry":{"kind":"primitive","shape":"cylinder","dims":[{"v":0.1,"unit":"m","origin":"user_stated","status":"unverified"}]},"maturity":"user_defined","prov":{"kind":"user_stated"}},{"op":"port.add","part":"base","id":"out","kind":"electrical","direction":"out","unit":"V"},{"op":"port.add","part":"body","id":"in","kind":"electrical","direction":"in","unit":"A"},{"op":"connection.add","id":"c1","from":{"part":"base","port":"out"},"to":{"part":"body","port":"in"},"kind":"electrical"}]}'
R=$(req POST "/v1/builds/$BID/ops" "$OPS"); echo "$R" | py '"events: " + ", ".join(e["kind"] for e in d["events"]) + "; cursor " + str(d["cursor"]) + "; issues " + str([i["code"] for i in d["issues"]])'
R=$(req POST "/v1/builds/$BID/commit"); echo "$R"; echo "$R" | grep -q state_conflict
R=$(req POST "/v1/builds/$BID/undo"); echo "$R" | py '"events: " + ", ".join(e["kind"] for e in d["events"]) + "; cursor " + str(d["cursor"]) + "; issues " + str(d["issues"])'
R=$(req GET "/v1/builds/$BID"); echo "$R" | py '"status " + d["status"] + ", cursor " + str(d["cursor"]) + ", stored ops " + str(len(d["ops"])) + ", issues " + str(d["issues"])'
R=$(req POST "/v1/builds/$BID/commit"); echo "$R"; echo "$R" | grep -q '"label":"r1"'
show curl -s -N "$BASE/v1/builds/$BID/events" -H "\"$AUTH\""
curl -s -N --max-time 10 "$BASE/v1/builds/$BID/events" -H "$AUTH" | grep -E '^(id|event):' | paste - - | head -20
R=$(req GET /v1/sources); echo "$R"
printf '\nK2 demo: PASS\n'

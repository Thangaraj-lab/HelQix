#!/usr/bin/env bash
# K4 demonstration against a running helqix-api that has HELQIX_FRAME_STORE set and the
# reference run recorded (helqix-frames-record <store>). Prints each command and its result.
# Usage: HELQIX_API_KEY=... scripts/k4_frames_demo.sh [base-url] [run-id]
set -euo pipefail
BASE="${1:-http://127.0.0.1:8080}"
RUN="${2:-reference-drop}"
AUTH="Authorization: Bearer ${HELQIX_API_KEY:?set HELQIX_API_KEY}"
show() { printf '\n$ %s\n' "$*" >&2; }
get() { show curl -s "\"$BASE$1\"" -H '"Authorization: Bearer $HELQIX_API_KEY"'; curl -s "$BASE$1" -H "$AUTH"; }
py() { python3 -c "import json,sys; d=json.load(sys.stdin); print($1)"; }

get "/v1/runs/$RUN/frames?limit=2" | py '"frames file BLAKE3 " + d["framesBlake3"] + "; rows " + str(d["rows"]) + "; first two t = " + str([f["t"]["v"] for f in d["frames"]]) + "; next " + str(d["next"])'
get "/v1/runs/$RUN/frame?t=1.0" | py '"t=1.0 -> index " + str(d["frame"]["index"]) + ", y = " + repr(d["frame"]["state"]["y"]["v"]) + " m, origin " + d["frame"]["state"]["y"]["origin"] + ", chain link " + str(d["frame"]["stateSha256"])[:16]'
get "/v1/runs/$RUN/frame?t=1.0005" | py '"t=1.0005 -> index " + str(d["frame"]["index"]) + ", y = " + repr(d["frame"]["state"]["y"]["v"]) + " m, origin " + d["frame"]["state"]["y"]["origin"] + " (interpolated)"'
code=$(curl -s -o /dev/null -w "%{http_code}" "$BASE/v1/runs/$RUN/frame?t=99" -H "$AUTH"); show "curl ... /frame?t=99"; echo "outside the run: HTTP $code"; [ "$code" = 422 ]
printf '\nK4 demo: PASS\n'

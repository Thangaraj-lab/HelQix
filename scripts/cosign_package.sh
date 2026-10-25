#!/usr/bin/env bash
# K5: cosign (Sigstore, Apache-2.0) on top of a HELQIX evidence package, fully offline.
#   1. sign the package manifest (the manifest covers every file by hash and Merkle root)
#   2. attest the same in-toto predicate HELQIX writes (<package>.intoto.dsse.json)
#   3. verify both with the public key, offline, and check that a tampered manifest fails both.
# Nothing is uploaded: the signing config lists no transparency log and no timestamp service, and
# all HTTP(S) goes to a dead proxy, so any attempted network call fails instead of publishing.
# Usage: scripts/cosign_package.sh <cosign-binary> <package-dir> <work-dir>
# The key pair is generated in <work-dir> for this run (empty password); a real deployment key is a
# founder decision. Exit 0 only if every expected PASS and every expected FAIL happened.
set -euo pipefail
COSIGN="$1"; PKG="$2"; W="$3"
TYPE="https://helqix.dev/attestations/evidence-package/v1"
mkdir -p "$W"
export HTTPS_PROXY=http://127.0.0.1:9 HTTP_PROXY=http://127.0.0.1:9 https_proxy=http://127.0.0.1:9 http_proxy=http://127.0.0.1:9 NO_PROXY= no_proxy=
export COSIGN_PASSWORD=""
step() { printf '\n== %s\n' "$*"; }

step "cosign version"
"$COSIGN" version 2>&1 | grep -E "GitVersion|GitCommit" || true

step "ephemeral key pair"
(cd "$W" && "$COSIGN" generate-key-pair --output-key-prefix run >/dev/null)
ls "$W"/run.key "$W"/run.pub

cat > "$W/signing-config.json" <<'JSON'
{"mediaType": "application/vnd.dev.sigstore.signingconfig.v0.2+json", "caUrls": [], "oidcUrls": [], "rekorTlogUrls": [], "tsaUrls": [], "rekorTlogConfig": {"selector": "ANY"}, "tsaConfig": {"selector": "ANY"}}
JSON

step "sign the manifest (no transparency log)"
"$COSIGN" sign-blob --yes --key "$W/run.key" --signing-config "$W/signing-config.json" --bundle "$W/manifest.sigstore.json" "$PKG/manifest.json"

step "verify the manifest signature offline"
"$COSIGN" verify-blob --key "$W/run.pub" --bundle "$W/manifest.sigstore.json" --insecure-ignore-tlog=true "$PKG/manifest.json"

cp "$PKG/manifest.json" "$W/manifest.tampered.json"
sed -i 's/"NotValidated"/"Validated"/' "$W/manifest.tampered.json"
cmp -s "$PKG/manifest.json" "$W/manifest.tampered.json" && { echo "tamper did not change the file"; exit 1; }
step "a tampered manifest must fail"
if "$COSIGN" verify-blob --key "$W/run.pub" --bundle "$W/manifest.sigstore.json" --insecure-ignore-tlog=true "$W/manifest.tampered.json" 2>&1; then echo "UNEXPECTED: tampered manifest verified"; exit 1; fi
echo "expected failure: tampered manifest refused"

step "attest the HELQIX in-toto predicate with cosign"
python3 -I -c "import base64,json,sys; e=json.load(open(sys.argv[1])); print(json.dumps(json.loads(base64.b64decode(e['payload']))['predicate']))" "$PKG.intoto.dsse.json" > "$W/predicate.json"
"$COSIGN" attest-blob --yes --key "$W/run.key" --signing-config "$W/signing-config.json" --predicate "$W/predicate.json" --type "$TYPE" --bundle "$W/attestation.sigstore.json" "$PKG/manifest.json"

step "verify the attestation offline"
"$COSIGN" verify-blob-attestation --key "$W/run.pub" --bundle "$W/attestation.sigstore.json" --type "$TYPE" --insecure-ignore-tlog=true "$PKG/manifest.json"

step "the attestation must not verify for the tampered manifest"
if "$COSIGN" verify-blob-attestation --key "$W/run.pub" --bundle "$W/attestation.sigstore.json" --type "$TYPE" --insecure-ignore-tlog=true "$W/manifest.tampered.json" 2>&1; then echo "UNEXPECTED: attestation verified for a tampered manifest"; exit 1; fi
echo "expected failure: attestation refused for the tampered manifest"

printf '\ncosign on %s: PASS (signed, attested, verified offline; tampering refused; nothing uploaded)\n' "$PKG"

# Accounts and ownership

Two kinds of caller:

| Caller | How it signs in | What it sees |
|---|---|---|
| Service (operator, backend, the Studio dev proxy) | the shared `HELQIX_API_KEY` | everything, including records created before ownership existed |
| User | an OIDC access token (RS256) from your identity provider, for example Keycloak (Apache-2.0) | only what they created, plus builds, revisions and runs of their own projects |

## Turn user sign-in on

Set all three or none (a partial setting stops the API from starting, so a typo cannot leave sign-in silently off):

```bash
HELQIX_OIDC_ISSUER=https://sso.example.com/realms/helqix   # must equal the token's iss
HELQIX_OIDC_AUDIENCE=helqix-api                             # must be in the token's aud
HELQIX_OIDC_JWKS_FILE=/etc/helqix/jwks.json                 # the provider's public keys
```

Instead of the file, the API can read the keys from the provider's URL and follow a rotation: set `HELQIX_OIDC_JWKS_URL=https://sso.example.com/realms/helqix/protocol/openid-connect/certs` (exactly one of `_FILE` or `_URL`). The keys are fetched at start (an unreachable provider or an empty key set stops the API from starting), again when a token names a key id the API does not know (at most once per 30 s, so a flood of unknown ids costs the provider one request per interval) and every 15 minutes. The provider's current set replaces the old one, so a key it stopped publishing stops working; a failed fetch keeps the keys that worked. The URL must be https (http only for a loopback address). With a file, a rotation still needs a restart.

A token is accepted only if: signed with RS256 by a key in the file (matching `kid`), issuer and audience match, `exp` is in the future (30 s leeway), and `sub` is present. Symmetric algorithms and unknown keys are refused (tested).

## Rules

- A new resource is owned by its creator. Records from before this change have no owner and are visible to the service key only.
- A resource that is not yours answers **404**, so its existence is not revealed.
- Listing returns only your resources. Attaching a resource to someone else's project is refused.
- Builds, revisions, model runs and the run list of a project follow the project's owner.
- `GET /v1/runs` lists, for a user, only their runs: the ones they started (a branch counts) and the ones made from a project they own, even by the operator's key. A run id is a hash of the request, so two users who make the same request get the same run and are both recorded as its owner. The service key lists every run. Without PostgreSQL run metadata (files on one machine) a user lists none. Runs fetched by id (`/v1/runs/{id}/...`) are not owner-checked: run ids are hashes of the request, so someone who does not already know a model's project id and parameters cannot guess one. Treat run ids as capability links.
- A route the ownership guard does not know is **403** for users by default (`src/ownership.rs`, `need`).

## Sign in from the Studio

Build the Studio with the provider and its public client (no secret goes into a browser): `VITE_ENGINE=real VITE_OIDC_ISSUER=https://sso.example.com/realms/helqix VITE_OIDC_CLIENT_ID=helqix-studio`. The Studio then shows a sign-in screen, signs in with the authorization code flow with PKCE, keeps the tokens in the tab's session storage (readable by any script in the page; closing the tab ends the session), refreshes the access token 30 s before it expires and sends it to the API. The Studio's dev server forwards a user's own token untouched and adds the service key only to requests without one.

`deploy/keycloak/helqix-realm.json` is a realm for Keycloak (Apache-2.0): the public client `helqix-studio` (PKCE S256, no password grant), the test clients `helqix-test` (password grant, audience `helqix-api`) and `helqix-other` (audience `other-api`), and the users `alice` and `bob` with **test passwords that exist only in this file: never import it into a real installation**. Start one with `docker run -p 8081:8080 -v "$PWD/deploy/keycloak:/opt/keycloak/data/import:ro" quay.io/keycloak/keycloak:26.8.2 start-dev --import-realm`. The tests `tests/keycloak_api.rs` use it: two real users cannot see each other's projects, builds or revisions, and a token for another audience or with a damaged signature is refused.

## Not built yet

Roles beyond owner (sharing a project, read-only collaborators), organisations and tenants, token revocation lists, per-user rate limits, and a test with a second realm (a token from another issuer is refused by a unit test, not against a second Keycloak realm).

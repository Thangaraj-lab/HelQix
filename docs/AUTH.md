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

With Keycloak the key file is `curl $ISSUER/protocol/openid-connect/certs > jwks.json`. Keys are read at startup, so a key rotation needs a restart (a refresh from the provider's URL is not built yet).

A token is accepted only if: signed with RS256 by a key in the file (matching `kid`), issuer and audience match, `exp` is in the future (30 s leeway), and `sub` is present. Symmetric algorithms and unknown keys are refused (tested).

## Rules

- A new resource is owned by its creator. Records from before this change have no owner and are visible to the service key only.
- A resource that is not yours answers **404**, so its existence is not revealed.
- Listing returns only your resources. Attaching a resource to someone else's project is refused.
- Builds, revisions, model runs and the run list of a project follow the project's owner.
- `GET /v1/runs` (every stored run) is service-only for users (**403**). Runs fetched by id (`/v1/runs/{id}/...`) are not owner-checked: run ids are hashes of the request, so someone who does not already know a model's project id and parameters cannot guess one. Treat run ids as capability links.
- A route the ownership guard does not know is **403** for users by default (`src/ownership.rs`, `need`).

## Not built yet

Roles beyond owner (sharing a project, read-only collaborators), organisations and tenants, key refresh from the provider, token revocation lists, and per-user rate limits. The Studio does not sign users in yet: it talks to the API through the dev proxy with the service key.

import { challengeS256, randomString } from './pkce';

// The Studio's sign-in with the identity provider (OpenID Connect, authorization code with PKCE, a public client: there is no secret in the browser).
// Tokens live in the tab's sessionStorage, so a reload keeps the session and closing the tab ends it. That is readable by any script running in the
// page; the Studio loads only its own bundle, and the access token is short-lived, but this is the trade-off of a browser-only client.

export interface OidcConfig { issuer: string; clientId: string; scope: string }

export interface StorageLike { getItem(k: string): string | null; setItem(k: string, v: string): void; removeItem(k: string): void }

export interface SessionDeps {
  config: OidcConfig;
  storage: StorageLike;
  fetchImpl?: typeof fetch;
  now?: () => number;
  /** Where the Studio is now (read each time, because the address changes when the provider sends the user back). */
  location: () => { origin: string; href: string; pathname: string };
  navigate: (url: string) => void;
  replaceUrl: (url: string) => void;
}

interface Tokens { accessToken: string; refreshToken: string | null; expiresAt: number }

export interface Session {
  /** Reads the address for a return from the provider and trades the code for tokens. `none`: this page is not a return. */
  handleRedirect(): Promise<'none' | 'signed-in' | 'error'>;
  signIn(): Promise<void>;
  signOut(): Promise<void>;
  /** A valid access token (refreshed shortly before it expires, once for any number of callers), or null when nobody is signed in. */
  accessToken(): Promise<string | null>;
  user(): string | null;
  lastError(): string | null;
}

const K = { tokens: 'helqix.tokens', verifier: 'helqix.pkce.verifier', state: 'helqix.pkce.state' };
const REFRESH_BEFORE_MS = 30_000;

/** Display name from the access token's payload. Not verified here (the API verifies the token); it is only a label. */
function nameOf(accessToken: string): string | null {
  try {
    const payload = JSON.parse(atob(accessToken.split('.')[1].replace(/-/g, '+').replace(/_/g, '/')));
    return String(payload.preferred_username ?? payload.email ?? payload.sub ?? '') || null;
  } catch { return null; }
}

export function createSession(deps: SessionDeps): Session {
  const { config, storage } = deps;
  const f = deps.fetchImpl ?? fetch.bind(globalThis);
  const now = deps.now ?? Date.now;
  let tokens: Tokens | null = null;
  let error: string | null = null;
  let discovered: { authorization_endpoint: string; token_endpoint: string; end_session_endpoint?: string } | null = null;
  let refreshing: Promise<string | null> | null = null;
  let redirect: Promise<'none' | 'signed-in' | 'error'> | null = null;

  try { const raw = storage.getItem(K.tokens); if (raw) tokens = JSON.parse(raw) as Tokens; } catch { tokens = null; }

  const save = (t: Tokens | null) => { tokens = t; if (t) storage.setItem(K.tokens, JSON.stringify(t)); else storage.removeItem(K.tokens); };
  const redirectUri = () => deps.location().origin + '/';

  async function endpoints() {
    if (discovered) return discovered;
    const res = await f(`${config.issuer.replace(/\/$/, '')}/.well-known/openid-configuration`);
    if (!res.ok) throw new Error(`The identity provider's configuration could not be read (${res.status}).`);
    discovered = await res.json();
    return discovered!;
  }

  async function tokenRequest(params: Record<string, string>): Promise<Tokens> {
    const e = await endpoints();
    const res = await f(e.token_endpoint, { method: 'POST', headers: { 'content-type': 'application/x-www-form-urlencoded' }, body: new URLSearchParams({ client_id: config.clientId, ...params }).toString() });
    const body = await res.json().catch(() => ({}));
    if (!res.ok || typeof body.access_token !== 'string') throw new Error(String(body.error_description ?? body.error ?? `The identity provider answered ${res.status}.`));
    return { accessToken: body.access_token, refreshToken: typeof body.refresh_token === 'string' ? body.refresh_token : null, expiresAt: now() + Number(body.expires_in ?? 60) * 1000 };
  }

  /** A return from the provider is handled once, whoever asks and however often (React runs effects twice in development). */
  async function handleReturn(): Promise<'none' | 'signed-in' | 'error'> {
    const url = new URL(deps.location().href);
    const code = url.searchParams.get('code');
    const providerError = url.searchParams.get('error');
    if (providerError) { error = `The identity provider refused the sign-in: ${url.searchParams.get('error_description') ?? providerError} (${providerError}).`; deps.replaceUrl(deps.location().pathname); return 'error'; }
    if (!code) return 'none';
    const verifier = storage.getItem(K.verifier);
    const state = storage.getItem(K.state);
    storage.removeItem(K.verifier);
    storage.removeItem(K.state);
    deps.replaceUrl(deps.location().pathname);
    if (!verifier || !state || url.searchParams.get('state') !== state) { error = 'The sign-in answer did not match this browser tab, so it was ignored. Start the sign-in again.'; return 'error'; }
    try {
      save(await tokenRequest({ grant_type: 'authorization_code', code, code_verifier: verifier, redirect_uri: redirectUri() }));
      error = null;
      return 'signed-in';
    } catch (e) { error = e instanceof Error ? e.message : 'The sign-in could not be completed.'; return 'error'; }
  }

  const session: Session = {
    async signIn() {
      const e = await endpoints();
      const verifier = randomString(48);
      const state = randomString(16);
      storage.setItem(K.verifier, verifier);
      storage.setItem(K.state, state);
      const q = new URLSearchParams({ response_type: 'code', client_id: config.clientId, redirect_uri: redirectUri(), scope: config.scope, state, code_challenge: await challengeS256(verifier), code_challenge_method: 'S256' });
      deps.navigate(`${e.authorization_endpoint}?${q.toString()}`);
    },

    handleRedirect() {
      redirect ??= handleReturn();
      return redirect;
    },

    async accessToken() {
      if (!tokens) return null;
      if (tokens.expiresAt - now() > REFRESH_BEFORE_MS) return tokens.accessToken;
      if (!tokens.refreshToken) { save(null); return null; }
      const refreshToken = tokens.refreshToken;
      refreshing ??= tokenRequest({ grant_type: 'refresh_token', refresh_token: refreshToken })
        .then(t => { save({ ...t, refreshToken: t.refreshToken ?? refreshToken }); return t.accessToken; })
        .catch(() => { save(null); return null; })
        .finally(() => { refreshing = null; });
      return refreshing;
    },

    async signOut() {
      save(null);
      const e = await endpoints().catch(() => null);
      if (e?.end_session_endpoint) deps.navigate(`${e.end_session_endpoint}?${new URLSearchParams({ post_logout_redirect_uri: redirectUri(), client_id: config.clientId }).toString()}`);
      else deps.navigate(redirectUri());
    },

    user: () => (tokens ? nameOf(tokens.accessToken) ?? 'signed in' : null),
    lastError: () => error
  };
  return session;
}

/** The sign-in configured for this build (`VITE_OIDC_ISSUER`, `VITE_OIDC_CLIENT_ID`), or null: then the Studio talks to the API as before. */
export function oidcConfigFromEnv(env: Record<string, string | undefined>): OidcConfig | null {
  const issuer = env.VITE_OIDC_ISSUER?.trim();
  const clientId = env.VITE_OIDC_CLIENT_ID?.trim();
  if (!issuer || !clientId) return null;
  return { issuer, clientId, scope: env.VITE_OIDC_SCOPE?.trim() || 'openid' };
}

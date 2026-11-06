import React from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { challengeS256 } from '../auth/pkce';
import { createSession, type Session } from '../auth/session';
import { AuthGate } from '../auth/AuthGate';
import { RealEngineClient } from '../engine/RealEngineClient';

// E10.2: sign in with the identity provider (authorization code with PKCE, a public client: no secret in the browser), keep the tokens for the
// tab, refresh them before they expire, and send the access token to the API.
const ISSUER = 'https://sso.example.test/realms/helqix';
const DISCOVERY = { authorization_endpoint: `${ISSUER}/protocol/openid-connect/auth`, token_endpoint: `${ISSUER}/protocol/openid-connect/token`, end_session_endpoint: `${ISSUER}/protocol/openid-connect/logout` };
const config = { issuer: ISSUER, clientId: 'helqix-studio', scope: 'openid' };

class MemoryStorage { m = new Map<string, string>(); getItem(k: string) { return this.m.get(k) ?? null; } setItem(k: string, v: string) { this.m.set(k, v); } removeItem(k: string) { this.m.delete(k); } }

function jwt(claims: Record<string, unknown>) { const b = (o: unknown) => btoa(JSON.stringify(o)).replace(/=+$/, '').replace(/\+/g, '-').replace(/\//g, '_'); return `${b({ alg: 'RS256' })}.${b(claims)}.sig`; }

interface Calls { token: URLSearchParams[]; navigated: string[]; replaced: string[] }
function setup(opts: { href?: string; tokenResponse?: () => Response; now?: () => number } = {}) {
  const calls: Calls = { token: [], navigated: [], replaced: [] };
  const storage = new MemoryStorage();
  let href = opts.href ?? 'https://studio.example.test/';
  let tokenResponse = opts.tokenResponse ?? (() => new Response(JSON.stringify({ access_token: jwt({ preferred_username: 'alice', sub: 'u1' }), refresh_token: 'refresh-1', expires_in: 300 }), { status: 200 }));
  const fetchImpl = vi.fn(async (url: string, init?: RequestInit) => {
    if (String(url).endsWith('/.well-known/openid-configuration')) return new Response(JSON.stringify(DISCOVERY), { status: 200 });
    if (String(url) === DISCOVERY.token_endpoint) { calls.token.push(new URLSearchParams(String(init?.body))); return tokenResponse(); }
    return new Response('not found', { status: 404 });
  });
  const session = createSession({
    config, storage, fetchImpl: fetchImpl as unknown as typeof fetch, now: opts.now ?? (() => 1_000_000),
    location: () => ({ origin: 'https://studio.example.test', href, pathname: '/' }),
    navigate: u => calls.navigated.push(u), replaceUrl: u => calls.replaced.push(u)
  });
  return { session, calls, storage, setHref: (h: string) => { href = h; }, setTokenResponse: (f: () => Response) => { tokenResponse = f; } };
}

describe('PKCE', () => {
  it('S256 of the RFC 7636 example verifier is the RFC challenge', async () => {
    expect(await challengeS256('dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk')).toBe('E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM');
  });
});

describe('signing in', () => {
  it('goes to the provider with a PKCE challenge, a state, the client id and the Studio as the return address', async () => {
    const { session, calls, storage } = setup();
    await session.signIn();
    const u = new URL(calls.navigated[0]);
    expect(`${u.origin}${u.pathname}`).toBe(DISCOVERY.authorization_endpoint);
    expect(u.searchParams.get('response_type')).toBe('code');
    expect(u.searchParams.get('client_id')).toBe('helqix-studio');
    expect(u.searchParams.get('redirect_uri')).toBe('https://studio.example.test/');
    expect(u.searchParams.get('code_challenge_method')).toBe('S256');
    expect(u.searchParams.get('scope')).toBe('openid');
    const verifier = storage.getItem('helqix.pkce.verifier')!;
    expect(verifier.length).toBeGreaterThanOrEqual(43);
    expect(u.searchParams.get('code_challenge')).toBe(await challengeS256(verifier));
    expect(u.searchParams.get('state')).toBe(storage.getItem('helqix.pkce.state'));
    expect(u.searchParams.get('client_secret')).toBeNull();
  });

  it('trades the code for tokens with the verifier, cleans the address bar, and is signed in', async () => {
    const first = setup();
    await first.session.signIn();
    const state = first.storage.getItem('helqix.pkce.state')!;
    const verifier = first.storage.getItem('helqix.pkce.verifier')!;
    const { session, calls, storage } = setup({ href: `https://studio.example.test/?code=abc&state=${state}&session_state=x` });
    storage.setItem('helqix.pkce.state', state); storage.setItem('helqix.pkce.verifier', verifier);
    expect(await session.handleRedirect()).toBe('signed-in');
    const body = calls.token[0];
    expect([body.get('grant_type'), body.get('code'), body.get('code_verifier'), body.get('client_id'), body.get('redirect_uri')]).toEqual(['authorization_code', 'abc', verifier, 'helqix-studio', 'https://studio.example.test/']);
    expect(calls.replaced).toEqual(['/']);
    expect(session.user()).toBe('alice');
    expect(await session.accessToken()).toMatch(/^eyJ/);
    expect(storage.getItem('helqix.pkce.verifier')).toBeNull();
  });

  it('refuses a return with the wrong state or no stored verifier, and signs nobody in', async () => {
    const { session, calls, storage } = setup({ href: 'https://studio.example.test/?code=abc&state=forged' });
    storage.setItem('helqix.pkce.state', 'real'); storage.setItem('helqix.pkce.verifier', 'v'.repeat(50));
    expect(await session.handleRedirect()).toBe('error');
    expect(calls.token).toEqual([]);
    expect(session.user()).toBeNull();
    const none = setup({ href: 'https://studio.example.test/?code=abc&state=real' });
    expect(await none.session.handleRedirect()).toBe('error');
  });

  it('asking twice at once (React runs an effect twice in development) trades the code once and both callers see the sign-in', async () => {
    const first = setup();
    await first.session.signIn();
    const state = first.storage.getItem('helqix.pkce.state')!;
    const { session, calls, storage } = setup({ href: `https://studio.example.test/?code=abc&state=${state}` });
    storage.setItem('helqix.pkce.state', state); storage.setItem('helqix.pkce.verifier', first.storage.getItem('helqix.pkce.verifier')!);
    const [a, b] = await Promise.all([session.handleRedirect(), session.handleRedirect()]);
    expect([a, b]).toEqual(['signed-in', 'signed-in']);
    expect(calls.token.filter(p => p.get('grant_type') === 'authorization_code')).toHaveLength(1);
    expect(session.user()).toBe('alice');
  });

  it('a provider error on return is reported, not hidden', async () => {
    const { session } = setup({ href: 'https://studio.example.test/?error=access_denied&error_description=No' });
    expect(await session.handleRedirect()).toBe('error');
    expect(session.lastError()).toMatch(/access_denied|No/);
  });

  it('a page that is not a return from the provider is left alone', async () => {
    const { session, calls } = setup();
    expect(await session.handleRedirect()).toBe('none');
    expect(calls.token).toEqual([]);
  });
});

describe('keeping the session', () => {
  async function signedIn(now: { t: number }) {
    const s = setup({ now: () => now.t });
    await s.session.signIn();
    const state = s.storage.getItem('helqix.pkce.state')!;
    s.setHref(`https://studio.example.test/?code=c&state=${state}`);
    await s.session.handleRedirect();
    return s;
  }

  it('hands out the same token until it is about to expire, then refreshes once for many callers', async () => {
    const now = { t: 1_000_000 };
    const s = await signedIn(now);
    const first = await s.session.accessToken();
    s.setTokenResponse(() => new Response(JSON.stringify({ access_token: jwt({ preferred_username: 'alice', n: 2 }), refresh_token: 'refresh-2', expires_in: 300 }), { status: 200 }));
    now.t += 100_000;
    expect(await s.session.accessToken()).toBe(first);
    now.t += 190_000;
    const [a, b, c] = await Promise.all([s.session.accessToken(), s.session.accessToken(), s.session.accessToken()]);
    expect(a).not.toBe(first);
    expect([a, b]).toEqual([b, c]);
    const refreshes = s.calls.token.filter(p => p.get('grant_type') === 'refresh_token');
    expect(refreshes).toHaveLength(1);
    expect(refreshes[0].get('refresh_token')).toBe('refresh-1');
  });

  it('a refresh the provider refuses signs the user out instead of looping', async () => {
    const now = { t: 1_000_000 };
    const s = await signedIn(now);
    s.setTokenResponse(() => new Response(JSON.stringify({ error: 'invalid_grant' }), { status: 400 }));
    now.t += 400_000;
    expect(await s.session.accessToken()).toBeNull();
    expect(s.session.user()).toBeNull();
  });

  it('a network error or a 5xx while refreshing keeps the session and the next call tries again', async () => {
    const now = { t: 1_000_000 };
    const s = await signedIn(now);
    const first = await s.session.accessToken();
    now.t += 280_000; // inside the 30 s before expiry: a refresh is due, the token still works
    s.setTokenResponse(() => new Response('{}', { status: 503 }));
    expect(await s.session.accessToken()).toBe(first);
    expect(s.session.user()).toBe('alice');
    s.setTokenResponse(() => { throw new TypeError('network down'); });
    expect(await s.session.accessToken()).toBe(first);
    expect(s.session.user()).toBe('alice');
    s.setTokenResponse(() => new Response(JSON.stringify({ access_token: jwt({ preferred_username: 'alice', n: 3 }), refresh_token: 'refresh-3', expires_in: 300 }), { status: 200 }));
    expect(await s.session.accessToken()).not.toBe(first);
  });

  it('a transient failure after the token has expired gives no token but still keeps the refresh token', async () => {
    const now = { t: 1_000_000 };
    const s = await signedIn(now);
    now.t += 400_000;
    s.setTokenResponse(() => new Response('{}', { status: 502 }));
    expect(await s.session.accessToken()).toBeNull();
    expect(s.session.user()).toBe('alice');
  });

  it('tells a listener when the tokens are lost', async () => {
    const now = { t: 1_000_000 };
    const s = await signedIn(now);
    const heard = vi.fn();
    const stop = s.session.subscribe(heard);
    s.setTokenResponse(() => new Response(JSON.stringify({ error: 'invalid_grant' }), { status: 400 }));
    now.t += 400_000;
    await s.session.accessToken();
    expect(heard).toHaveBeenCalledTimes(1);
    stop();
    await s.session.signOut();
    expect(heard).toHaveBeenCalledTimes(1);
  });

  it('signing out forgets the tokens and goes to the provider to end its session', async () => {
    const now = { t: 1_000_000 };
    const s = await signedIn(now);
    await s.session.signOut();
    expect(s.session.user()).toBeNull();
    expect(await s.session.accessToken()).toBeNull();
    const u = new URL(s.calls.navigated.at(-1)!);
    expect(`${u.origin}${u.pathname}`).toBe(DISCOVERY.end_session_endpoint);
    expect(u.searchParams.get('post_logout_redirect_uri')).toBe('https://studio.example.test/');
    expect(u.searchParams.get('client_id')).toBe('helqix-studio');
  });

  it('a page reload in the same tab keeps the session', async () => {
    const now = { t: 1_000_000 };
    const s = await signedIn(now);
    const again = createSession({ config, storage: s.storage, fetchImpl: vi.fn() as unknown as typeof fetch, now: () => now.t, location: () => ({ origin: 'https://studio.example.test', href: 'https://studio.example.test/', pathname: '/' }), navigate: () => undefined, replaceUrl: () => undefined });
    expect(again.user()).toBe('alice');
    expect(await again.accessToken()).toMatch(/^eyJ/);
  });
});

describe('the engine client with a session', () => {
  let server: typeof fetch;
  beforeEach(() => { server = globalThis.fetch; });
  afterEach(() => { globalThis.fetch = server; });

  it('sends the access token as the bearer, and does nothing when there is no session', async () => {
    const seen: (string | null)[] = [];
    globalThis.fetch = vi.fn(async (_u: unknown, init?: RequestInit) => { seen.push(new Headers(init?.headers).get('authorization')); return new Response(JSON.stringify({ fmus: [] }), { status: 200 }); }) as unknown as typeof fetch;
    await new RealEngineClient('/api', async () => 'tok-1').listFmus();
    await new RealEngineClient('/api').listFmus();
    expect(seen).toEqual(['Bearer tok-1', null]);
  });

  it('an answer of 401 for a signed-in user tells the session so', async () => {
    globalThis.fetch = vi.fn(async () => new Response('{}', { status: 401 })) as unknown as typeof fetch;
    const onUnauthorized = vi.fn();
    await expect(new RealEngineClient('/api', async () => 'tok', onUnauthorized).listFmus()).rejects.toThrow(/401/);
    expect(onUnauthorized).toHaveBeenCalledTimes(1);
  });
});

describe('AuthGate', () => {
  afterEach(cleanup);
  function fake(state: { user: string | null; result?: 'none' | 'signed-in' | 'error'; error?: string | null }): Session {
    return { handleRedirect: vi.fn(async () => state.result ?? 'none'), user: () => state.user, signIn: vi.fn(async () => undefined), signOut: vi.fn(async () => undefined), accessToken: async () => 'x', lastError: () => state.error ?? null, subscribe: () => () => undefined } as unknown as Session;
  }
  it('shows a sign-in screen, not the app, until the user is signed in', async () => {
    const s = fake({ user: null });
    render(<AuthGate session={s}><p>the app</p></AuthGate>);
    expect(await screen.findByRole('button', { name: 'Sign in' })).toBeTruthy();
    expect(screen.queryByText('the app')).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Sign in' }));
    expect(s.signIn).toHaveBeenCalled();
  });
  it('shows the app and a sign-out button with the user name when signed in', async () => {
    const s = fake({ user: 'alice' });
    render(<AuthGate session={s}><p>the app</p></AuthGate>);
    expect(await screen.findByText('the app')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: /Sign out alice/ }));
    expect(s.signOut).toHaveBeenCalled();
  });
  it('says why a sign-in failed', async () => {
    render(<AuthGate session={fake({ user: null, result: 'error', error: 'The sign-in answer did not match this browser tab.' })}><p>the app</p></AuthGate>);
    expect((await screen.findByRole('alert')).textContent).toContain('did not match');
  });
  it('without a session configured the app is shown as before', async () => {
    render(<AuthGate session={null}><p>the app</p></AuthGate>);
    await waitFor(() => expect(screen.getByText('the app')).toBeTruthy());
  });
});

describe('AuthGate follows the session', () => {
  afterEach(cleanup);
  it('goes back to the sign-in screen when the tokens are lost while the app is open', async () => {
    let user: string | null = 'alice';
    const listeners = new Set<() => void>();
    const s = { handleRedirect: async () => 'none', user: () => user, signIn: async () => undefined, signOut: async () => undefined, accessToken: async () => 'x', lastError: () => null, subscribe: (l: () => void) => { listeners.add(l); return () => { listeners.delete(l); }; } } as unknown as Session;
    render(<AuthGate session={s}><p>the app</p></AuthGate>);
    expect(await screen.findByText('the app')).toBeTruthy();
    user = null;
    React.act(() => { listeners.forEach(l => l()); });
    expect(await screen.findByRole('button', { name: 'Sign in' })).toBeTruthy();
    expect(screen.queryByText('the app')).toBeNull();
  });
});

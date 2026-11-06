import { createSession, oidcConfigFromEnv, type Session } from './session';

/** The Studio's session when sign-in is configured for this build (real engine and VITE_OIDC_ISSUER + VITE_OIDC_CLIENT_ID), else null. */
const config = import.meta.env.VITE_ENGINE === 'real' ? oidcConfigFromEnv(import.meta.env as Record<string, string | undefined>) : null;

export const appSession: Session | null = config && typeof window !== 'undefined'
  ? createSession({
      config,
      storage: window.sessionStorage,
      location: () => ({ origin: window.location.origin, href: window.location.href, pathname: window.location.pathname }),
      navigate: url => window.location.assign(url),
      replaceUrl: url => window.history.replaceState(null, '', url)
    })
  : null;

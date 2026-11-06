/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** "real" uses the HELQIX API through the dev proxy; anything else uses the mock engine. */
  readonly VITE_ENGINE?: string;
  /** The identity provider (OpenID Connect issuer URL) and the Studio's public client id: both set turns user sign-in on. */
  readonly VITE_OIDC_ISSUER?: string;
  readonly VITE_OIDC_CLIENT_ID?: string;
  readonly VITE_OIDC_SCOPE?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

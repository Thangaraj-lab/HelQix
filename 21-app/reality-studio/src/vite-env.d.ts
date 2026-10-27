/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** "real" uses the HELQIX API through the dev proxy; anything else uses the mock engine. */
  readonly VITE_ENGINE?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

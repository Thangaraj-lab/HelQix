/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** Origin of the Layer 22 HELQIX API server, if configured. */
  readonly VITE_API_ORIGIN?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

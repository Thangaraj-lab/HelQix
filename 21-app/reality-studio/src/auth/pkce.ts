// PKCE (RFC 7636): the browser proves it is the one that started the sign-in without holding a client secret.

const b64url = (bytes: Uint8Array): string => btoa(String.fromCharCode(...bytes)).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');

/** A random URL-safe string of `bytes` random bytes (43 characters for 32 bytes). */
export function randomString(bytes = 32): string {
  const a = new Uint8Array(bytes);
  crypto.getRandomValues(a);
  return b64url(a);
}

/** The S256 code challenge of a verifier: base64url(SHA-256(verifier)). */
export async function challengeS256(verifier: string): Promise<string> {
  const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(verifier));
  return b64url(new Uint8Array(digest));
}

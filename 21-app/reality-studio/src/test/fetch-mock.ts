/** Minimal deterministic fetch mock for API-boundary tests. */
type Responder = (url: string, init?: RequestInit) => { status: number; body: unknown };

export interface FetchMock {
  readonly calls: Array<{ url: string; init?: RequestInit }>;
  respond(pathContains: string, responder: Responder): void;
  restore(): void;
}

export function installFetchMock(defaults: Responder = () => ({ status: 404, body: null })): FetchMock {
  const routes: Array<{ match: string; responder: Responder }> = [];
  const calls: FetchMock["calls"] = [];
  const original = globalThis.fetch;

  globalThis.fetch = (async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = typeof input === "string" ? input : input.toString();
    calls.push({ url, init });
    const route = routes.find((r) => url.includes(r.match));
    const { status, body } = (route?.responder ?? defaults)(url, init);
    return new Response(body === null ? "" : JSON.stringify(body), {
      status,
      headers: { "Content-Type": "application/json" },
    });
  }) as typeof fetch;

  return {
    calls,
    respond(match, responder) {
      routes.push({ match, responder });
    },
    restore() {
      globalThis.fetch = original;
    },
  };
}

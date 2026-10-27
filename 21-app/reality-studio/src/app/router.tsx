// Flat routes from file 35 section 12: / Hero, /home, /new (creates a project and redirects),
// /p/:projectId (Studio). Studio state lives in the URL query (stage, step, sel, t, run, rev), so
// back and forward work. A tiny router on the History API, so no routing library is needed.
import { useEffect, useState } from 'react';

export type Route =
  | { name: 'hero' }
  | { name: 'home' }
  | { name: 'new' }
  | { name: 'studio'; projectId: string }
  | { name: 'not-found'; path: string };

export function parseRoute(pathname: string): Route {
  const path = pathname.replace(/\/+$/, '') || '/';
  if (path === '/') return { name: 'hero' };
  if (path === '/home') return { name: 'home' };
  if (path === '/new') return { name: 'new' };
  const m = /^\/p\/([A-Za-z0-9_-]{1,64})$/.exec(path);
  if (m) return { name: 'studio', projectId: m[1] };
  return { name: 'not-found', path };
}

const EVENT = 'helqix:navigate';

/** Navigates within the app. `replace` swaps the history entry (used by /new after creating). */
export function navigate(to: string, opts: { replace?: boolean } = {}): void {
  if (opts.replace) window.history.replaceState({}, '', to);
  else window.history.pushState({}, '', to);
  window.dispatchEvent(new Event(EVENT));
}

/** Updates Studio query keys (stage, step, sel, t, run, rev) without leaving the page. */
export function setQuery(updates: Record<string, string | null>, opts: { replace?: boolean } = {}): void {
  const url = new URL(window.location.href);
  for (const [k, v] of Object.entries(updates)) {
    if (v === null) url.searchParams.delete(k);
    else url.searchParams.set(k, v);
  }
  navigate(url.pathname + url.search, opts);
}

export function useLocation(): { route: Route; query: URLSearchParams } {
  const read = () => ({ route: parseRoute(window.location.pathname), query: new URLSearchParams(window.location.search) });
  const [loc, setLoc] = useState(read);
  useEffect(() => {
    const update = () => setLoc(read());
    window.addEventListener('popstate', update);
    window.addEventListener(EVENT, update);
    return () => {
      window.removeEventListener('popstate', update);
      window.removeEventListener(EVENT, update);
    };
  }, []);
  return loc;
}

import React, { useEffect, useState } from 'react';
import type { Session } from './session';

/** Shows the app only to a signed-in user. With no session configured it is transparent (the Studio talks to the API as before). */
export const AuthGate: React.FC<{ session: Session | null; children: React.ReactNode }> = ({ session, children }) => {
  const [ready, setReady] = useState(session === null);
  const [error, setError] = useState<string | null>(null);
  const [, follow] = useState(0);

  // The tokens can be lost later (a refresh the provider refuses, a 401): show the sign-in screen then, not only after a reload.
  useEffect(() => session?.subscribe(() => follow(n => n + 1)), [session]);

  useEffect(() => {
    if (!session) return;
    let live = true;
    session.handleRedirect().then(r => { if (!live) return; if (r === 'error') setError(session.lastError()); setReady(true); });
    return () => { live = false; };
  }, [session]);

  if (!session) return <>{children}</>;
  if (!ready) return <div className="min-h-screen grid place-items-center text-xs text-muted">Signing in…</div>;
  const user = session.user();
  if (!user) {
    return (
      <div className="min-h-screen grid place-items-center p-6">
        <div className="max-w-sm space-y-3 text-center">
          <h1 className="text-lg text-strong">Sign in to HELQIX</h1>
          <p className="text-xs text-muted">Your projects, builds and runs belong to your account; other people cannot see them.</p>
          {error && <p role="alert" className="text-xs text-bad">{error}</p>}
          <button type="button" className="h-9 px-4 rounded-[10px] bg-accent text-void text-sm" onClick={() => void session.signIn()}>Sign in</button>
        </div>
      </div>
    );
  }
  return (
    <>
      {children}
      <button type="button" aria-label={`Sign out ${user}`} onClick={() => void session.signOut()} className="fixed bottom-3 left-3 z-50 text-[11px] text-muted underline">Sign out ({user})</button>
    </>
  );
};

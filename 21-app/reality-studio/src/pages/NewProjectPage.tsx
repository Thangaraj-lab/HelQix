import React, { useEffect, useRef, useState } from 'react';
import { navigate } from '../app/router';
import { engine } from '../engine';

// Route /new (file 36 M2): create the project with an automatic name, then replace this entry
// with the Studio's Prompt screen. No modal, no domain question, no model-type cards.
export const NewProjectPage: React.FC = () => {
  const [error, setError] = useState<string | null>(null);
  const started = useRef(false);

  useEffect(() => {
    if (started.current) return; // React StrictMode runs effects twice in development
    started.current = true;
    engine
      .createProject()
      .then((p) => navigate(`/p/${encodeURIComponent(p.id)}?stage=define&step=intent`, { replace: true }))
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
  }, []);

  return (
    <div className="h-screen w-screen flex items-center justify-center bg-[var(--bg-void)] text-[var(--text-ink)]">
      {error ? (
        <div role="alert" className="max-w-md rounded-xl border border-red-500/40 bg-red-500/10 p-5 space-y-3 text-sm">
          <p className="font-semibold text-red-300">The project could not be created.</p>
          <p className="text-[#D6E2F3]">{error}</p>
          <div className="flex gap-2">
            <button className="px-3 py-1.5 rounded-md bg-[#2F7BFF] text-white text-xs" onClick={() => window.location.reload()}>Try again</button>
            <button className="px-3 py-1.5 rounded-md border border-white/20 text-xs" onClick={() => navigate('/home')}>Back to Home</button>
          </div>
        </div>
      ) : (
        <p role="status" className="text-sm text-[#7F93AF] font-mono">Creating project…</p>
      )}
    </div>
  );
};

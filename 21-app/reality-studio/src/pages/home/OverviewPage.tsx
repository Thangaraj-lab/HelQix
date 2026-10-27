import React, { useEffect, useState } from 'react';
import { Project } from '../../contracts';
import { engine } from '../../engine';
import { navigate } from '../../app/router';
import { writeBuildInput } from '../../app/draft';
import { EXAMPLE_PROMPT } from '../../mock/examplePack';
import { ExampleTag } from '../../components/ui/UiKit';

// Home (file 36 section 6.2): a prompt box like the Hero's, then this user's projects as the engine
// reports them. No invented runs, trust chips or alerts: only what the engine knows.
export const OverviewPage: React.FC = () => {
  const [promptText, setPromptText] = useState('');
  const [projects, setProjects] = useState<Project[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let live = true;
    engine
      .listProjects()
      .then(p => live && setProjects(p))
      .catch((e: unknown) => live && setError(e instanceof Error ? e.message : String(e)));
    return () => {
      live = false;
    };
  }, []);

  // Creates the project here (rather than via /new) so the typed prompt goes with it.
  const startWithPrompt = async () => {
    const text = promptText.trim();
    if (!text || busy) return;
    setBusy(true);
    try {
      const p = await engine.createProject();
      writeBuildInput(p.id, { kind: 'prompt', text });
      navigate(`/p/${encodeURIComponent(p.id)}?stage=define&step=intent`);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setBusy(false);
    }
  };

  const open = (p: Project) => {
    const n = p.modelRevisions.length;
    navigate(`/p/${encodeURIComponent(p.id)}?stage=${n ? 'model' : 'define'}${n ? `&rev=r${n}` : '&step=intent'}`);
  };

  return (
    <div className="p-6 space-y-6 max-w-7xl mx-auto">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-bold text-[#F2F6FB] tracking-wide">Home</h1>
        <button
          onClick={() => navigate('/new')}
          className="px-4 py-2 rounded-[8px] text-xs font-semibold text-white bg-[#2F7BFF] hover:bg-[#3B82F6] transition-all"
        >
          + New project
        </button>
      </div>

      <div className="w-full rounded-[14px] bg-[#0A1422] border border-[#22395A] p-4 flex flex-col gap-3">
        <label htmlFor="home-prompt" className="text-xs font-semibold text-[#D6E2F3]">Start a new model from a description</label>
        <textarea
          id="home-prompt"
          value={promptText}
          onChange={e => setPromptText(e.target.value)}
          placeholder="Describe what you want to engineer"
          rows={2}
          className="w-full bg-[#0E1B2E] rounded-[8px] p-3 border border-[#22395A] text-xs text-[#F2F6FB] placeholder-[#7F93AF] focus:outline-none focus:border-[#2F7BFF]"
        />
        <div className="flex items-center justify-between">
          <button
            onClick={() => setPromptText(EXAMPLE_PROMPT)}
            className="px-2.5 py-1 rounded-[6px] bg-[#13253D] hover:bg-[#1E3A5F] border border-[#22395A] text-[11px] text-[#22D3EE] font-mono flex items-center gap-1.5"
          >
            <ExampleTag /> Paste example
          </button>
          <button
            onClick={startWithPrompt}
            disabled={!promptText.trim() || busy}
            className="px-4 py-1.5 rounded-[8px] text-xs font-semibold text-white bg-[#2F7BFF] hover:bg-[#3B82F6] disabled:bg-[#1E293B] disabled:text-slate-500 disabled:cursor-not-allowed"
          >
            Describe it →
          </button>
        </div>
      </div>

      <section className="space-y-3">
        <div className="flex items-center justify-between border-b border-[#22395A] pb-2">
          <h2 className="text-sm font-semibold text-[#F2F6FB]">Projects</h2>
          {projects && <span className="text-xs text-[#7F93AF] font-mono">{projects.length}</span>}
        </div>
        {error ? (
          <p role="alert" className="text-xs text-amber-300">Projects could not be loaded: {error}</p>
        ) : projects === null ? (
          <p className="text-xs text-[#7F93AF] font-mono">Loading…</p>
        ) : projects.length === 0 ? (
          <p className="text-xs text-[#7F93AF]">No projects yet. Describe something above, or start a new project.</p>
        ) : (
          <ul className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
            {projects.map(p => (
              <li key={p.id}>
                <button
                  onClick={() => open(p)}
                  className="w-full text-left rounded-[14px] bg-[#0A1422] border border-[#22395A] hover:border-[#2F7BFF]/60 p-4 space-y-1 transition-colors"
                >
                  <div className="text-xs font-semibold text-[#F2F6FB] truncate">{p.name}</div>
                  <div className="text-[11px] text-[#7F93AF] font-mono">
                    {p.modelRevisions.length ? `Model r${p.modelRevisions.length}` : 'Draft - not committed'}
                  </div>
                  <div className="text-[10px] text-[#7F93AF] font-mono">Updated {new Date(p.updatedAt).toLocaleString()}</div>
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
};

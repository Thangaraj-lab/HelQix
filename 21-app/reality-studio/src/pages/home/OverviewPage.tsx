import React, { useEffect, useMemo, useState } from 'react';
import { Boxes, CircleAlert, Paperclip, Sparkles } from 'lucide-react';
import { Project, RunSummary, Source } from '../../contracts';
import { EXAMPLE_SCENARIOS } from '../../mock/scenarios';
import { engine } from '../../engine';
import { navigate } from '../../app/router';
import { startProject } from '../../app/start';
import { EXAMPLE_PROMPT } from '../../mock/examplePack';
import { Button, Chip, EmptyState, ErrorState, ExampleTag, PreviewTag, SectionLabel, Skeleton, StatusDot } from '../../components/ui/UiKit';

// Home (file 37 section 6.2): greeting, the Hero's prompt box, then only what the engine knows:
// projects, runs, sources, and a trust summary made of facts. Nothing here is invented.

const greeting = () => {
  const h = new Date().getHours();
  return h < 12 ? 'Good morning' : h < 18 ? 'Good afternoon' : 'Good evening';
};

const projectState = (p: Project) => (p.modelRevisions.length ? `Model r${p.modelRevisions.length}` : 'Draft, not committed');

export const OverviewPage: React.FC = () => {
  const [text, setText] = useState('');
  const [projects, setProjects] = useState<Project[] | null>(null);
  const [sources, setSources] = useState<Source[] | null>(null);
  const [runs, setRuns] = useState<RunSummary[] | null>(null);
  const [runTotal, setRunTotal] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const load = () => {
    setError(null);
    engine
      .listProjects()
      .then(async list => {
        setProjects(list);
        Promise.all(list.map(p => engine.listRuns(p.id).catch(() => [] as RunSummary[]))).then(r => { const all = r.flat().sort((a, b) => b.startedAt.localeCompare(a.startedAt)); setRunTotal(all.length); setRuns(all.slice(0, 6)); });
        try {
          setSources(await engine.listSources(list[0]?.id ?? ''));
        } catch {
          setSources([]);
        }
      })
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
  };
  useEffect(load, []);

  const start = async (input: Parameters<typeof startProject>[0]) => {
    if (busy) return;
    setBusy(true);
    try {
      await startProject(input);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setBusy(false);
    }
  };

  const open = (p: Project) => {
    const n = p.modelRevisions.length;
    navigate(`/p/${encodeURIComponent(p.id)}?` + (n ? `stage=model&rev=r${n}` : 'stage=define&step=intent'));
  };

  // Needs attention: facts derived from the projects, never alerts from nowhere.
  const attention = useMemo(
    () => (projects ?? []).filter(p => p.modelRevisions.length === 0).map(p => ({ p, text: 'Draft, not committed. Nothing can run until a model revision exists.' })),
    [projects]
  );
  const committed = (projects ?? []).filter(p => p.modelRevisions.length > 0).length;

  return (
    <div className="max-w-[1180px] mx-auto px-8 py-10 space-y-9">
      <header className="space-y-1">
        <h1 className="font-display font-light text-[28px] text-strong tracking-[0.005em]">{greeting()}</h1>
        <p className="text-sm text-muted">Describe something to engineer, or pick up where you left off.</p>
      </header>

      <section className="surface p-4" aria-label="New model">
        <label htmlFor="home-prompt" className="sr-only">Describe what you want to engineer</label>
        <textarea
          id="home-prompt"
          value={text}
          onChange={e => setText(e.target.value)}
          onKeyDown={e => e.key === 'Enter' && (e.metaKey || e.ctrlKey) && text.trim() && void start({ kind: 'prompt', text: text.trim() })}
          placeholder="Describe what you want to engineer…"
          rows={2}
          className="w-full resize-none bg-transparent text-[15px] leading-relaxed placeholder:text-muted/70 focus:outline-none px-1"
        />
        <div className="flex flex-wrap items-center justify-between gap-2 pt-2 border-t border-line mt-2">
          <div className="flex flex-wrap items-center gap-1">
            <Button variant="ghost" size="sm" icon={<Paperclip size={14} />} disabled>Attach files <PreviewTag /></Button>
            <Button variant="ghost" size="sm" icon={<Sparkles size={14} />} onClick={() => start({ kind: 'scratch' })}>Start from scratch</Button>
            <Button variant="ghost" size="sm" onClick={() => setText(EXAMPLE_PROMPT)}>Use the example prompt <ExampleTag /></Button>
          </div>
          <Button variant="primary" disabled={!text.trim() || busy} onClick={() => start({ kind: 'prompt', text: text.trim() })}>Build model</Button>
        </div>
      </section>

      <div className="grid grid-cols-1 lg:grid-cols-[1fr_320px] gap-8 items-start">
        <div className="space-y-9 min-w-0">
          <section aria-label="Projects" className="space-y-3">
            <SectionLabel aside={projects && <span className="num text-xs text-muted">{projects.length}</span>}>Projects</SectionLabel>
            {error ? (
              <ErrorState error="Projects could not be loaded" reason={error} onRetry={load} />
            ) : projects === null ? (
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-3"><Skeleton className="h-28" /><Skeleton className="h-28" /></div>
            ) : projects.length === 0 ? (
              <EmptyState title="No projects yet" description="Describe a system above, or start from scratch. Your first project appears here." actionLabel="Start from scratch" onAction={() => start({ kind: 'scratch' })} />
            ) : (
              <ul className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                {projects.map(p => (
                  <li key={p.id}>
                    <button onClick={() => open(p)} className="group w-full text-left surface p-4 h-full hover:border-line-strong transition-colors">
                      <div className="flex items-start justify-between gap-3">
                        <div className="w-9 h-9 rounded-[9px] bg-white/[0.04] border border-line flex items-center justify-center text-muted group-hover:text-accent transition-colors"><Boxes size={16} /></div>
                        <Chip tone={p.modelRevisions.length ? 'accent' : 'neutral'}>{projectState(p)}</Chip>
                      </div>
                      <div className="mt-3 text-[14px] font-medium text-strong truncate">{p.name}</div>
                      <div className="mt-0.5 text-xs text-muted">Updated {new Date(p.updatedAt).toLocaleString()}</div>
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </section>

          <section aria-label="Recent runs" className="space-y-3">
            <SectionLabel>Recent runs</SectionLabel>
            {runs === null ? <Skeleton className="h-14" /> : runs.length === 0 ? (
              <EmptyState title="No runs yet" description="A run appears here after you commit a model and simulate it." />
            ) : (
              <ul className="surface divide-y divide-line overflow-hidden">
                {runs.map(r => {
                  const p = projects?.find(x => x.id === r.projectId);
                  return (
                    <li key={r.id}>
                      <button onClick={() => navigate(`/p/${encodeURIComponent(r.projectId)}?stage=simulate&rev=r${r.revisionNum}`)} className="w-full text-left px-4 py-3 flex items-center justify-between gap-3 hover:bg-white/[0.03] transition-colors">
                        <span className="min-w-0">
                          <span className="block text-[13px] text-strong truncate">{p?.name ?? r.projectId}</span>
                          <span className="block text-[11px] text-muted">{EXAMPLE_SCENARIOS.find(s => s.id === r.scenarioId)?.title ?? r.scenarioId} · Model r{r.revisionNum}</span>
                        </span>
                        <span className="text-[11px] text-muted shrink-0">{new Date(r.startedAt).toLocaleString()}</span>
                      </button>
                    </li>
                  );
                })}
              </ul>
            )}
          </section>
        </div>

        <aside className="space-y-6" aria-label="Status">
          <section className="surface p-4 space-y-3">
            <SectionLabel aside={<span className="num text-xs text-muted">{attention.length}</span>}>Needs attention</SectionLabel>
            {attention.length === 0 ? (
              <p className="text-xs text-muted">Nothing needs attention.</p>
            ) : (
              <ul className="space-y-2.5">
                {attention.map(({ p, text }) => (
                  <li key={p.id}>
                    <button onClick={() => open(p)} className="w-full text-left flex gap-2.5 group">
                      <CircleAlert size={15} className="text-warn mt-0.5 shrink-0" />
                      <span className="min-w-0">
                        <span className="block text-[13px] text-strong truncate group-hover:underline">{p.name}</span>
                        <span className="block text-xs text-muted">{text}</span>
                      </span>
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </section>

          <section className="surface p-4 space-y-3">
            <SectionLabel aside={<ExampleTag />}>Connected sources</SectionLabel>
            {sources === null ? (
              <Skeleton className="h-16" />
            ) : sources.length === 0 ? (
              <p className="text-xs text-muted">No sources connected. Parts will be placeholders or defined from scratch.</p>
            ) : (
              <ul className="space-y-2">
                {sources.map(s => (
                  <li key={s.id} className="flex items-center gap-2 text-[13px] text-ink">
                    <StatusDot tone={s.status === 'connected' ? 'ok' : s.status === 'syncing' ? 'warn' : 'bad'} />
                    <span className="truncate">{s.name}</span>
                  </li>
                ))}
              </ul>
            )}
          </section>

          <section className="surface p-4 space-y-3">
            <SectionLabel>Trust summary</SectionLabel>
            <dl className="space-y-2 text-[13px]">
              <div className="flex justify-between"><dt className="text-muted">Committed models</dt><dd className="num text-strong">{committed}</dd></div>
              <div className="flex justify-between"><dt className="text-muted">Runs</dt><dd className="num text-strong">{runTotal}</dd></div>
              <div className="flex justify-between"><dt className="text-muted">Signed evidence</dt><dd className="num text-strong">0</dd></div>
            </dl>
            <p className="text-[11px] text-muted/80">Counts only. Nothing is verified until its criteria are shown.</p>
          </section>
        </aside>
      </div>
    </div>
  );
};

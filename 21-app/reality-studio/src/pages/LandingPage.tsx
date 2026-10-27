import React, { useState } from 'react';
import { Paperclip, Plug, Sparkles } from 'lucide-react';
import { navigate } from '../app/router';
import { startProject } from '../app/start';
import { Button, ExampleTag, Logo, PreviewTag } from '../components/ui/UiKit';

// Hero (file 37 section 6.1): the public door. One promise, one prompt box, one static explainer
// of how a result earns trust. It never shows live claims.

const PROOF_STEPS = ['Run', 'Replay matches', 'Requirement checked', 'Signed', 'Verified by anyone'];
const TOOLS = ['FMI / Modelica', 'CalculiX', 'Blender', 'glTF', 'PyBaMM'];

export const LandingPage: React.FC = () => {
  const [text, setText] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const go = async (input: Parameters<typeof startProject>[0]) => {
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      await startProject(input);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setBusy(false);
    }
  };
  const canBuild = text.trim() !== '' && !busy;

  return (
    <div className="relative h-screen w-screen overflow-hidden bg-void text-ink">
      {/* Scene: the portal image, with the wordmark painted into it masked out so the page's own type leads. */}
      <img src="/assets/helqix-portal.jpg" alt="" className="absolute inset-0 w-full h-full object-cover object-[50%_62%] opacity-90" />
      <div className="absolute inset-0 bg-[radial-gradient(ellipse_46%_36%_at_50%_50%,#030914_62%,rgba(3,9,20,0.82)_78%,transparent_100%)]" />
      <div className="absolute inset-x-0 bottom-0 h-[40%] bg-gradient-to-t from-[#030914] via-[#030914]/80 to-transparent" />
      <div className="absolute inset-x-0 top-0 h-32 bg-gradient-to-b from-[#030914]/80 to-transparent" />

      <header className="relative z-10 h-16 px-8 flex items-center justify-between">
        <Logo />
        <nav className="flex items-center gap-2" aria-label="Primary">
          <Button variant="ghost" onClick={() => navigate('/home')}>Open studio</Button>
          <Button variant="secondary" onClick={() => go({ kind: 'scratch' })}>New project</Button>
        </nav>
      </header>

      <main className="relative z-10 h-[calc(100%-4rem)] flex flex-col items-center px-6">
        <div className="mt-[9vh] text-center animate-rise">
          <h1 className="font-display font-extralight text-strong text-[clamp(34px,4.6vw,60px)] leading-[1.08] tracking-[0.01em]">
            Engineering truth,
            <br />
            with proof.
          </h1>
          <p className="mt-5 text-[15px] text-muted max-w-[520px] mx-auto">
            Describe a system, simulate it, understand why it fails, and hand over evidence anyone can check.
          </p>
        </div>

        <div className="mt-9 w-full max-w-[720px] surface-glass p-3 animate-rise [animation-delay:120ms]">
          <label htmlFor="hero-prompt" className="sr-only">Describe what you want to engineer</label>
          <textarea
            id="hero-prompt"
            value={text}
            onChange={e => setText(e.target.value)}
            onKeyDown={e => {
              if (e.key === 'Enter' && (e.metaKey || e.ctrlKey) && canBuild) void go({ kind: 'prompt', text: text.trim() });
            }}
            placeholder="Describe what you want to engineer…"
            rows={3}
            className="w-full resize-none bg-transparent px-3 pt-2 pb-1 text-[15px] leading-relaxed text-strong placeholder:text-muted/70 focus:outline-none"
          />
          <div className="flex flex-wrap items-center justify-between gap-2 px-1 pt-1">
            <div className="flex flex-wrap items-center gap-1">
              <Button variant="ghost" size="sm" icon={<Paperclip size={14} />} disabled title="Attachments are not connected yet">
                Attach files <PreviewTag />
              </Button>
              <Button variant="ghost" size="sm" icon={<Sparkles size={14} />} onClick={() => go({ kind: 'scratch' })}>
                Start from scratch
              </Button>
              <Button variant="ghost" size="sm" icon={<Plug size={14} />} disabled title="Tool import is not connected yet">
                Import from tools <PreviewTag />
              </Button>
            </div>
            <Button variant="primary" size="md" disabled={!canBuild} onClick={() => go({ kind: 'prompt', text: text.trim() })}>
              Build model
            </Button>
          </div>
        </div>
        {error && <p role="alert" className="mt-3 text-xs text-bad">The project could not be created: {error}</p>}

        <div className="mt-auto mb-8 w-full max-w-[860px] flex flex-col items-center gap-5">
          <div className="flex items-center gap-2 text-[11px] text-muted">
            <span>Connects to the tools you already trust</span>
            <ExampleTag />
            <span className="flex items-center gap-1.5 ml-1">
              {TOOLS.map(t => (
                <span key={t} className="px-2 h-6 inline-flex items-center rounded-full border border-line text-ink/80 bg-white/[0.03]">{t}</span>
              ))}
            </span>
          </div>

          <div className="w-full">
            <ol className="flex items-center justify-between gap-2" aria-label="How a result earns trust">
              {PROOF_STEPS.map((s, i) => (
                <React.Fragment key={s}>
                  <li className="flex items-center gap-2 text-[12px] text-ink animate-rise" style={{ animationDelay: `${420 + i * 110}ms` }}>
                    <span className="num w-5 h-5 inline-flex items-center justify-center rounded-full border border-line-strong text-[10px] text-muted">{i + 1}</span>
                    {s}
                  </li>
                  {i < PROOF_STEPS.length - 1 && <span aria-hidden className="flex-1 h-px bg-gradient-to-r from-line-strong to-line" />}
                </React.Fragment>
              ))}
            </ol>
            <p className="mt-3 text-center text-[11px] text-muted/80">
              What every result goes through. An explainer, not a live status. Results are labelled Simulated or Observed, and Unverified until checked.
            </p>
          </div>
        </div>
      </main>
    </div>
  );
};

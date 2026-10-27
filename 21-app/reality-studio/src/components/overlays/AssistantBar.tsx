import React from 'react';
import { Sparkles, X } from 'lucide-react';
import { Button, IconButton, PreviewTag } from '../ui/UiKit';

// The Assistant (file 37 section 6.17): one rule-based suggestion at a time, shown on the canvas.
// It is never a chat window. Each suggestion states Intent, Proposal, Why and Expected effect, and
// has one action plus Reject. Expected effects describe what will happen, never invented numbers.

export interface AssistantProposal {
  id: string;
  intent: string;
  proposal: string;
  why: string;
  effect: string;
  action: { label: string; run: () => void };
}

export interface AssistantBarProps {
  isOpen: boolean;
  proposal: AssistantProposal | null;
  onClose: () => void;
}

export const AssistantBar: React.FC<AssistantBarProps> = ({ isOpen, proposal, onClose }) => {
  if (!isOpen) return null;

  return (
    <aside
      aria-label="Assistant suggestion"
      className="absolute left-5 bottom-5 z-40 w-[340px] surface-glass animate-rise overflow-hidden"
    >
      <header className="flex items-center justify-between pl-4 pr-2 h-11 border-b border-line">
        <div className="flex items-center gap-2 text-[13px] font-medium text-strong">
          <Sparkles size={14} className="text-assistant" />
          Suggestion
          <PreviewTag />
        </div>
        <IconButton label="Close suggestion" onClick={onClose}><X size={15} /></IconButton>
      </header>

      {proposal ? (
        <div className="p-4 space-y-4">
          <div>
            <p className="text-[11px] text-muted">Intent</p>
            <p className="text-[14px] font-medium text-strong leading-snug">{proposal.intent}</p>
          </div>
          <p className="text-[13px] text-ink leading-relaxed">{proposal.proposal}</p>
          <dl className="space-y-2.5 text-xs">
            <div>
              <dt className="text-muted">Why</dt>
              <dd className="text-ink mt-0.5">{proposal.why}</dd>
            </div>
            <div>
              <dt className="text-muted">Expected effect</dt>
              <dd className="text-ink mt-0.5">{proposal.effect}</dd>
            </div>
          </dl>
          <div className="flex items-center justify-between pt-1">
            <p className="text-[11px] text-muted">Rule-based, not AI.</p>
            <div className="flex items-center gap-1.5">
              <Button size="sm" variant="ghost" onClick={onClose}>Reject</Button>
              <Button size="sm" variant="assistant" onClick={() => { proposal.action.run(); onClose(); }}>{proposal.action.label}</Button>
            </div>
          </div>
        </div>
      ) : (
        <div className="p-4 space-y-1">
          <p className="text-[13px] text-strong">Nothing to suggest here.</p>
          <p className="text-xs text-muted">Suggestions come from rules over your model's state, and appear when a next step is clear.</p>
        </div>
      )}
    </aside>
  );
};

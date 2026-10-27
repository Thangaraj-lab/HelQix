import React, { useEffect, useState } from 'react';
import { Source } from '../../contracts';
import { engine } from '../../engine';
import { Button, Drawer, EmptyState, ExampleTag, PreviewTag, StatusDot } from '../ui/UiKit';

export interface SourcesManagerSheetProps {
  isOpen: boolean;
  onClose: () => void;
  projectId: string;
}

const kindLabel: Record<Source['kind'], string> = {
  connected_tool: 'Connected tool',
  cad_repository: 'CAD repository',
  catalog: 'Catalog',
  manual: 'Manual'
};

export const SourcesManagerSheet: React.FC<SourcesManagerSheetProps> = ({ isOpen, onClose, projectId }) => {
  const [sources, setSources] = useState<Source[] | null>(null);

  useEffect(() => {
    if (!isOpen) return;
    setSources(null);
    engine.listSources(projectId).then(setSources).catch(() => setSources([]));
  }, [isOpen, projectId]);

  return (
    <Drawer isOpen={isOpen} onClose={onClose} title="Sources">
      <div className="space-y-5">
        <p className="text-xs text-muted">Parts and data come only from sources listed here. No vendor data or prices are shown unless a source supplies them.</p>
        {sources === null ? null : sources.length === 0 ? (
          <EmptyState title="No sources connected" description="Parts will be placeholders or defined from scratch." />
        ) : (
          <ul className="space-y-2">
            {sources.map(s => (
              <li key={s.id} className="surface p-3.5 flex items-center justify-between gap-3">
                <div className="min-w-0">
                  <p className="text-[13px] text-strong truncate">{s.name}</p>
                  <p className="text-xs text-muted">{kindLabel[s.kind]}</p>
                </div>
                <span className="flex items-center gap-2 text-xs text-muted shrink-0">
                  <StatusDot tone={s.status === 'connected' ? 'ok' : s.status === 'syncing' ? 'warn' : 'bad'} />
                  <span className="capitalize">{s.status}</span>
                </span>
              </li>
            ))}
          </ul>
        )}
        <div className="flex items-center gap-2">
          <ExampleTag label="Example sources" />
        </div>
        <Button variant="secondary" disabled>Connect a source <PreviewTag /></Button>
      </div>
    </Drawer>
  );
};

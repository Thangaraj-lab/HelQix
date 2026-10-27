import React, { useState } from 'react';
import { useModelData } from '../studio/ModelContext';
import { Chip, EmptyState, PreviewTag, Tabs } from '../../components/ui/UiKit';

// Build (file 37 section 6.16, PREVIEW): architecture, BOM and tests. The BOM lists the revision's
// own parts. Supplier and cost stay Unknown: no source has supplied them, and none are invented.
export const BuildStage: React.FC = () => {
  const { revision } = useModelData();
  const [tab, setTab] = useState<'architecture' | 'bom' | 'tests'>('bom');
  const parts = revision ? Object.values(revision.parts) : [];

  return (
    <div className="relative z-10 flex-1 min-h-0 overflow-y-auto">
      <div className="max-w-[880px] mx-auto px-6 py-7 space-y-5 animate-rise">
        <div className="flex items-end justify-between gap-4">
          <div>
            <h2 className="font-display font-light text-[22px] text-strong flex items-center gap-3">Build <PreviewTag /></h2>
            <p className="text-[13px] text-muted mt-1">What it takes to make the model: parts, how they fit, and how to test them.</p>
          </div>
          <Tabs tabs={[{ id: 'architecture', label: 'Architecture' }, { id: 'bom', label: 'BOM' }, { id: 'tests', label: 'Tests' }]} activeTab={tab} onChange={id => setTab(id as typeof tab)} />
        </div>

        {tab === 'bom' && (
          parts.length === 0 ? (
            <EmptyState title="No parts to list" description="Commit a model first. The bill of materials is built from its parts." />
          ) : (
            <div className="surface overflow-hidden">
              <table className="w-full text-[13px]">
                <thead>
                  <tr className="text-left text-xs text-muted border-b border-line">
                    <th className="font-normal px-4 py-3">Item</th><th className="font-normal px-4 py-3">Qty</th><th className="font-normal px-4 py-3">Maturity</th><th className="font-normal px-4 py-3">Supplier</th><th className="font-normal px-4 py-3">Cost</th>
                  </tr>
                </thead>
                <tbody>
                  {parts.map(p => (
                    <tr key={p.id} className="border-b border-line last:border-0">
                      <td className="px-4 py-3 text-strong">{p.name}</td>
                      <td className="px-4 py-3 num text-ink">1</td>
                      <td className="px-4 py-3"><Chip>{p.maturity.replace(/_/g, ' ')}</Chip></td>
                      <td className="px-4 py-3 text-muted">Unknown</td>
                      <td className="px-4 py-3 text-muted">Data unavailable</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )
        )}
        {tab === 'architecture' && <EmptyState title="Architecture view is not built yet" description="It will show the subsystems and interfaces of the committed model." />}
        {tab === 'tests' && <EmptyState title="No tests defined" description="Test plans derived from requirements will appear here." />}
      </div>
    </div>
  );
};

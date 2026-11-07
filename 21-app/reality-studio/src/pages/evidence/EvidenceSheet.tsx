import React from 'react';
import { short } from '../../app/evidence';
import { engine, engineMode } from '../../engine';
import { Button, Drawer, MockTag, PreviewTag, SectionLabel } from '../../components/ui/UiKit';
import { useModelData } from '../studio/ModelContext';
import { EvidenceExportPanel } from './EvidenceExportPanel';
import { useEvidencePackage } from './usePackage';

export interface EvidenceSheetProps {
  isOpen: boolean;
  onClose: () => void;
}

export const EvidenceSheet: React.FC<EvidenceSheetProps> = ({ isOpen, onClose }) => {
  const pkg = useEvidencePackage(isOpen);
  const { run, requirements } = useModelData();

  return (
    <Drawer isOpen={isOpen} onClose={onClose} title="Evidence">
      <div className="space-y-7">
        <section className="space-y-3">
          <SectionLabel aside={<MockTag />}>Contents preview (hashed in this browser)</SectionLabel>
          {pkg === null ? (
            <p className="text-xs text-muted">Hashing.</p>
          ) : pkg.items.length === 0 ? (
            <p className="text-xs text-muted">Nothing to package yet. Commit a model and run a simulation first.</p>
          ) : (
            <ul className="space-y-2.5">
              {pkg.items.map(i => (
                <li key={i.id} className="surface px-3.5 py-2.5">
                  <p className="text-[13px] text-strong">{i.title}</p>
                  <p className="text-[11px] text-muted">{i.detail}</p>
                  <p className="num text-[11px] text-ink mt-1">{short(i.hash)}</p>
                </li>
              ))}
            </ul>
          )}
        </section>

        <section className="space-y-2">
          <SectionLabel>Root hash</SectionLabel>
          <p className="num text-xs text-ink break-all">{pkg?.rootHash ?? 'Not computed'}</p>
          <p className="text-[11px] text-muted">SHA-256 over the item hashes above: a preview. The signed package has its own manifest and Merkle root, made on the server.</p>
        </section>

        <EvidenceExportPanel client={engine} runId={run?.id ?? null} requirements={requirements} variables={Object.keys(run?.frames[0]?.values ?? {})} engineIsReal={engineMode === 'real'} />

        <div className="flex items-center gap-2">
          <Button variant="secondary" disabled>Generate report <PreviewTag /></Button>
        </div>
      </div>
    </Drawer>
  );
};

import React, { useState } from 'react';
import { Check, Copy } from 'lucide-react';
import { short } from '../../app/evidence';
import { Button, Drawer, MockTag, PreviewTag, SectionLabel } from '../../components/ui/UiKit';
import { useEvidencePackage } from './usePackage';

export interface EvidenceSheetProps {
  isOpen: boolean;
  onClose: () => void;
}

// The command is the real offline verifier (13-verification/crates/helqix-evidence-verify). The
// package directory and trusted key exist only after the evidence service has signed a package.
const VERIFIER = 'helqix-evidence-verify <package-dir> --trusted-key-file <public-key>';

export const EvidenceSheet: React.FC<EvidenceSheetProps> = ({ isOpen, onClose }) => {
  const pkg = useEvidencePackage(isOpen);
  const [copied, setCopied] = useState(false);

  const copy = () => {
    void navigator.clipboard?.writeText(VERIFIER);
    setCopied(true);
    setTimeout(() => setCopied(false), 1800);
  };

  return (
    <Drawer isOpen={isOpen} onClose={onClose} title="Evidence">
      <div className="space-y-7">
        <section className="space-y-3">
          <SectionLabel aside={<MockTag />}>Package contents</SectionLabel>
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
          <p className="text-[11px] text-muted">SHA-256 over the item hashes above.</p>
        </section>

        <section className="space-y-2">
          <SectionLabel aside={<PreviewTag />}>Signature</SectionLabel>
          <p className="text-[13px] text-strong">Not signed.</p>
          <p className="text-xs text-muted">Signing runs in the evidence service, which this build does not call yet. Signing proves origin and integrity, not correct physics.</p>
        </section>

        <section className="space-y-2">
          <SectionLabel>Verify a signed package</SectionLabel>
          <div className="flex items-start gap-2 surface p-3">
            <code className="num text-[11px] text-ink break-all flex-1">{VERIFIER}</code>
            <button onClick={copy} aria-label="Copy verifier command" className="text-muted hover:text-strong shrink-0 mt-0.5">{copied ? <Check size={14} className="text-ok" /> : <Copy size={14} />}</button>
          </div>
        </section>

        <div className="flex items-center gap-2">
          <Button variant="primary" disabled>Export evidence <PreviewTag /></Button>
          <Button variant="secondary" disabled>Generate report <PreviewTag /></Button>
        </div>
      </div>
    </Drawer>
  );
};

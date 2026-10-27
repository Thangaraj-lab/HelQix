import React, { useState } from 'react';
import { useStudio } from '../../stores';
import { checkHdr, LICENCES, Licence, putAsset } from '../../app/assets';
import { Button, Drawer, SectionLabel, Tabs } from '../ui/UiKit';

export interface SettingsSheetProps {
  isOpen: boolean;
  onClose: () => void;
  onToggleTheme: () => void;
  currentTheme: 'dark' | 'light';
}

// Settings sheet: only settings that change something in this build.
export const SettingsSheet: React.FC<SettingsSheetProps> = ({ isOpen, onClose, onToggleTheme, currentTheme }) => {
  const { session } = useStudio();
  const [file, setFile] = useState<File | null>(null);
  const [licence, setLicence] = useState<Licence | ''>('');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const pick = async (f: File | null) => {
    setFile(null); setError(null);
    if (!f) return;
    const c = checkHdr(f.name, await f.arrayBuffer());
    if (c.ok === false) { setError(c.reason); return; }
    setFile(f);
  };

  const apply = async () => {
    if (!file || !licence) return;
    setBusy(true);
    try {
      const asset = await putAsset(file, await file.arrayBuffer());
      session.setEnvironment({ assetId: asset.id, name: file.name, licence });
      setFile(null); setLicence('');
    } catch (e) {
      setError(e instanceof Error ? e.message : 'The file could not be stored.');
    }
    setBusy(false);
  };

  return (
    <Drawer isOpen={isOpen} onClose={onClose} title="Settings">
      <div className="space-y-7">
        <section className="space-y-2.5">
          <SectionLabel>Theme</SectionLabel>
          <Tabs
            tabs={[{ id: 'dark', label: 'Dark' }, { id: 'light', label: 'Light' }]}
            activeTab={currentTheme}
            onChange={id => id !== currentTheme && onToggleTheme()}
          />
        </section>

        <section className="space-y-2.5">
          <SectionLabel>Rendering quality</SectionLabel>
          <Tabs
            tabs={(['Auto', 'High', 'Low'] as const).map(t => ({ id: t, label: t }))}
            activeTab={session.qualityTier}
            onChange={id => session.setQualityTier(id as 'Auto' | 'High' | 'Low')}
          />
          <p className="text-xs text-muted">Lower tiers turn off shadows to keep the canvas smooth on modest hardware.</p>
        </section>

        <section className="space-y-3">
          <SectionLabel>Environment light (HDRI)</SectionLabel>
          {session.environment ? (
            <div className="surface p-3.5 space-y-2">
              <p className="text-[13px] text-strong truncate">{session.environment.name}</p>
              <p className="text-xs text-muted">Licence: {session.environment.licence}. Lights the scene and is the Realistic backdrop.</p>
              <Button size="sm" variant="secondary" onClick={() => session.setEnvironment(null)}>Use the built-in light</Button>
            </div>
          ) : (
            <div className="space-y-3">
              <label className="block space-y-1.5">
                <span className="text-xs text-muted">A Radiance .hdr file (up to 40 MB). 2K is plenty.</span>
                <input type="file" accept=".hdr" onChange={e => void pick(e.target.files?.[0] ?? null)} className="block w-full text-xs text-muted file:mr-3 file:h-8 file:px-3 file:rounded-[8px] file:border file:border-line file:bg-raised file:text-strong" />
              </label>
              {error && <p role="alert" className="text-xs text-bad">{error}</p>}
              <label className="block space-y-1.5">
                <span className="text-xs text-muted">Licence of this file</span>
                <select value={licence} onChange={e => setLicence(e.target.value as Licence)} aria-label="HDRI licence" className="h-8 w-full rounded-[8px] bg-void/70 border border-line px-2 text-xs focus:outline-none focus:border-accent/60">
                  <option value="">Choose a licence</option>{LICENCES.map(l => <option key={l} value={l}>{l}</option>)}
                </select>
              </label>
              <Button size="sm" variant="primary" disabled={!file || !licence || busy} onClick={() => void apply()}>Use this environment</Button>
            </div>
          )}
        </section>
      </div>
    </Drawer>
  );
};

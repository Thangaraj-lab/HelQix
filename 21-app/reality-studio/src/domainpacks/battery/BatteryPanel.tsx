import React, { useEffect, useState } from 'react';
import { ModelOp } from '../../contracts';
import { engine } from '../../engine';
import { Button } from '../../components/ui/UiKit';
import { buildCellOp, packNotice } from './batteryCell';
import type { PackInfo } from './types';

const input = 'h-8 w-full rounded-[8px] bg-void/70 border border-line px-2 text-xs focus:outline-none focus:border-accent/60';

/** Make a part a battery cell: open-circuit voltage, contact resistance and a constant discharge current, with units. The pack's label is shown first. */
export const BatteryPanel: React.FC<{ partId: string; locked: boolean; onApply: (op: ModelOp) => void }> = ({ partId, locked, onApply }) => {
  const [packs, setPacks] = useState<PackInfo[] | null>(null);
  const [t, setT] = useState({ ocv: '', resistance: '', current: '' });
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    if (!engine.listPacks) { setPacks([]); return; }
    engine.listPacks().then(p => live && setPacks(p)).catch(e => { if (live) { setPacks([]); setError(e instanceof Error ? e.message.replace(/^The engine answered \d+:\s*/, '') : 'The battery packs could not be read.'); } });
    return () => { live = false; };
  }, []);

  if (locked || packs === null) return null;
  const pack = packs[0];
  const attach = () => {
    if (!pack) return;
    const r = buildCellOp(partId, pack.id, t);
    if ('error' in r) { setError(r.error); return; }
    setError(null);
    onApply(r.op);
  };
  const field = (k: keyof typeof t, label: string, placeholder = '') => (
    <label className="block space-y-0.5"><span className="text-[11px] text-muted">{label}</span>
      <input value={t[k]} inputMode="decimal" placeholder={placeholder} onChange={e => setT({ ...t, [k]: e.target.value })} aria-label={label} className={input} /></label>
  );
  return (
    <div className="space-y-2" aria-label="Battery cell">
      <p className="text-[11px] text-muted">Battery cell</p>
      {!pack ? <p className="text-xs text-muted">No battery pack is installed on the server, so a part cannot be made a battery cell.</p> : (
        <>
          <p role="note" className="text-xs text-warn rounded-[8px] border border-line p-2">{packNotice(pack)}</p>
          {field('ocv', 'Open-circuit voltage (V)', '4.18')}
          <div className="space-y-1">
            {field('resistance', 'Contact resistance (Ohm)', 'empty means 0')}
            {pack.calibratedContactResistanceOhm !== null && (
              <Button size="sm" variant="secondary" onClick={() => setT({ ...t, resistance: String(pack.calibratedContactResistanceOhm) })}>Use the calibrated value ({pack.calibratedContactResistanceOhm} Ohm)</Button>
            )}
          </div>
          {field('current', 'Discharge current (A)', '5')}
          <Button size="sm" variant="secondary" onClick={attach}>Make this part a battery cell</Button>
        </>
      )}
      {error && <p role="alert" className="text-xs text-bad">{error}</p>}
    </div>
  );
};

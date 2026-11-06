// Domain packs (E5). This directory is the one place where domain words (battery, motor, pump ...) belong in product code: the Studio itself
// stays domain-neutral, and a pack adds what a domain needs (its behaviour type, its panel, its labels). plan rule 2 in src/test/rules.test.ts
// scans every other product file for domain words. The Studio asks this module generic questions (is there a pack model? show the pack's panel).
import React, { useEffect, useState } from 'react';
import type { ModelOp, ModelRevision } from '../contracts';
import { engine } from '../engine';
import { BatteryPanel } from './battery/BatteryPanel';
import { BatteryRunForm } from './battery/BatteryRunForm';
import { cellPartOf, packNotice } from './battery/batteryCell';
import type { BatteryCellBehavior, PackInfo } from './battery/types';

export type { BatteryCellBehavior, PackInfo, PackRunResult } from './battery/types';

/** The behaviours the installed domain packs add to a part, next to the FMU behaviour. */
export type PackBehavior = BatteryCellBehavior;

/** True when a part of the model gets its behaviour from a domain pack. */
export const hasPackModel = (revision: ModelRevision | null | undefined): boolean => !!cellPartOf(revision);

/** The panels the installed packs add to a part's inspector (Define). */
export const PackPanels: React.FC<{ partId: string; locked: boolean; onApply: (op: ModelOp) => void }> = ({ partId, locked, onApply }) => (
  <BatteryPanel partId={partId} locked={locked} onApply={onApply} />
);

/** The run form the installed packs add to Simulate; renders nothing for a model without a pack part. */
export const PackRunForm: React.FC<{ revision: ModelRevision; onRan: (runId: string) => void }> = ({ revision, onRan }) => <BatteryRunForm revision={revision} onRan={onRan} />;

/** What Verify says about a model that comes from a domain pack: the pack's label and published error, read from the server. Nothing for other models. */
export const PackVerifyNote: React.FC<{ revision: ModelRevision | null | undefined }> = ({ revision }) => {
  const [pack, setPack] = useState<PackInfo | null>(null);
  const has = hasPackModel(revision);
  useEffect(() => {
    if (!has || !engine.listPacks) return;
    let live = true;
    engine.listPacks().then(p => live && setPack(p[0] ?? null)).catch(() => undefined);
    return () => { live = false; };
  }, [has]);
  if (!has) return null;
  return <p role="note" className="text-xs text-warn rounded-[8px] border border-line p-2" aria-label="Pack validation">{pack ? packNotice(pack) : 'This model uses a domain pack. Its validation status could not be read from the server, so treat its results as unvalidated.'}</p>;
};


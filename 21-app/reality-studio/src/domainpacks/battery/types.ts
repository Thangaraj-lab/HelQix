import type { Value } from '../../contracts';

/** A battery cell of the battery domain pack: the user states the open-circuit voltage, the contact resistance and a constant discharge
 *  current, each with a unit. The engine converts and checks them (one pack today: `lgm50_spm_chen2020`, PyBaMM SPM with Chen2020). */
export type BatteryCellBehavior = {
  kind: 'battery_cell';
  pack: string;
  initialOcv: Value;
  contactResistance?: Value;
  load: { kind: 'constant_current'; current: Value };
};

/** What the server says about an installed pack: its label and how far to trust it (from the pack's library entry). */
export interface PackInfo {
  id: string;
  /** `NotVerified` or `Verified`, as the trust layer decided. */
  label: string;
  contextOfUse: string;
  /** Held-out RMSE of the terminal voltage in millivolts, one per held-out case. */
  heldOutRmseMv: number[];
  thresholdMv: number;
  /** The contact resistance the pack was calibrated to: offered to the user, never applied for them. */
  calibratedContactResistanceOhm: number | null;
  validatedModelIdentitySha256: string;
  reportDate: string;
}

export interface PackRunResult { runId: string; rows: number; determinism: string; pack: PackInfo; modelIdentitySha256: string }

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

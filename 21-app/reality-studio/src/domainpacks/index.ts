// Domain packs (E5). This directory is the one place where domain words (battery, motor, pump ...) belong in product code: the Studio itself
// stays domain-neutral, and a pack adds what a domain needs (its behaviour type, its panel, its labels). plan rule 2 in src/test/rules.test.ts
// scans every other product file for domain words.
import type { BatteryCellBehavior } from './battery/types';

export type { BatteryCellBehavior } from './battery/types';

/** The behaviours the installed domain packs add to a part, next to the FMU behaviour. */
export type PackBehavior = BatteryCellBehavior;

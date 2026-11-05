import { describe, expect, it } from 'vitest';
import { partsMissingDensity } from '../pages/define/density';

// E1 finding: a model committed without a density cannot be run ("Run my model" names the part and refuses) and a
// committed revision never changes. The Define step warns before the commit, so the user sets the density first.

const part = (id: string, name: string, density?: number) => ({ id, name, params: density === undefined ? {} : { density_kg_m3: { v: density } } });

describe('partsMissingDensity', () => {
  it('lists parts that state no density, by name', () => {
    expect(partsMissingDensity([part('a', 'Frame', 7850), part('b', 'Cover'), part('c', 'Motor')])).toEqual(['Cover', 'Motor']);
  });
  it('is empty when every part states one', () => {
    expect(partsMissingDensity([part('a', 'Frame', 7850)])).toEqual([]);
  });
  it('a zero, negative or non-numeric density counts as missing', () => {
    expect(partsMissingDensity([part('a', 'A', 0), part('b', 'B', -3), part('c', 'C', Number.NaN)])).toEqual(['A', 'B', 'C']);
  });
  it('no parts, nothing missing', () => {
    expect(partsMissingDensity([])).toEqual([]);
  });
});

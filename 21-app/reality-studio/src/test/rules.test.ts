// Plan rules enforced as tests (files 35-37): domain words, honesty strings, mock data honesty.
// These read the source text, so they also catch copy that is hard-coded in components.
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import { describe, expect, it } from 'vitest';
import { MOCK_CAUSAL_CHAIN, MOCK_COMPONENTS, MOCK_VALIDATION_RULES } from '../mock/engineeringData';

const SRC = join(__dirname, '..');

function files(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) return files(p);
    return /\.(ts|tsx)$/.test(name) ? [p] : [];
  });
}

/** Product source: everything under src except mock data and the tests themselves. */
const productFiles = files(SRC).filter((f) => {
  const rel = relative(SRC, f).replace(/\\/g, '/');
  return !rel.startsWith('mock/') && !rel.startsWith('test/');
});

function hits(pattern: RegExp, list = productFiles): string[] {
  const out: string[] = [];
  for (const f of list) {
    readFileSync(f, 'utf8').split('\n').forEach((line, i) => {
      if (pattern.test(line)) out.push(`${relative(SRC, f).replace(/\\/g, '/')}:${i + 1}: ${line.trim().slice(0, 120)}`);
    });
  }
  return out;
}

describe('plan rule 2: no domain parts in product code (only src/mock, labelled Example)', () => {
  it('has no domain words outside src/mock', () => {
    expect(hits(/\b(motors?|batter(y|ies)|shafts?|bearings?|inverters?|drones?|pumps?)\b/i)).toEqual([]);
  });
});

describe('plan rule 4: never fake', () => {
  it('makes no claims about engines HELQIX does not use (UE5, Lumen, Omniverse)', () => {
    expect(hits(/Unreal|Lumen|\bUE5\b|Omniverse|Nanite|Datasmith/, files(SRC))).toEqual([]);
  });

  it('never hard-codes a Verified status in a screen (status must come from data with criteria)', () => {
    expect(hits(/status=["']Verified["']/)).toEqual([]);
  });

  it('never invents project history in copy (save times, run numbers)', () => {
    expect(hits(/Saved \d+ min|Run \d{1,3}\b(?! *\{)|finished \d+ min ago/)).toEqual([]);
  });

  it('never states a computed effect without a run (e.g. "lowers ... by ~14.2 °C")', () => {
    expect(hits(/(lowers|reduces|raises|improves)[^"'<]*~\s*\d/i)).toEqual([]);
  });
});

describe('mock data honesty (ported from the original honesty.test.mjs)', () => {
  it('keeps a failing requirement failing', () => {
    const failing = MOCK_VALIDATION_RULES.find((r) => !r.passed);
    expect(failing).toBeTruthy();
    expect(failing!.passed).toBe(false);
  });

  it('calls unproven causes "Candidate root cause", never "Root cause"', () => {
    const roots = MOCK_CAUSAL_CHAIN.filter((n) => n.isCandidateRoot);
    expect(roots.length).toBeGreaterThan(0);
    for (const n of roots) {
      expect(n.label).toContain('Candidate root cause');
      expect(n.label.startsWith('Root cause:')).toBe(false);
    }
  });

  it('gives every component explicit units', () => {
    for (const c of MOCK_COMPONENTS) {
      expect(c.massUnit?.length).toBeGreaterThan(0);
      expect(c.dimUnit?.length).toBeGreaterThan(0);
    }
  });
});

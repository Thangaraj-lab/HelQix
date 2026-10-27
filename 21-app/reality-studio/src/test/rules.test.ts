// Plan rules enforced as tests (files 35-37): domain words, honesty strings, mock data honesty.
// These read the source text, so they also catch copy that is hard-coded in components.
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import { describe, expect, it } from 'vitest';
import { MockEngineClient } from '../mock/MockEngineClient';
import { scriptedBranchTemperature } from '../mock/branches';

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
    // Product and mock files; this test file itself names the words it forbids.
    const scanned = files(SRC).filter((f) => !relative(SRC, f).replace(/\\/g, '/').startsWith('test/'));
    expect(hits(/Unreal|Lumen|\bUE5\b|Omniverse|Nanite|Datasmith/, scanned)).toEqual([]);
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

describe('plan rule 4 (continued): copy that claims what the build cannot back', () => {
  it('never says Calibrated or Confidence (verification is a checklist, not a score)', () => {
    expect(hits(/Calibrated|Confidence/)).toEqual([]);
  });

  it('never claims a compliance standard (ASIL, ISO 26262, ASME V&V)', () => {
    expect(hits(/ASIL|ISO 26262|ASME V&V/)).toEqual([]);
  });

  it('never uses a browser alert() to fake an action', () => {
    expect(hits(/(^|[^.\w])alert\(|window\.alert\(/)).toEqual([]);
  });

  it('never renders a bare "Verified" badge string outside the verification checklist logic', () => {
    expect(hits(/>\s*Verified\s*</)).toEqual([]);
  });
});

describe('mock engine honesty', () => {
  it('tags every value mock, with a unit, and never verified', async () => {
    for (const t of [0, 4.2, 8.42, 12]) {
      const frame = await MockEngineClient.getFrame('run', t);
      for (const v of Object.values(frame.values)) {
        expect(v.mock).toBe(true);
        expect(v.unit.length).toBeGreaterThan(0);
        expect(v.status).toBe('unverified');
      }
    }
  });

  it('is deterministic: the same time gives identical frames, however it is reached', async () => {
    const a = await MockEngineClient.getFrame('run', 8.42);
    await MockEngineClient.getFrame('run', 2);
    const b = await MockEngineClient.getFrame('run', 8.42);
    expect(JSON.stringify(a)).toBe(JSON.stringify(b));
  });

  it('replays bit-identically: every streamed frame equals getFrame at the same time', async () => {
    for await (const f of MockEngineClient.runFrames('run')) {
      expect(JSON.stringify(await MockEngineClient.getFrame('run', f.t))).toBe(JSON.stringify(f));
    }
  });

  it('scripted branch: no intervention changes nothing, more cooling lowers the peak, and it is repeatable', () => {
    const base: [number, number][] = [[0, 25], [5, 80], [10, 130]];
    expect(scriptedBranchTemperature(base, { coolingPct: 0, loadPct: 0 })).toEqual(base);
    const cooled = scriptedBranchTemperature(base, { coolingPct: 20, loadPct: 0 });
    expect(cooled[2][1]).toBeLessThan(130);
    expect(scriptedBranchTemperature(base, { coolingPct: 20, loadPct: 0 })).toEqual(cooled);
  });
});

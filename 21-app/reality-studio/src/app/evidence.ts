import { RunFrame } from '../contracts';
import { engine } from '../engine';
import type { EngineMode } from '../engine';

// Content hashes and the replay check, computed in the browser with SHA-256 over canonical JSON.
// They describe the data the UI holds. They are not signatures: signing happens in the evidence
// service (helqix-evidence-pack / helqix-evidence-verify), which this build does not call yet.

export async function sha256Hex(text: string): Promise<string> {
  const bytes = new TextEncoder().encode(text);
  const digest = await crypto.subtle.digest('SHA-256', bytes);
  return Array.from(new Uint8Array(digest)).map(b => b.toString(16).padStart(2, '0')).join('');
}

/** JSON with sorted object keys, so equal data hashes equally. */
export function canonicalJson(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(',')}]`;
  if (value && typeof value === 'object') {
    const o = value as Record<string, unknown>;
    return `{${Object.keys(o).sort().map(k => `${JSON.stringify(k)}:${canonicalJson(o[k])}`).join(',')}}`;
  }
  return JSON.stringify(value);
}

export const hashOf = (value: unknown) => sha256Hex(canonicalJson(value));
export const short = (hex: string) => `${hex.slice(0, 8)}…${hex.slice(-6)}`;

export interface ReplayResult {
  matches: boolean;
  /** Frames in the run */
  frames: number;
  /** Frames re-read from the engine and compared (all of them for the mock run, an even sample for a real one) */
  checked: number;
  originalHash: string;
  replayHash: string;
}

/** How many frames of a real run are re-read: one request each would be thousands for a long run. */
export const REAL_REPLAY_SAMPLE = 50;

/** Re-reads frames of run `runId` by time through getFrame and compares them with the streamed frames.
 *  The mock engine re-reads all of them; a real run is checked on an even sample that includes the first and last frame. */
export async function replayCheck(runId: string, original: RunFrame[], mode: EngineMode): Promise<ReplayResult> {
  const n = original.length;
  const picks = mode === 'mock' || n <= REAL_REPLAY_SAMPLE
    ? original.map((_, i) => i)
    : Array.from({ length: REAL_REPLAY_SAMPLE }, (_, k) => Math.round((k * (n - 1)) / (REAL_REPLAY_SAMPLE - 1)));
  const before = picks.map(i => original[i]);
  const replayed: RunFrame[] = [];
  for (const f of before) replayed.push(await engine.getFrame(runId, f.t));
  const [originalHash, replayHash] = await Promise.all([hashOf(before), hashOf(replayed)]);
  return { matches: originalHash === replayHash, frames: n, checked: before.length, originalHash, replayHash };
}

import { RunFrame } from '../contracts';
import { engine } from '../engine';
import { MOCK_RUN_ID } from '../mock/scenarios';

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
  frames: number;
  originalHash: string;
  replayHash: string;
}

/** Re-reads every frame by time through getFrame and compares with the streamed frames. */
export async function replayCheck(original: RunFrame[]): Promise<ReplayResult> {
  const replayed: RunFrame[] = [];
  for (const f of original) replayed.push(await engine.getFrame(MOCK_RUN_ID, f.t));
  const [originalHash, replayHash] = await Promise.all([hashOf(original), hashOf(replayed)]);
  return { matches: originalHash === replayHash, frames: original.length, originalHash, replayHash };
}

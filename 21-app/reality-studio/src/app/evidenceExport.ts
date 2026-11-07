import type { EngineClient, EvidenceKeyInfo, EvidenceRequirement } from '../contracts';
import type { Requirement } from './requirements';

// Signed evidence is built and signed on the server (helqix-evidence-pack); the browser asks for it and saves the ZIP it gets back.
// Nothing here signs anything or holds a key.

/** The Studio's requirement as the evidence service states it. A strict comparison stays strict: it is broken by a value that only equals the limit. */
export function toEvidenceRequirement(r: Requirement): EvidenceRequirement {
  const comparator = ({ '<': 'below', '<=': 'atMost', '>': 'above', '>=': 'atLeast' } as const)[r.op];
  const name = `${r.id}: ${r.text}`.replace(/\s+/g, ' ').trim().slice(0, 120);
  return { name, variable: r.variable, comparator, threshold: r.limit };
}

export interface ExportedEvidence { filename: string; bytes: number; key: EvidenceKeyInfo | null; keyError: string | null }

/** Asks the server for the package, hands the file to `save`, and reads which key signed it. The file is saved even when the key cannot be read. */
export async function exportEvidence(
  client: Pick<EngineClient, 'exportEvidence' | 'evidenceKey'>,
  runId: string,
  requirement: Requirement,
  save: (blob: Blob, filename: string) => void
): Promise<ExportedEvidence> {
  if (!client.exportEvidence) throw new Error('This engine cannot export signed evidence.');
  const { blob, filename } = await client.exportEvidence(runId, toEvidenceRequirement(requirement));
  save(blob, filename);
  let key: EvidenceKeyInfo | null = null;
  let keyError: string | null = null;
  try { key = client.evidenceKey ? await client.evidenceKey() : null; } catch (e) { keyError = e instanceof Error ? e.message : String(e); }
  return { filename, bytes: blob.size, key, keyError };
}

/** Saves a file through the browser's download. */
export function saveBlob(blob: Blob, filename: string): void {
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 10_000);
}

/** The command that checks an exported package offline, after unzipping it next to its public key and attestation. */
export function verifierCommand(runId: string | null): string {
  const folder = runId ? `evidence-${runId}` : 'evidence-<run>';
  return `helqix-evidence-verify ${folder} --trusted-key-file trusted-public-key.txt --attestation ${folder}.intoto.dsse.json`;
}

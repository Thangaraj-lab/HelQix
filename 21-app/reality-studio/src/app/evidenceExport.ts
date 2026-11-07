import type { EngineClient, EvidenceRequirement, EvidenceSigner } from '../contracts';
import type { Requirement } from './requirements';

// Signed evidence is built and signed on the server (helqix-evidence-pack); the browser asks for it and saves the ZIP it gets back.
// Nothing here signs anything or holds a key.

/** The Studio's requirement as the evidence service states it. A strict comparison stays strict: it is broken by a value that only equals the limit. */
export function toEvidenceRequirement(r: Requirement): EvidenceRequirement {
  const comparator = ({ '<': 'below', '<=': 'atMost', '>': 'above', '>=': 'atLeast' } as const)[r.op];
  const name = `${r.id}: ${r.text}`.replace(/\s+/g, ' ').trim().slice(0, 120);
  return { name, variable: r.variable, comparator, threshold: r.limit };
}

/** `signedBy` is the key the server sent beside this very file (null when it sent none: then the key to trust is the one inside the ZIP). */
export interface ExportedEvidence { filename: string; bytes: number; signedBy: EvidenceSigner | null }

/** Asks the server for the package and hands the file to `save`. Which key signed it comes with the file, never from a second request. */
export async function exportEvidence(
  client: Pick<EngineClient, 'exportEvidence'>,
  runId: string,
  requirement: Requirement,
  save: (blob: Blob, filename: string) => void
): Promise<ExportedEvidence> {
  if (!client.exportEvidence) throw new Error('This engine cannot export signed evidence.');
  const { blob, filename, signedBy } = await client.exportEvidence(runId, toEvidenceRequirement(requirement));
  save(blob, filename);
  return { filename, bytes: blob.size, signedBy };
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

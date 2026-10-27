import { useEffect, useState } from 'react';
import { engineMode } from '../../engine';
import { useModelData } from '../studio/ModelContext';
import { hashOf, replayCheck, ReplayResult } from '../../app/evidence';

export interface PackageItem {
  id: string;
  title: string;
  detail: string;
  hash: string;
}

export interface EvidencePackage {
  items: PackageItem[];
  rootHash: string;
  replay: ReplayResult | null;
}

/**
 * What the evidence package would contain for the current revision and run, with SHA-256 content
 * hashes computed here. It is a preview of the package: nothing is signed or written until the
 * evidence service (helqix-evidence-pack) is connected.
 */
export function useEvidencePackage(enabled = true): EvidencePackage | null {
  const { revision, run, requirements, failure } = useModelData();
  const [pkg, setPkg] = useState<EvidencePackage | null>(null);

  useEffect(() => {
    if (!enabled || !revision) { setPkg(null); return; }
    let live = true;
    (async () => {
      const items: PackageItem[] = [];
      let replay: ReplayResult | null = null;
      if (revision) items.push({ id: 'revision', title: `Model revision r${revision.revisionNum}`, detail: `${Object.keys(revision.parts).length} parts, ${Object.keys(revision.connections).length} connections`, hash: await hashOf(revision) });
      if (run) {
        items.push({ id: 'frames', title: engineMode === 'mock' ? 'Run frames (mock)' : `Run frames (${run.id})`, detail: `${run.frames.length} frames`, hash: await hashOf(run.frames) });
        items.push({ id: 'requirement', title: requirements.length ? `Requirement ${requirements.map(r => r.id).join(', ')} result` : 'Requirements', detail: requirements.length ? (failure ? `Broken at ${failure.t.toFixed(2)} s` : 'Not broken in the run') : 'None defined', hash: await hashOf({ requirements, failureT: failure?.t ?? null }) });
        replay = await replayCheck(run.frames);
        items.push({ id: 'replay', title: 'Replay check', detail: replay.matches ? 'Replay matches original' : 'Replay differs', hash: replay.replayHash });
      }
      const rootHash = await hashOf(items.map(i => i.hash));
      if (live) setPkg({ items, rootHash, replay });
    })();
    return () => { live = false; };
  }, [enabled, revision, run, requirements, failure]);

  return pkg;
}

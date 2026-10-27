import { useEffect, useState } from 'react';
import { ModelRevision } from '../../contracts';
import { EXAMPLE_REQUIREMENT } from '../../mock/scenarios';
import { hashOf, replayCheck, ReplayResult } from '../../app/evidence';
import { useRun } from '../simulate/useRun';

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
export function useEvidencePackage(revision: ModelRevision | null): EvidencePackage | null {
  const run = useRun();
  const [pkg, setPkg] = useState<EvidencePackage | null>(null);

  useEffect(() => {
    if (!revision) { setPkg(null); return; }
    let live = true;
    (async () => {
      const items: PackageItem[] = [];
      let replay: ReplayResult | null = null;
      if (revision) items.push({ id: 'revision', title: `Model revision r${revision.revisionNum}`, detail: `${Object.keys(revision.parts).length} parts, ${Object.keys(revision.connections).length} connections`, hash: await hashOf(revision) });
      if (run) {
        items.push({ id: 'frames', title: 'Run frames (mock)', detail: `${run.frames.length} frames`, hash: await hashOf(run.frames) });
        items.push({ id: 'requirement', title: `Requirement ${EXAMPLE_REQUIREMENT.id} result`, detail: run.failureT != null ? `Violated at ${run.failureT.toFixed(2)} s` : 'Not violated', hash: await hashOf({ requirement: EXAMPLE_REQUIREMENT, failureT: run.failureT }) });
        replay = await replayCheck(run.frames);
        items.push({ id: 'replay', title: 'Replay check', detail: replay.matches ? 'Replay matches original' : 'Replay differs', hash: replay.replayHash });
      }
      const rootHash = await hashOf(items.map(i => i.hash));
      if (live) setPkg({ items, rootHash, replay });
    })();
    return () => { live = false; };
  }, [revision, run]);

  return pkg;
}

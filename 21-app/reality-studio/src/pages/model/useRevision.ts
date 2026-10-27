import { useEffect, useState } from 'react';
import { ModelRevision } from '../../contracts';
import { engine } from '../../engine';
import type { ScenePart } from '../../scene/HeroScene';

/** The committed revision named by the URL's rev (for example "r1"), read through the engine. */
export function useRevision(projectId: string, rev: string | null): { revision: ModelRevision | null; loading: boolean } {
  const [revision, setRevision] = useState<ModelRevision | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    const n = rev && /^r(\d+)$/.exec(rev) ? Number(/^r(\d+)$/.exec(rev)![1]) : null;
    if (!n) { setRevision(null); return; }
    let live = true;
    setLoading(true);
    engine.getRevision(projectId, n).then(r => { if (live) { setRevision(r); setLoading(false); } }).catch(() => { if (live) { setRevision(null); setLoading(false); } });
    return () => { live = false; };
  }, [projectId, rev]);

  return { revision, loading };
}

export function revisionToScene(r: ModelRevision | null): ScenePart[] | null {
  if (!r) return null;
  return Object.values(r.parts).map(p => ({
    id: p.id,
    name: p.name,
    geometry: p.geometry,
    pos: [p.transform.pos[0].v, p.transform.pos[1].v, p.transform.pos[2].v] as [number, number, number],
    placeholder: p.maturity === 'placeholder'
  }));
}

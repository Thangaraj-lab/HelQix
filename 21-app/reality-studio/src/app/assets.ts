import { AssetRef } from '../contracts';

// Imported model files (glTF 2.0). The mock engine keeps them in this browser's IndexedDB; the real
// engine will take them through the asset pipeline (Blender, glTF-Transform, Khronos validator,
// licence record). Files are untrusted data: only their geometry and materials are read, and a
// .gltf with links to other files is refused because those files would never be loaded.

export const MAX_ASSET_BYTES = 40 * 1024 * 1024;
export const LICENCES = ['CC0', 'CC BY', 'Own work', 'Other', 'Unknown'] as const;
export type Licence = (typeof LICENCES)[number];

export type AssetCheck = { ok: true; kind: 'glb' | 'gltf' } | { ok: false; reason: string };

/** Cheap structural check before anything is stored or parsed. */
export function checkGltf(name: string, bytes: ArrayBuffer): AssetCheck {
  if (bytes.byteLength === 0) return { ok: false, reason: 'The file is empty.' };
  if (bytes.byteLength > MAX_ASSET_BYTES) return { ok: false, reason: `The file is larger than ${MAX_ASSET_BYTES / 1024 / 1024} MB.` };
  const head = new Uint8Array(bytes, 0, Math.min(4, bytes.byteLength));
  if (head.length === 4 && String.fromCharCode(...head) === 'glTF') {
    const version = new DataView(bytes).getUint32(4, true);
    return version === 2 ? { ok: true, kind: 'glb' } : { ok: false, reason: `Only glTF 2.0 is supported (this is version ${version}).` };
  }
  if (/\.gltf$/i.test(name)) {
    try {
      const json = JSON.parse(new TextDecoder().decode(bytes));
      if (json?.asset?.version !== '2.0') return { ok: false, reason: 'Only glTF 2.0 is supported.' };
      const external = [...(json.buffers ?? []), ...(json.images ?? [])].some((x: { uri?: string }) => x.uri && !x.uri.startsWith('data:'));
      if (external) return { ok: false, reason: 'This .gltf links to other files. Export it as a single .glb instead.' };
      return { ok: true, kind: 'gltf' };
    } catch {
      return { ok: false, reason: 'The file is not valid glTF JSON.' };
    }
  }
  return { ok: false, reason: 'Not a glTF file. Use a .glb or a self-contained .gltf.' };
}

/** A Radiance .hdr environment map (what Poly Haven and similar sources provide). */
export function checkHdr(name: string, bytes: ArrayBuffer): AssetCheck {
  if (bytes.byteLength === 0) return { ok: false, reason: 'The file is empty.' };
  if (bytes.byteLength > MAX_ASSET_BYTES) return { ok: false, reason: `The file is larger than ${MAX_ASSET_BYTES / 1024 / 1024} MB.` };
  const head = new TextDecoder().decode(new Uint8Array(bytes, 0, Math.min(10, bytes.byteLength)));
  if (!/\.hdr$/i.test(name) || !(head.startsWith('#?RADIANCE') || head.startsWith('#?RGBE'))) return { ok: false, reason: 'Not a Radiance .hdr file.' };
  return { ok: true, kind: 'glb' };
}

const DB = 'helqix_assets';
const STORE = 'files';

function open(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB, 1);
    req.onupgradeneeded = () => req.result.createObjectStore(STORE);
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

export async function putAsset(file: File, bytes: ArrayBuffer): Promise<AssetRef> {
  const id = `asset-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 7)}`;
  const db = await open();
  await new Promise<void>((resolve, reject) => {
    const tx = db.transaction(STORE, 'readwrite');
    tx.objectStore(STORE).put(bytes, id);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });
  return { id, name: file.name, sizeBytes: bytes.byteLength, mimeType: file.type || 'model/gltf-binary', uri: `idb:${id}` };
}

export async function getAsset(id: string): Promise<ArrayBuffer | null> {
  const db = await open();
  return new Promise((resolve, reject) => {
    const req = db.transaction(STORE).objectStore(STORE).get(id);
    req.onsuccess = () => resolve((req.result as ArrayBuffer | undefined) ?? null);
    req.onerror = () => reject(req.error);
  });
}

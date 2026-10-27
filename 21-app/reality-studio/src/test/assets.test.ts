import { describe, expect, it } from 'vitest';
import { checkGltf } from '../app/assets';

const glb = (version: number) => { const b = new ArrayBuffer(12); const dv = new DataView(b); [0x67, 0x6c, 0x54, 0x46].forEach((c, i) => dv.setUint8(i, c)); dv.setUint32(4, version, true); return b; };
const text = (o: unknown) => new TextEncoder().encode(JSON.stringify(o)).buffer as ArrayBuffer;

describe('glTF import checks (files are untrusted)', () => {
  it('accepts a glTF 2.0 binary and refuses other versions', () => {
    expect(checkGltf('a.glb', glb(2))).toEqual({ ok: true, kind: 'glb' });
    expect(checkGltf('a.glb', glb(1)).ok).toBe(false);
  });
  it('accepts a self-contained .gltf and refuses one that links to other files', () => {
    expect(checkGltf('a.gltf', text({ asset: { version: '2.0' }, buffers: [{ uri: 'data:application/octet-stream;base64,AA==' }] })).ok).toBe(true);
    const r = checkGltf('a.gltf', text({ asset: { version: '2.0' }, buffers: [{ uri: 'model.bin' }] }));
    expect(r.ok).toBe(false);
  });
  it('refuses empty files and non-glTF content', () => {
    expect(checkGltf('a.glb', new ArrayBuffer(0)).ok).toBe(false);
    expect(checkGltf('a.obj', new TextEncoder().encode('v 0 0 0').buffer as ArrayBuffer).ok).toBe(false);
  });
});

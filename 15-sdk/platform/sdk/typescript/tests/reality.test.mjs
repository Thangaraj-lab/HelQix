import assert from 'node:assert/strict';
import test from 'node:test';
import { createReality, validateReality } from '../dist/reality.js';

const validReality = () => ({
  schemaVersion: 1,
  id: 'reality-1',
  metadata: { name: 'Demo', tags: [], extensions: {} },
  components: [{ id: 'component-1', type: 'transform', data: { x: 1 } }],
});

test('strict reality decoder accepts the shared SDK wire contract', () => {
  const reality = createReality(validReality());
  assert.equal(reality.id, 'reality-1');
  assert.deepEqual(reality.components[0].data, { x: 1 });
});

test('strict reality decoder rejects unknown fields at every object boundary', () => {
  const base = validReality();
  const fixtures = [
    { ...base, unexpected: true },
    { ...base, metadata: { ...base.metadata, unexpected: true } },
    { ...base, components: [{ ...base.components[0], unexpected: true }] },
  ];
  for (const fixture of fixtures) assert.ok(validateReality(fixture).length > 0);
});

test('strict reality decoder requires version and component collection', () => {
  const base = validReality();
  for (const field of ['schemaVersion', 'components']) {
    const fixture = { ...base };
    delete fixture[field];
    assert.ok(validateReality(fixture).length > 0, `${field} should be required`);
  }
});

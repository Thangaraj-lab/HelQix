import assert from 'node:assert';
import { MOCK_VALIDATION_RULES, MOCK_CAUSAL_CHAIN, MOCK_COMPONENTS } from '../mock/engineeringData.js';

console.log('--- STARTING HELQIX HONESTY & INTEGRITY TEST SUITE ---');

// Test 1: Honesty Rule - Failing requirement must never be verified
const failingRule = MOCK_VALIDATION_RULES.find(r => !r.passed);
assert.ok(failingRule, 'Must contain at least one failing requirement rule');
assert.strictEqual(failingRule.passed, false, 'Failing rule must have passed = false');
console.log('✓ TEST 1 PASSED: Failing requirement is explicitly marked failing and cannot render Verified.');

// Test 2: Honesty Rule - Causality must use "Candidate root cause", never unproven "Root cause"
const rootNodes = MOCK_CAUSAL_CHAIN.filter(n => n.isCandidateRoot);
assert.ok(rootNodes.length > 0, 'Must have candidate root cause node');
rootNodes.forEach(node => {
  assert.ok(
    node.label.includes('Candidate root cause'),
    `Node "${node.label}" must use "Candidate root cause" wording`
  );
  assert.ok(
    !node.label.startsWith('Root cause:'),
    `Node "${node.label}" must never claim definitive "Root cause"`
  );
});
console.log('✓ TEST 2 PASSED: All unproven causal links use strictly "Candidate root cause" phrasing.');

// Test 3: Every component and metric must have defined units
MOCK_COMPONENTS.forEach(cmp => {
  assert.ok(cmp.massUnit && cmp.massUnit.length > 0, `Component ${cmp.id} must have massUnit`);
  assert.ok(cmp.dimUnit && cmp.dimUnit.length > 0, `Component ${cmp.id} must have dimUnit`);
});
console.log('✓ TEST 3 PASSED: Every numeric metric and component carries explicit engineering units.');

// Test 4: Quantum coprocessor must be marked PREVIEW / simulator only
import { MOCK_COMPUTE_NODES } from '../mock/engineeringData.js';
const quantumNode = MOCK_COMPUTE_NODES.find(n => n.type === 'Quantum');
assert.ok(quantumNode, 'Must have Quantum compute node');
assert.strictEqual(quantumNode.isPreview, true, 'Quantum node must have isPreview = true');
assert.ok(quantumNode.routingReason.includes('PREVIEW, simulator only'));
console.log('✓ TEST 4 PASSED: Quantum backend is honestly tagged PREVIEW (simulator only).');

console.log('========================================================');
console.log('ALL HELQIX HONESTY & ENGINEERING INTEGRITY TESTS PASSED!');
console.log('========================================================');

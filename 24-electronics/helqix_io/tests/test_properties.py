"""Randomized serial-link properties, one case per seed.

Nightly-only (see verify.py --nightly): 20 seeds, each with a random fault
profile, must be reproducible and must never alter or reorder a delivered frame.
"""

import random

import pytest

from helqix_io import IOTimeoutError, serial_pair

pytestmark = pytest.mark.nightly

SEEDS = list(range(20))


def _is_subsequence(items, sequence):
    iterator = iter(sequence)
    return all(any(item == candidate for candidate in iterator) for item in items)


@pytest.mark.parametrize("seed", SEEDS)
def test_random_faults_never_corrupt_or_reorder_frames(seed):
    rng = random.Random(seed)
    payloads = [bytes([index]) + bytes(rng.randrange(256) for _ in range(rng.randrange(0, 64)))
                for index in range(50)]
    drop = rng.random() * 0.4
    corrupt = rng.random() * 0.4
    delay_s = rng.random() * 0.002

    def run():
        left, right = serial_pair(capacity_frames=64)
        right.configure_faults(seed=seed, drop=drop, corrupt=corrupt, delay_s=delay_s)
        for payload in payloads:
            left.write(payload, timeout_s=0.5)
        delivered = []
        while True:
            try:
                delivered.append(right.read(timeout_s=0.01))
            except IOTimeoutError:
                break
        return delivered, right.stats.dropped, right.stats.corrupted

    first = run()
    assert first == run(), "the same seed and profile must be reproducible"
    delivered, dropped, corrupted = first
    assert dropped + corrupted + len(delivered) == len(payloads)
    assert _is_subsequence(delivered, payloads)

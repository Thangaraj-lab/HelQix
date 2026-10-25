"""Benchmarks for the solver path used by the battery domain pack (PyBaMM SPM, Chen2020).

Each expected value is derived in closed form here, independently of the solver:
- charge conservation: discharged charge = I * t
- lithium mass conservation in the negative particles: mean concentration falls by
  I * t / (F * eps_s * L_n * W * H * n_electrodes)
- spatial convergence: voltage error against a fine reference mesh shrinks with refinement
- determinism: identical inputs give identical outputs on the same machine and build
"""
import math
import unittest
from dataclasses import replace

import numpy as np
import pybamm

from helqix_battery import model

CURRENT_A = 5.0
SECONDS = 1800.0
TIMES = [float(t) for t in range(0, int(SECONDS) + 1, 30)]
CURRENTS = [CURRENT_A] * len(TIMES)
V0 = 4.18


def solution(config=model.ModelConfig()):
    return model.solve(TIMES, CURRENTS, V0, config)


class SpmBenchmarks(unittest.TestCase):
    def test_discharged_charge_equals_current_times_time(self):
        q = solution()["Discharge capacity [A.h]"].entries
        expected = [CURRENT_A * t / 3600.0 for t in TIMES]
        for got, want in zip(q, expected):
            self.assertAlmostEqual(got, want, delta=1e-9 + 1e-7 * want)

    def test_negative_particle_lithium_is_conserved(self):
        p = pybamm.ParameterValues("Chen2020")
        volume = (p["Negative electrode active material volume fraction"] * p["Negative electrode thickness [m]"]
                  * p["Electrode width [m]"] * p["Electrode height [m]"] * p["Number of electrodes connected in parallel to make a cell"])
        c = solution()["Average negative particle concentration [mol.m-3]"].entries
        for t, value in zip(TIMES, c):
            expected_drop = CURRENT_A * t / (pybamm.constants.F.value * volume)
            self.assertAlmostEqual(c[0] - value, expected_drop, delta=1e-6 * c[0])

    def test_voltage_converges_under_mesh_refinement(self):
        reference = np.array(solution(replace(model.ModelConfig(), mesh_scale=8))["Voltage [V]"].entries)
        errors = []
        for scale in (1, 2, 4):
            v = np.array(solution(replace(model.ModelConfig(), mesh_scale=scale))["Voltage [V]"].entries)
            errors.append(float(np.max(np.abs(v - reference))))
        self.assertTrue(errors[0] > errors[1] > errors[2], errors)
        orders = [math.log2(errors[0] / errors[1]), math.log2(errors[1] / errors[2])]
        self.assertGreaterEqual(min(orders), 1.0, f"observed orders {orders}, errors {errors}")

    def test_same_inputs_give_bitwise_identical_voltage(self):
        a = model.simulate(TIMES, CURRENTS, V0, model.ModelConfig())
        b = model.simulate(TIMES, CURRENTS, V0, model.ModelConfig())
        self.assertEqual([x.hex() for x in a], [x.hex() for x in b])


if __name__ == "__main__":
    unittest.main()

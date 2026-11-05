"""The FMI runner's unit check (E4.3a): the FMU's own UnitDefinitions are authoritative, offsets are refused, and a
parameter stated in SI is converted to the unit the FMU declares. Needs neither FMPy nor an FMU: the model description is faked."""
import importlib.util
import json
import subprocess
import sys
import unittest
from pathlib import Path
from types import SimpleNamespace as NS

from helqix_fmi import cli


def base(m=0, kg=0, s=0, A=0, K=0, mol=0, cd=0, rad=0, factor=1.0, offset=0.0):
    return NS(m=m, kg=kg, s=s, A=A, K=K, mol=mol, cd=cd, rad=rad, factor=factor, offset=offset)


def md_with(*units):
    return NS(unitDefinitions=[NS(name=n, baseUnit=b) for n, b in units], modelVariables=[])


class UnitCheck(unittest.TestCase):
    def test_dimensionless(self):
        self.assertEqual(cli.unit_factor_and_dimension("1"), (1.0, [0] * 7))
        self.assertEqual(cli.unit_factor_and_dimension(""), (1.0, [0] * 7))

    def test_the_fmu_definition_gives_the_factor_and_the_dimension(self):
        md = md_with(("mm", base(m=1, factor=1e-3)), ("m/s2", base(m=1, s=-2)), ("kN", base(kg=1, m=1, s=-2, factor=1000.0)))
        self.assertEqual(cli.unit_factor_and_dimension("mm", md), (1e-3, [1, 0, 0, 0, 0, 0, 0]))
        self.assertEqual(cli.unit_factor_and_dimension("m/s2", md), (1.0, [1, 0, -2, 0, 0, 0, 0]))
        self.assertEqual(cli.unit_factor_and_dimension("kN", md), (1000.0, [1, 1, -2, 0, 0, 0, 0]))

    def test_an_offset_unit_is_refused(self):
        md = md_with(("degC", base(K=1, offset=273.15)))
        with self.assertRaises(ValueError) as e:
            cli.unit_factor_and_dimension("degC", md)
        self.assertIn("offset", str(e.exception))

    def test_a_unit_nobody_knows_is_refused(self):
        with self.assertRaises(ValueError):
            cli.unit_factor_and_dimension("furlongs-per-fortnight-squared-ish", md_with())

    @unittest.skipUnless(importlib.util.find_spec("pint"), "Pint is not installed")
    def test_pint_is_the_fallback_for_a_plain_unit_the_fmu_does_not_define(self):
        self.assertEqual(cli.unit_factor_and_dimension("km", md_with()), (1000.0, [1, 0, 0, 0, 0, 0, 0]))
        with self.assertRaises(ValueError):
            cli.unit_factor_and_dimension("degC", md_with())

    def test_si_values_are_converted_to_the_fmu_unit(self):
        md = md_with(("mm", base(m=1, factor=1e-3)))
        var = NS(name="radius", unit="mm", declaredType=None)
        out = cli.si_start_values({"radius": {"value_si": 0.25, "dimension": [1, 0, 0, 0, 0, 0, 0]}}, {"radius": var}, md)
        self.assertAlmostEqual(out["radius"], 250.0, places=9)

    def test_a_dimension_mismatch_exits_with_code_2(self):
        md = md_with(("mm", base(m=1, factor=1e-3)))
        var = NS(name="radius", unit="mm", declaredType=None)
        with self.assertRaises(SystemExit) as e:
            cli.si_start_values({"radius": {"value_si": 0.25, "dimension": [0, 1, 0, 0, 0, 0, 0]}}, {"radius": var}, md)
        self.assertEqual(e.exception.code, 2)


if __name__ == "__main__":
    unittest.main()

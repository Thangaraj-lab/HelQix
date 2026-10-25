"""End-to-end run of the validation pipeline on synthetic cycler files (no network, no real data)."""
import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from helqix_battery import model, validate

HEADER = ("Name:,synthetic,,,,,,,,,,,,,\n,,,,,,,,,,,,,,\n"
          "Rec,Cycle P,Cycle C,Step,Test Time [s],Step Time [s],Capacity [Ah],Energy [Wh],Current [A],Voltage [V],Md,ES,DPT Time,Temperature Cell [degC],Temperature Chamber [degC]\n")
TRUE_R = 0.015


def synthetic_cell_csv() -> str:
    """Charge, rest, then discharges at C/2, 1C and C/10, each followed by a rest; voltages from the model."""
    lines, rec, t = [HEADER], 1, 0.0
    step = 1

    def emit(mode, currents, voltages):
        nonlocal rec, t, step
        for i, v in zip(currents, voltages):
            lines.append(f"{rec},0,1,{step},{t},{t},0,0,{i},{v},{mode},0,x,25.0,25.0\n")
            rec += 1
            t += 30.0
        step += 1

    for current, seconds in ((2.5, 1200), (5.0, 900), (0.5, 1800)):
        emit("C", [1.5, 1.5], [4.0, 4.2])
        emit("R", [0.0, 0.0], [4.19, 4.18])
        times = [30.0 * k for k in range(seconds // 30 + 10)]
        currents = [current] * (seconds // 30) + [0.0] * 10
        volts = model.simulate(times, currents, 4.18, model.ModelConfig(contact_resistance_ohm=TRUE_R))
        n = seconds // 30
        emit("D", currents[:n], volts[:n])
        emit("R", currents[n:], volts[n:])
    return "".join(lines)


class ValidatePipelineTests(unittest.TestCase):
    def test_pipeline_calibrates_then_reports_held_out_cases(self):
        with tempfile.TemporaryDirectory() as tmp:
            data, out = Path(tmp) / "data", Path(tmp) / "out"
            data.mkdir()
            files = {}
            for name in ("SYN_cell01.csv", "SYN_cell02.csv"):
                body = synthetic_cell_csv().encode("utf-8")
                (data / name).write_bytes(body)
                files[name] = hashlib.md5(body).hexdigest()
            result = validate.run(data, out, date="2026-10-25", files=files, expected_split_sha256=None, sensitivity_step=0.10)
            report = json.loads((out / "report.json").read_text(encoding="utf-8"))
            self.assertEqual(report["schema"], "helqix.validation-report.v1")
            self.assertAlmostEqual(report["calibration"]["contact_resistance_ohm"], TRUE_R, delta=3e-4)
            held = [c for c in report["cases"] if c["role"] == "validation"]
            self.assertEqual(sorted(c["id"] for c in held), ["cell01_c10", "cell02_c10"])
            self.assertFalse(set(report["split"]["fitted_on"]) & {c["id"] for c in held})
            self.assertTrue(report["numerical_integrity"]["passed"])
            self.assertIn("uncertainty", report)
            self.assertIn("sensitivity", report)
            self.assertIn("reference_unfitted_rmse_v", report)
            self.assertTrue((out / "summary.md").is_file())
            self.assertEqual(result["split_sha256"], report["split"]["sha256"])

    def test_refuses_a_split_that_differs_from_the_record(self):
        with tempfile.TemporaryDirectory() as tmp:
            data, out = Path(tmp) / "data", Path(tmp) / "out"
            data.mkdir()
            body = synthetic_cell_csv().encode("utf-8")
            (data / "SYN_cell01.csv").write_bytes(body)
            with self.assertRaises(validate.SplitMismatch):
                validate.run(data, out, date="2026-10-25", files={"SYN_cell01.csv": hashlib.md5(body).hexdigest()}, expected_split_sha256="0" * 64)
            self.assertFalse((out / "report.json").exists())

    def test_cli_pins_the_recorded_split(self):
        self.assertEqual(validate.EXPECTED_SPLIT_SHA256, "9858fefda7fc8f779601bad7f25b0567ca4d0e676d8a69e8ac617fb9f28031db")


if __name__ == "__main__":
    unittest.main()

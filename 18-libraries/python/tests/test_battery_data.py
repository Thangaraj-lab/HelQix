import hashlib
import io
import math
import tempfile
import unittest
from pathlib import Path

from helqix_battery import cases, dataset, maccor, metrics, split

HEADER = (
    "Name:,LG_M50_validation.038,,,,,,,,,,,,,\n"
    "Channel:,38,,,,,,,,,,,,,\n"
    ",,,,,,,,,,,,,,\n"
    "Rec,Cycle P,Cycle C,Step,Test Time [s],Step Time [s],Capacity [Ah],Energy [Wh],Current [A],Voltage [V],Md,ES,DPT Time,Temperature Cell [degC],Temperature Chamber [degC]\n"
)


def maccor_csv(segments):
    """segments: list of (step, mode, current, [voltages]); 10 s per sample."""
    lines, rec, t = [HEADER], 1, 0.0
    for step, mode, current, voltages in segments:
        for v in voltages:
            lines.append(f"{rec},0,1,{step},{t},{t},0,0,{current},{v},{mode},0,01/01/2019 00:00,25.0,25.0\n")
            rec += 1
            t += 10.0
    return "".join(lines)


PROTOCOL = [
    (1, "R", 0, [3.13, 3.13]),
    (2, "D", 0.5, [3.11, 2.9, 2.5]),           # discharge from unknown state: must be excluded
    (3, "R", 0, [2.52, 2.8]),
    (4, "C", 1.5, [2.9, 4.2]),
    (5, "C", 0.5, [4.2, 4.2]),
    (6, "R", 0, [4.199, 4.185]),
    (7, "D", 0.5, [4.17, 3.9, 3.6, 2.5]),      # C/10
    (8, "R", 0, [2.52, 2.8]),
    (8, "SBr", 0, [2.88]),
    (8, "R", 0, [2.9]),
    (9, "C", 1.5, [2.96, 4.2]),
    (10, "C", 0.5, [4.2, 4.2]),
    (11, "R", 0, [4.198, 4.186]),
    (12, "D", 2.5, [4.10, 3.7, 2.5]),          # C/2
    (13, "R", 0, [2.58, 3.0]),
]


def write(text):
    handle = tempfile.NamedTemporaryFile("w", suffix=".csv", delete=False, encoding="utf-8")
    handle.write(text)
    handle.close()
    return Path(handle.name)


class DatasetTests(unittest.TestCase):
    def test_registry_pins_the_published_zenodo_record(self):
        self.assertEqual(dataset.DATASET["doi"], "10.5281/zenodo.4032561")
        self.assertEqual(dataset.DATASET["licence"], "CC-BY-4.0")
        self.assertEqual(dataset.DATASET["files"]["LGM50_cell02.csv"], "0bbf959cbfc367296ec69d7a74d2f5c1")

    def test_verify_accepts_matching_and_rejects_changed_file(self):
        path = write("abc")
        dataset.verify_file(path, hashlib.md5(b"abc").hexdigest())
        with self.assertRaises(dataset.DatasetIntegrityError):
            dataset.verify_file(path, "0" * 32)

    def test_fetch_verifies_and_never_keeps_a_corrupt_download(self):
        good = {name: b"payload-" + name.encode() for name in dataset.DATASET["files"]}
        files = {name: hashlib.md5(body).hexdigest() for name, body in good.items()}
        with tempfile.TemporaryDirectory() as tmp:
            paths = dataset.fetch(Path(tmp), files=files, opener=lambda url: io.BytesIO(good[url.split("/files/")[1].split("?")[0]]))
            self.assertEqual(sorted(paths), sorted(files))
            for name, path in paths.items():
                self.assertEqual(path.read_bytes(), good[name])
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaises(dataset.DatasetIntegrityError):
                dataset.fetch(Path(tmp), files=files, opener=lambda url: io.BytesIO(b"tampered"))
            self.assertEqual(list(Path(tmp).iterdir()), [])


class MaccorTests(unittest.TestCase):
    def test_reads_segments_by_step_and_mode(self):
        steps = maccor.read_maccor_csv(write(maccor_csv(PROTOCOL)))
        self.assertEqual([(s.number, s.mode) for s in steps][:4], [(1, "R"), (2, "D"), (3, "R"), (4, "C")])
        seven = next(s for s in steps if s.number == 7)
        self.assertEqual(seven.voltage_v, [4.17, 3.9, 3.6, 2.5])
        self.assertEqual(seven.time_s[1] - seven.time_s[0], 10.0)

    def test_missing_column_is_a_format_error(self):
        broken = maccor_csv(PROTOCOL).replace("Voltage [V]", "Volts")
        with self.assertRaises(maccor.MaccorFormatError):
            maccor.read_maccor_csv(write(broken))

    def test_non_numeric_cell_is_a_format_error(self):
        broken = maccor_csv(PROTOCOL).replace(",4.17,", ",abc,", 1)
        with self.assertRaises(maccor.MaccorFormatError):
            maccor.read_maccor_csv(write(broken))


class CaseExtractionTests(unittest.TestCase):
    def setUp(self):
        self.extracted, self.excluded = cases.extract_cases("cell02", maccor.read_maccor_csv(write(maccor_csv(PROTOCOL))))

    def test_only_discharges_after_charge_and_rest_become_cases(self):
        self.assertEqual([c.id for c in self.extracted], ["cell02_c10", "cell02_c2"])
        self.assertEqual(len(self.excluded), 1)
        self.assertIn("not preceded by a charge", self.excluded[0][1])

    def test_case_includes_following_rest_and_starts_at_zero(self):
        c10 = self.extracted[0]
        self.assertEqual(c10.rate, "c10")
        self.assertEqual(c10.time_s[0], 0.0)
        self.assertEqual(c10.current_a, [0.5, 0.5, 0.5, 0.5, 0, 0, 0, 0])
        self.assertEqual(c10.voltage_v[-1], 2.9)
        self.assertEqual(c10.discharge_samples, 4)
        self.assertEqual(c10.initial_rest_voltage_v, 4.185)

    def test_unrecognised_current_is_excluded_with_reason(self):
        odd = [seg if seg[0] != 12 else (12, "D", 3.3, [4.1, 3.0]) for seg in PROTOCOL]
        extracted, excluded = cases.extract_cases("cell02", maccor.read_maccor_csv(write(maccor_csv(odd))))
        self.assertEqual([c.id for c in extracted], ["cell02_c10"])
        self.assertTrue(any("3.3" in reason for _, reason in excluded))


class SplitTests(unittest.TestCase):
    def test_golden_vectors_match_rust(self):
        a = split.data_sha256([0.0, 1.0], [0.5, 0.5], [4.0, 3.9])
        b = split.data_sha256([0.0, 2.0], [5.0, 5.0], [4.0, 3.5])
        self.assertEqual(a, "648fa04cd43ea396914ad5b01b64bdf9dd10ef3f3a52753597e80a335d47ac26")
        self.assertEqual(b, "b068e0e17017c6fc0282c459fd58acdeff43b556da9c889cde7b60e945f7514b")
        digest = split.split_sha256([("cell_a_c10", "validation", a), ("cell_a_1c", "calibration", b)])
        self.assertEqual(digest, "eadf27faeff4c6511645295ab731e7e924ed568baf4a26999e9241ead16fa819")

    def test_c10_is_held_out_and_other_rates_calibrate(self):
        extracted, _ = cases.extract_cases("cell02", maccor.read_maccor_csv(write(maccor_csv(PROTOCOL))))
        s = split.make_split(extracted)
        self.assertEqual([c.id for c in s.validation], ["cell02_c10"])
        self.assertEqual([c.id for c in s.calibration], ["cell02_c2"])
        self.assertEqual(len(s.sha256), 64)
        self.assertEqual(s.role_of("cell02_c10"), "validation")

    def test_calibration_set_cannot_hold_held_out_data(self):
        extracted, _ = cases.extract_cases("cell02", maccor.read_maccor_csv(write(maccor_csv(PROTOCOL))))
        s = split.make_split(extracted)
        calib = s.calibration_set()
        self.assertEqual([c.id for c in calib.cases], ["cell02_c2"])
        with self.assertRaises(split.SplitViolation):
            split.CalibrationSet(s, list(s.validation), token=None)
        with self.assertRaises(split.SplitViolation):
            split.require_calibration_set(list(s.validation))

    def test_unknown_rate_cannot_enter_a_split(self):
        extracted, _ = cases.extract_cases("cell02", maccor.read_maccor_csv(write(maccor_csv(PROTOCOL))))
        stray = cases.Case(id="cell02_c5", cell="cell02", rate="c5", time_s=[0.0, 1.0], current_a=[1.0, 1.0], voltage_v=[4.0, 3.9], discharge_samples=2, initial_rest_voltage_v=4.18)
        with self.assertRaises(split.SplitViolation):
            split.make_split(extracted + [stray])


class MetricTests(unittest.TestCase):
    def test_rmse_and_peak(self):
        self.assertAlmostEqual(metrics.rmse([1.0, 2.0, 3.0], [1.0, 2.0, 5.0]), math.sqrt(4 / 3))
        self.assertEqual(metrics.peak_abs_error([1.0, 2.0], [1.5, 1.0]), 1.0)

    def test_nearest_rank_percentile(self):
        measured = [0.0] * 20
        simulated = [float(k) for k in range(1, 21)]  # |errors| 1..20
        self.assertEqual(metrics.percentile_abs_error(simulated, measured, 95.0), 19.0)
        self.assertEqual(metrics.percentile_abs_error(simulated, measured, 100.0), 20.0)
        self.assertEqual(metrics.percentile_abs_error(simulated, measured, 50.0), 10.0)

    def test_mismatched_or_non_finite_series_are_rejected(self):
        with self.assertRaises(ValueError):
            metrics.rmse([1.0], [1.0, 2.0])
        with self.assertRaises(ValueError):
            metrics.rmse([1.0, float("nan")], [1.0, 2.0])
        with self.assertRaises(ValueError):
            metrics.rmse([], [])


if __name__ == "__main__":
    unittest.main()

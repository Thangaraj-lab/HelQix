"""Break formulas on purpose; a named test must fail for each mutation.

Each entry is a source replacement applied to a throwaway copy of
24-electronics/. A mutation that no test catches is reported as a survivor.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent
IGNORE = shutil.ignore_patterns(".venv", "__pycache__", ".pytest_cache", "*.pyc", "docs", "schemas")

MUTATIONS = [
    {
        "phase": "E1",
        "name": "resistor conductance halved",
        "file": "helqix_circuit/mna.py",
        "old": "g = 1.0 / component.value",
        "new": "g = 1.0 / (2.0 * component.value)",
        "test": "helqix_circuit/tests/test_dc.py::test_voltage_divider_matches_hand_formula",
    },
    {
        "phase": "E1",
        "name": "ohm scale drifted by 1e-7",
        "file": "helqix_circuit/units.py",
        "old": '"ohm": 1.0,',
        "new": '"ohm": 1.0000001,',
        "test": "helqix_circuit/tests/test_dc.py::test_voltage_divider_matches_hand_formula",
    },
    {
        "phase": "E1",
        "name": "current source injection sign flipped",
        "file": "helqix_circuit/mna.py",
        "old": "rhs[a] -= component.value",
        "new": "rhs[a] += component.value",
        "test": "helqix_circuit/tests/test_dc.py::test_current_source_sign_follows_spice_convention",
    },
    {
        "phase": "E1",
        "name": "VCCS output-row sign flipped",
        "file": "helqix_circuit/mna.py",
        "old": "add(a, c, gm)",
        "new": "add(a, c, -gm)",
        "test": "helqix_circuit/tests/test_dc.py::test_vcvs_and_vccs_stamps",
    },
    {
        "phase": "E1",
        "name": "voltage-source loop check disabled",
        "file": "helqix_circuit/mna.py",
        "old": "def _check_voltage_loops(circuit: Circuit) -> None:\n    union = _UnionFind()",
        "new": "def _check_voltage_loops(circuit: Circuit) -> None:\n    return\n    union = _UnionFind()",
        "test": "helqix_circuit/tests/test_dc.py::test_voltage_source_loop_is_refused_with_names",
    },
    {
        "phase": "E2",
        "name": "trapezoidal capacitor factor turned into backward Euler",
        "file": "helqix_circuit/core.py",
        "old": "g = 2.0 * capacitance / step_s",
        "new": "g = capacitance / step_s",
        "test": "helqix_circuit/tests/test_transient.py::test_rc_charge_trapezoidal_within_half_percent",
    },
    {
        "phase": "E2",
        "name": "diode saturation current doubled",
        "file": "helqix_circuit/devices.py",
        "old": "current = params[\"is_a\"] * (exp_term - 1.0)",
        "new": "current = 2.0 * params[\"is_a\"] * (exp_term - 1.0)",
        "test": "helqix_circuit/tests/test_nonlinear.py::test_diode_matches_shockley_within_one_percent",
    },
    {
        "phase": "E2",
        "name": "MOSFET saturation coefficient halved",
        "file": "helqix_circuit/devices.py",
        "old": "id_n = 0.5 * k * overdrive * overdrive * (1.0 + lam * vds)",
        "new": "id_n = 0.25 * k * overdrive * overdrive * (1.0 + lam * vds)",
        "test": "helqix_circuit/tests/test_nonlinear.py::test_nmos_saturation_current_matches_level_one",
    },
    {
        "phase": "E2",
        "name": "AC capacitor admittance doubled",
        "file": "helqix_circuit/core.py",
        "old": "stamp_conductance(a, b, 1j * 2.0 * np.pi * frequency_hz * capacitance)",
        "new": "stamp_conductance(a, b, 1j * 2.0 * np.pi * frequency_hz * capacitance * 2.0)",
        "test": "helqix_circuit/tests/test_ac.py::test_rc_lowpass_cutoff_within_one_percent",
    },
    {
        "phase": "E2",
        "name": "finite op-amp gain clamped to 1e3",
        "file": "helqix_circuit/core.py",
        "old": "gain = component.params[\"gain\"]",
        "new": "gain = 1000.0",
        "test": "helqix_circuit/tests/test_ac.py::test_finite_gain_inverting_amplifier_within_point_one_percent",
    },
    {
        "phase": "E2",
        "name": "inductor branch column removed",
        "file": "helqix_circuit/core.py",
        "old": "                builder.add(k, b, -1.0)\n                if mode == \"dc\":\n                    pass",
        "new": "                builder.add(k, b, -1.0)\n                if mode == \"dc\":\n                    builder.add(a, k, -1.0)",
        "test": "helqix_circuit/tests/test_nonlinear.py::test_capacitor_open_and_inductor_short_in_dc",
    },
    {
        "phase": "E2-review",
        "name": "initial point ignores waveform value at t=0",
        "file": "helqix_circuit/core.py",
        "old": "elif mode in {\"tran\", \"ic\"}:",
        "new": "elif mode == \"tran\":",
        "test": "helqix_circuit/tests/test_robustness.py::test_adaptive_transient_handles_step_source_at_zero",
    },
    {
        "phase": "E2-review",
        "name": "waveform breakpoints disabled",
        "file": "helqix_circuit/transient.py",
        "old": "breakpoints = _breakpoints(circuit, stop)",
        "new": "breakpoints = []",
        "test": "helqix_circuit/tests/test_robustness.py::test_adaptive_transient_lands_on_delayed_step_breakpoint",
    },
    {
        "phase": "E2-review",
        "name": "non-finite AC frequency guard removed",
        "file": "helqix_circuit/ac.py",
        "old": "np.any(grid <= 0) or not np.all(np.isfinite(grid))",
        "new": "np.any(grid <= 0)",
        "test": "helqix_circuit/tests/test_robustness.py::test_ac_sweep_rejects_non_finite_frequency",
    },
    {
        "phase": "E2-review",
        "name": "diode absolute-zero guard removed",
        "file": "helqix_circuit/netlist.py",
        "old": "        if result[\"t_deg_c\"] <= -273.15:\n            raise InputError(f\"{where} diode temperature must be above absolute zero (-273.15 degC)\")\n",
        "new": "",
        "test": "helqix_circuit/tests/test_robustness.py::test_diode_temperature_must_be_above_absolute_zero",
    },
    {
        "phase": "E3",
        "name": "AWG exponent denominator changed",
        "file": "helqix_circuit/harness.py",
        "old": "return 0.127 * 92.0 ** ((36.0 - gauge) / 39.0)",
        "new": "return 0.127 * 92.0 ** ((36.0 - gauge) / 38.0)",
        "test": "helqix_circuit/tests/test_harness.py::test_awg_diameter_matches_the_definition",
    },
    {
        "phase": "E3",
        "name": "temperature correction dropped from resistance",
        "file": "helqix_circuit/harness.py",
        "old": "rho = rho20 * (1.0 + alpha * (conductor_temp_c - REFERENCE_TEMP_C))",
        "new": "rho = rho20",
        "test": "helqix_circuit/tests/test_harness.py::test_temperature_correction_uses_alpha",
    },
    {
        "phase": "E3",
        "name": "aluminium resistivity replaced by copper",
        "file": "helqix_circuit/harness.py",
        "old": '"aluminium": (2.826e-8, 0.00403),',
        "new": '"aluminium": (1.724e-8, 0.00403),',
        "test": "helqix_circuit/tests/test_harness.py::test_aluminium_resistivity_is_used_for_aluminium",
    },
    {
        "phase": "E3",
        "name": "drop percentage lost its factor of 100",
        "file": "helqix_circuit/harness.py",
        "old": "percent = 100.0 * drop / supply",
        "new": "percent = drop / supply",
        "test": "helqix_circuit/tests/test_harness.py::test_wire_in_series_with_a_load_drop_matches_hand_formula",
    },
    {
        "phase": "E3",
        "name": "temperature rise perimeter replaced by diameter",
        "file": "helqix_circuit/harness.py",
        "old": "perimeter = math.pi * spec.diameter_m",
        "new": "perimeter = spec.diameter_m",
        "test": "helqix_circuit/tests/test_harness.py::test_temperature_rise_uses_the_documented_convection_assumption",
    },
    {
        "phase": "E3",
        "name": "Table B.52.2 row 2.5 mm2 method C drifted",
        "file": "helqix_circuit/ampacity_table.py",
        "old": '"C": 27.0, "E": 30.0',
        "new": '"C": 26.0, "E": 30.0',
        "test": "helqix_circuit/tests/test_harness.py::test_ampacity_verdict_on_both_sides_of_each_limit",
    },
    {
        "phase": "E3",
        "name": "size match widened so 0.75 mm2 matches a row",
        "file": "helqix_circuit/ampacity_table.py",
        "old": "SIZE_MATCH_RELATIVE = 0.01",
        "new": "SIZE_MATCH_RELATIVE = 2.0",
        "test": "helqix_circuit/tests/test_harness.py::test_untabulated_size_gets_unknown_and_no_verdict",
    },
    {
        "phase": "E3",
        "name": "overall verdict ignores FAIL",
        "file": "helqix_circuit/harness.py",
        "old": '    for level in ("FAIL", "WARN", "UNKNOWN"):',
        "new": '    for level in ("WARN", "UNKNOWN"):',
        "test": "helqix_circuit/tests/test_harness.py::test_overload_is_never_reported_as_pass",
    },
    {
        "phase": "E4",
        "name": "white noise scaled by fs instead of fs/2",
        "file": "helqix_sensors/sensor.py",
        "old": "white_sigma = model.noise_density * math.sqrt(fs / 2.0)",
        "new": "white_sigma = model.noise_density * math.sqrt(fs)",
        "test": "helqix_sensors/tests/test_sensors.py::test_noise_sigma_within_three_percent_over_100k_samples",
    },
    {
        "phase": "E4",
        "name": "matched-Z pole drifts the -3 dB point",
        "file": "helqix_sensors/sensor.py",
        "old": "a = (2.0 - c) - math.sqrt((2.0 - c) ** 2 - 1.0)",
        "new": "a = math.exp(-2.0 * math.pi * bandwidth_hz / sample_rate_hz)",
        "test": "helqix_sensors/tests/test_sensors.py::test_minus_three_db_point_within_two_percent[1000.0]",
    },
    {
        "phase": "E4",
        "name": "resolution step uses 2**bits - 1 levels",
        "file": "helqix_sensors/sensor.py",
        "old": "step = (high - low) / 2**bits",
        "new": "step = (high - low) / (2**bits - 1)",
        "test": "helqix_sensors/tests/test_sensors.py::test_quantization_step_is_exact",
    },
    {
        "phase": "E4",
        "name": "saturation rail scaled by 0.999",
        "file": "helqix_sensors/sensor.py",
        "old": "reported = np.clip(quantized, model.min_value, model.max_value)",
        "new": "reported = np.clip(quantized, model.min_value, model.max_value) * 0.999",
        "test": "helqix_sensors/tests/test_sensors.py::test_saturation_is_exact_and_flagged",
    },
    {
        "phase": "E4",
        "name": "latency delay skipped",
        "file": "helqix_sensors/sensor.py",
        "old": "analog = _delay(filtered, shift)",
        "new": "analog = filtered",
        "test": "helqix_sensors/tests/test_sensors.py::test_latency_shifts_the_reading_by_whole_samples",
    },
    {
        "phase": "E4",
        "name": "drift sign flipped",
        "file": "helqix_sensors/sensor.py",
        "old": "pre = truth + model.bias + model.drift_per_hour * (clock / 3600.0)",
        "new": "pre = truth + model.bias - model.drift_per_hour * (clock / 3600.0)",
        "test": "helqix_sensors/tests/test_sensors.py::test_bias_and_drift_are_applied",
    },
    {
        "phase": "E4",
        "name": "equivalent noise bandwidth inverted",
        "file": "helqix_sensors/sensor.py",
        "old": "enbw = (fs / 2.0) * (1.0 - a) / (1.0 + a)",
        "new": "enbw = (fs / 2.0) * (1.0 + a) / (1.0 - a)",
        "test": "helqix_sensors/tests/test_sensors.py::test_noise_sigma_within_three_percent_over_100k_samples",
    },
    {
        "phase": "E4",
        "name": "seed ignored by the generator",
        "file": "helqix_sensors/sensor.py",
        "old": "rng = np.random.default_rng(seed)",
        "new": "rng = np.random.default_rng(1)",
        "test": "helqix_sensors/tests/test_sensors.py::test_same_seed_gives_identical_bytes",
    },
    {
        "phase": "E4",
        "name": "results document read from values instead of probes",
        "file": "helqix_sensors/sensor.py",
        "old": 'if isinstance(signal, dict) and "probes" in signal:',
        "new": 'if isinstance(signal, dict) and "values" in signal:',
        "test": "helqix_sensors/tests/test_sensors.py::test_probes_from_results_json_are_read_and_resampled",
    },
    {
        "phase": "E5",
        "name": "bad input exits 3 instead of 2",
        "file": "helqix_circuit/cli.py",
        "old": '        print(f"error: {exc}", file=sys.stderr)\n        return 2',
        "new": '        print(f"error: {exc}", file=sys.stderr)\n        return 3',
        "test": "helqix_circuit/tests/test_cli_circuit.py::test_bad_input_exits_2_with_a_plain_message",
    },
    {
        "phase": "E5",
        "name": "probe cap raised to 20000",
        "file": "helqix_circuit/cli.py",
        "old": "MAX_PROBE_POINTS = 2000",
        "new": "MAX_PROBE_POINTS = 20000",
        "test": "helqix_circuit/tests/test_cli_circuit.py::test_probes_are_downsampled_to_2000_points",
    },
    {
        "phase": "E5",
        "name": "reference label never VALIDATED",
        "file": "helqix_circuit/cli.py",
        "old": 'label = "VALIDATED" if reference_report["matched"] else "UNVALIDATED"',
        "new": 'label = "CHECKED"',
        "test": "helqix_circuit/tests/test_cli_circuit.py::test_divider_case_writes_the_contract_folder",
    },
    {
        "phase": "E5",
        "name": "fresh-folder rule disabled",
        "file": "helqix_circuit/cli.py",
        "old": "        if foreign:",
        "new": "        if False:",
        "test": "helqix_circuit/tests/test_cli_circuit.py::test_out_folder_must_be_fresh",
    },
    {
        "phase": "E5",
        "name": "max_nodes limit ignored",
        "file": "helqix_circuit/cli.py",
        "old": 'if node_count > limits["max_nodes"]:',
        "new": 'if node_count > limits["max_nodes"] * 100:',
        "test": "helqix_circuit/tests/test_cli_circuit.py::test_limits_max_nodes_is_enforced",
    },
    {
        "phase": "E5",
        "name": "transient deadline check disabled",
        "file": "helqix_circuit/transient.py",
        "old": "        if deadline is not None and time.perf_counter() > deadline:",
        "new": "        if False:",
        "test": "helqix_circuit/tests/test_cli_circuit.py::test_timeout_is_enforced",
    },
    {
        "phase": "E5",
        "name": "wire verdicts dropped from the result",
        "file": "helqix_circuit/cli.py",
        "old": 'result["wires"] = list(report["wires"].values())',
        "new": 'result["wires"] = []',
        "test": "helqix_circuit/tests/test_cli_circuit.py::test_wire_verdicts_are_in_the_result",
    },
    {
        "phase": "E5",
        "name": "downsample keeps only the first 2000 points",
        "file": "helqix_circuit/cli.py",
        "old": "return np.unique(np.rint(np.linspace(0.0, size - 1.0, MAX_PROBE_POINTS)).astype(int))",
        "new": "return np.arange(MAX_PROBE_POINTS)",
        "test": "helqix_circuit/tests/test_cli_circuit.py::test_probes_are_downsampled_to_2000_points",
    },
    {
        "phase": "E5",
        "name": "sensor limits accept any key",
        "file": "helqix_sensors/cli.py",
        "old": "    unknown = sorted(set(raw) - set(DEFAULT_LIMITS))",
        "new": "    unknown = []",
        "test": "helqix_sensors/tests/test_cli_sensors.py::test_sensor_limits_accept_only_timeout",
    },
    {
        "phase": "E5",
        "name": "sensor reference label never VALIDATED",
        "file": "helqix_sensors/cli.py",
        "old": 'label = "VALIDATED" if reference_report["matched"] else "UNVALIDATED"',
        "new": 'label = "CHECKED"',
        "test": "helqix_sensors/tests/test_cli_sensors.py::test_reference_can_validate_a_case",
    },
    {
        "phase": "E5",
        "name": "sensor clipped series dropped",
        "file": "helqix_sensors/cli.py",
        "old": 'SERIES_IDS = ("truth", "analog", "reported", "clipped")',
        "new": 'SERIES_IDS = ("truth", "analog", "reported")',
        "test": "helqix_sensors/tests/test_cli_sensors.py::test_sensor_case_writes_the_contract_folder",
    },
    {
        "phase": "E5",
        "name": "circuit unknown-field check disabled",
        "file": "helqix_circuit/netlist.py",
        "old": "    unknown = sorted(set(body) - allowed)\n    if unknown:",
        "new": "    unknown = sorted(set(body) - allowed)\n    if False:",
        "test": "helqix_circuit/tests/test_cli_circuit.py::test_unknown_fields_inside_the_circuit_are_refused",
    },
    {
        "phase": "E5",
        "name": "sensor unknown-field check disabled",
        "file": "helqix_sensors/sensor.py",
        "old": "    unknown = sorted(set(body) - allowed)\n    if unknown:",
        "new": "    unknown = sorted(set(body) - allowed)\n    if False:",
        "test": "helqix_sensors/tests/test_cli_sensors.py::test_unknown_model_field_is_refused",
    },
    {
        "phase": "E5",
        "name": "sensor post-run timeout check disabled",
        "file": "helqix_sensors/cli.py",
        "old": "    if time.perf_counter() > deadline:",
        "new": "    if False:",
        "test": "helqix_sensors/tests/test_cli_sensors.py::test_timeout_is_enforced",
    },
    {
        "phase": "E7",
        "name": "frame checksum check removed",
        "file": "helqix_io/faults.py",
        "old": "    if zlib.crc32(body) != checksum:\n        raise ValueError(\"frame checksum does not match\")",
        "new": "    if False:\n        raise ValueError(\"frame checksum does not match\")",
        "test": "helqix_io/tests/test_serial.py::test_framing_preserves_empty_and_binary_payloads",
    },
    {
        "phase": "E7",
        "name": "serial back-pressure boundary widened",
        "file": "helqix_io/serial_port.py",
        "old": "while len(self._peer._incoming) >= self._peer.capacity_frames:",
        "new": "while len(self._peer._incoming) > self._peer.capacity_frames:",
        "test": "helqix_io/tests/test_serial.py::test_back_pressure_raises_after_the_deadline",
    },
    {
        "phase": "E7",
        "name": "serial drop injection disabled",
        "file": "helqix_io/serial_port.py",
        "old": "                if self.faults.drop and self._rng.random() < self.faults.drop:",
        "new": "                if False:",
        "test": "helqix_io/tests/test_serial.py::test_drop_injection_is_seeded_and_counted",
    },
    {
        "phase": "E7",
        "name": "corrupted frame delivered instead of counted",
        "file": "helqix_io/serial_port.py",
        "old": "                if self.faults.corrupt and self._rng.random() < self.faults.corrupt:\n                    raw = raw[:-1] + bytes([raw[-1] ^ 0xFF])\n                    self.stats.corrupted += 1\n                    continue",
        "new": "                if self.faults.corrupt and self._rng.random() < self.faults.corrupt:\n                    self.stats.corrupted += 1",
        "test": "helqix_io/tests/test_serial.py::test_corrupt_injection_never_delivers_bad_bytes",
    },
    {
        "phase": "E7",
        "name": "serial delay ignored",
        "file": "helqix_io/serial_port.py",
        "old": "        self._peer._incoming.append((raw, time.monotonic() + self._peer.faults.delay_s))",
        "new": "        self._peer._incoming.append((raw, time.monotonic()))",
        "test": "helqix_io/tests/test_serial.py::test_delay_holds_the_frame_until_its_time",
    },
    {
        "phase": "E7",
        "name": "single-level wildcard broken",
        "file": "helqix_io/bus.py",
        "old": '        if level != "+" and level != topic_levels[index]:',
        "new": '        if level != "?" and level != topic_levels[index]:',
        "test": "helqix_io/tests/test_bus.py::test_topic_wildcards",
    },
    {
        "phase": "E7",
        "name": "bus delivery order reversed",
        "file": "helqix_io/bus.py",
        "old": "            client._incoming.append(\n                Message(message_id, topic, data, checksum, time.monotonic() + client.faults.delay_s)\n            )",
        "new": "            client._incoming.appendleft(\n                Message(message_id, topic, data, checksum, time.monotonic() + client.faults.delay_s)\n            )",
        "test": "helqix_io/tests/test_bus.py::test_ten_thousand_messages_round_trip_exactly_and_in_order",
    },
    {
        "phase": "E7",
        "name": "QoS 1 pending message never recorded",
        "file": "helqix_io/bus.py",
        "old": "        if qos == 1:\n            self._pending[message_id] = _Pending(topic, bytes(payload))",
        "new": "        if qos == 2:\n            self._pending[message_id] = _Pending(topic, bytes(payload))",
        "test": "helqix_io/tests/test_bus.py::test_qos1_retry_after_disconnect_then_ack_stops_it",
    },
    {
        "phase": "E7",
        "name": "ack not recorded",
        "file": "helqix_io/bus.py",
        "old": "        self.broker.record_ack(message_id)",
        "new": "        pass",
        "test": "helqix_io/tests/test_bus.py::test_qos1_retry_after_disconnect_then_ack_stops_it",
    },
    {
        "phase": "E7",
        "name": "bus disconnect ignored",
        "file": "helqix_io/bus.py",
        "old": "    def disconnect(self) -> None:\n        self._closed = True\n        self._incoming.clear()",
        "new": "    def disconnect(self) -> None:\n        self._closed = False\n        self._incoming.clear()",
        "test": "helqix_io/tests/test_bus.py::test_disconnect_drops_delivery_and_reconnect_restores_it",
    },
    {
        "phase": "E7",
        "name": "bus full-queue refusal disabled",
        "file": "helqix_io/bus.py",
        "old": "        if full:",
        "new": "        if False:",
        "test": "helqix_io/tests/test_bus.py::test_back_pressure_refuses_the_whole_publish",
    },
]


def _copy_tree(destination: Path) -> None:
    shutil.copytree(ROOT, destination, ignore=IGNORE)


def _run_test(root: Path, node: str) -> tuple[int, str]:
    env = dict(os.environ)
    env["PYTHONPATH"] = str(root) + os.pathsep + env.get("PYTHONPATH", "")
    proc = subprocess.run(
        [sys.executable, "-m", "pytest", node, "-q", "--tb=no", "-p", "no:cacheprovider"],
        cwd=str(root),
        capture_output=True,
        text=True,
        env=env,
        timeout=120,
    )
    return proc.returncode, (proc.stdout + proc.stderr).strip().splitlines()[-1] if (proc.stdout or proc.stderr) else ""


def run() -> tuple[bool, str]:
    caught = 0
    survivors = []
    for mutation in MUTATIONS:
        with tempfile.TemporaryDirectory(prefix="helqix-mut-") as tmp:
            root = Path(tmp) / "24-electronics"
            _copy_tree(root)
            target = root / mutation["file"]
            text = target.read_text(encoding="utf-8")
            if mutation["old"] not in text:
                survivors.append(f"{mutation['phase']} {mutation['name']}: pattern not found")
                continue
            target.write_text(text.replace(mutation["old"], mutation["new"], 1), encoding="utf-8")
            code, summary = _run_test(root, mutation["test"])
            if code == 0:
                survivors.append(f"{mutation['phase']} {mutation['name']}: survived ({summary})")
            else:
                caught += 1
    total = len(MUTATIONS)
    if survivors:
        return False, f"{caught}/{total} caught; survivors: " + "; ".join(survivors)
    return True, f"{caught}/{total} caught"


if __name__ == "__main__":
    ok, text = run()
    print(text)
    raise SystemExit(0 if ok else 1)

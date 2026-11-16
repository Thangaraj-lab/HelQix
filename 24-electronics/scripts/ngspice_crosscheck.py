#!/usr/bin/env python
"""Cross-check the in-repo circuit solver against ngspice, run as a process.

ngspice is never imported or linked.  For each case the exporter in
helqix_circuit.spice writes a deck to a temporary directory, ngspice is run in
batch mode on that deck, and the printed values are compared with the Python
result.  A missing ngspice executable is a clean skip (exit 0) so the local
fast suite keeps working offline; CI installs ngspice and runs the check.

Exit code is 0 when every comparison is within its tolerance, 1 otherwise.
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from helqix_circuit import load_circuit  # noqa: E402
from helqix_circuit.ac import AcAnalyzer  # noqa: E402
from helqix_circuit.mna import solve_dc  # noqa: E402
from helqix_circuit.spice import build_deck, parse_ngspice_output  # noqa: E402
from helqix_circuit.transient import transient  # noqa: E402

TITLE = "helqix circuit cross-check against ngspice"


def comp(cid: str, kind: str, nodes: list[str], value=None, unit=None, **extra) -> dict:
    body = {"id": cid, "kind": kind, "nodes": nodes}
    if value is not None:
        body["value"] = {"value": value, "unit": unit}
    body.update(extra)
    return body


def _diode(cid: str, nodes: list[str]) -> dict:
    return comp(cid, "diode", nodes, parameters={"is_a": 1e-12, "n": 1.0, "rs_ohm": 0.0, "t_deg_c": 27.0})


def circuit(body: dict, title: str, analysis: dict) -> dict:
    return {"version": "circuit/0.1", "title": title, "components": body, "analysis": analysis}


DC_CASES = [
    ("dc divider", 1e-5, 1e-9, circuit([
        comp("V1", "vsource", ["in", "0"], 10.0, "v"),
        comp("R1", "resistor", ["in", "out"], 1000.0, "ohm"),
        comp("R2", "resistor", ["out", "0"], 2000.0, "ohm"),
    ], "dc divider", {"type": "dc"})),
    ("dc ladder", 1e-5, 1e-9, circuit([
        comp("V1", "vsource", ["in", "0"], 5.0, "v"),
        comp("R1", "resistor", ["in", "a"], 100.0, "ohm"),
        comp("R2", "resistor", ["a", "b"], 220.0, "ohm"),
        comp("R3", "resistor", ["b", "c"], 330.0, "ohm"),
        comp("R4", "resistor", ["c", "0"], 470.0, "ohm"),
    ], "dc ladder", {"type": "dc"})),
    ("dc bridge", 1e-5, 1e-9, circuit([
        comp("V1", "vsource", ["in", "0"], 12.0, "v"),
        comp("R1", "resistor", ["in", "a"], 1000.0, "ohm"),
        comp("R2", "resistor", ["a", "0"], 2000.0, "ohm"),
        comp("R3", "resistor", ["in", "b"], 3000.0, "ohm"),
        comp("R4", "resistor", ["b", "0"], 4000.0, "ohm"),
    ], "dc bridge", {"type": "dc"})),
    ("dc current source", 1e-5, 1e-9, circuit([
        comp("I1", "isource", ["0", "out"], 2.0, "ma"),
        comp("R1", "resistor", ["out", "0"], 1000.0, "ohm"),
        comp("R2", "resistor", ["out", "0"], 3000.0, "ohm"),
    ], "dc current source", {"type": "dc"})),
    ("dc two sources", 1e-5, 1e-9, circuit([
        comp("V1", "vsource", ["in", "0"], 9.0, "v"),
        comp("R1", "resistor", ["in", "mid"], 1000.0, "ohm"),
        comp("V2", "vsource", ["mid2", "0"], 3.0, "v"),
        comp("R2", "resistor", ["mid", "mid2"], 2000.0, "ohm"),
        comp("R3", "resistor", ["mid", "0"], 3000.0, "ohm"),
    ], "dc two sources", {"type": "dc"})),
    ("dc vcvs", 1e-5, 1e-9, circuit([
        comp("V1", "vsource", ["in", "0"], 1.0, "v"),
        comp("E1", "vcvs", ["out", "0", "in", "0"], 4.0, "1"),
        comp("R1", "resistor", ["out", "0"], 10000.0, "ohm"),
    ], "dc vcvs", {"type": "dc"})),
    ("dc vccs", 1e-5, 1e-9, circuit([
        comp("V1", "vsource", ["in", "0"], 2.0, "v"),
        comp("G1", "vccs", ["out", "0", "in", "0"], 1.0, "ms"),
        comp("R1", "resistor", ["out", "0"], 1000.0, "ohm"),
    ], "dc vccs", {"type": "dc"})),
    ("dc switch closed", 1e-4, 1e-9, circuit([
        comp("V1", "vsource", ["in", "0"], 5.0, "v"),
        comp("SW1", "switch", ["in", "out"], state="closed"),
        comp("R1", "resistor", ["out", "0"], 100.0, "ohm"),
    ], "dc switch closed", {"type": "dc"})),
    ("dc switch open", 1e-4, 1e-6, circuit([
        comp("V1", "vsource", ["in", "0"], 5.0, "v"),
        comp("SW1", "switch", ["in", "out"], state="open"),
        comp("R1", "resistor", ["out", "0"], 1000.0, "ohm"),
    ], "dc switch open", {"type": "dc"})),
    ("dc diode forward", 1e-3, 1e-9, circuit([
        comp("V1", "vsource", ["in", "0"], 0.7, "v"),
        comp("R1", "resistor", ["in", "out"], 1000.0, "ohm"),
        _diode("D1", ["out", "0"]),
    ], "dc diode forward", {"type": "dc"})),
    ("dc diode reverse", 1e-3, 1e-9, circuit([
        comp("V1", "vsource", ["in", "0"], -1.0, "v"),
        comp("R1", "resistor", ["in", "out"], 1000.0, "ohm"),
        _diode("D1", ["out", "0"]),
    ], "dc diode reverse", {"type": "dc"})),
]

AC_CASES = [
    ("ac rc low-pass", [100.0, 1591.5494309189535, 10000.0], 1e-5, 1e-9, circuit([
        comp("V1", "vsource", ["in", "0"], 1.0, "v",
             waveform={"type": "sine", "amplitude": 1.0, "frequency_hz": 1000.0, "offset": 0.0, "phase_deg": 0.0}),
        comp("R1", "resistor", ["in", "out"], 1000.0, "ohm"),
        comp("C1", "capacitor", ["out", "0"], 100.0, "nf"),
    ], "ac rc low-pass", {"type": "ac", "start_hz": 100.0, "stop_hz": 10000.0})),
    ("ac rl high-pass", [100.0, 1591.5494309189535, 10000.0], 1e-5, 1e-9, circuit([
        comp("V1", "vsource", ["in", "0"], 1.0, "v",
             waveform={"type": "sine", "amplitude": 1.0, "frequency_hz": 1000.0, "offset": 0.0, "phase_deg": 0.0}),
        comp("R1", "resistor", ["in", "out"], 100.0, "ohm"),
        comp("L1", "inductor", ["out", "0"], 10.0, "mh"),
    ], "ac rl high-pass", {"type": "ac", "start_hz": 100.0, "stop_hz": 10000.0})),
    ("ac rlc series", [1000.0, 15915.494309189533, 25000.0], 1e-5, 1e-9, circuit([
        comp("V1", "vsource", ["in", "0"], 1.0, "v",
             waveform={"type": "sine", "amplitude": 1.0, "frequency_hz": 15915.494309189533, "offset": 0.0, "phase_deg": 0.0}),
        comp("R1", "resistor", ["in", "a"], 10.0, "ohm"),
        comp("L1", "inductor", ["a", "out"], 1.0, "mh"),
        comp("C1", "capacitor", ["out", "0"], 100.0, "nf"),
    ], "ac rlc series", {"type": "ac", "start_hz": 1000.0, "stop_hz": 25000.0})),
    ("ac rc ladder", [500.0, 5000.0], 1e-5, 1e-9, circuit([
        comp("V1", "vsource", ["in", "0"], 1.0, "v",
             waveform={"type": "sine", "amplitude": 1.0, "frequency_hz": 1000.0, "offset": 0.0, "phase_deg": 30.0}),
        comp("R1", "resistor", ["in", "a"], 1000.0, "ohm"),
        comp("C1", "capacitor", ["a", "0"], 100.0, "nf"),
        comp("R2", "resistor", ["a", "out"], 1000.0, "ohm"),
        comp("C2", "capacitor", ["out", "0"], 10.0, "nf"),
    ], "ac rc ladder", {"type": "ac", "start_hz": 500.0, "stop_hz": 5000.0})),
]

TRAN_CASES = [
    ("tran rc step", {"stop_s": 5e-3, "step_s": 1e-6},
     [("m1", "out", 0.5e-3), ("m2", "out", 1.0e-3), ("m3", "out", 5.0e-3)], 1e-3, 1e-6,
     circuit([
         comp("V1", "vsource", ["in", "0"], 5.0, "v", waveform={"type": "step", "delay_s": 0.0}),
         comp("R1", "resistor", ["in", "out"], 1000.0, "ohm"),
         comp("C1", "capacitor", ["out", "0"], 1.0, "uf"),
     ], "tran rc step", {"type": "tran", "stop_s": 5e-3, "step_s": 1e-6})),
    ("tran rl step", {"stop_s": 5e-3, "step_s": 1e-6},
     [("m1", "out", 0.2e-3), ("m2", "out", 0.5e-3), ("m3", "out", 2.0e-3)], 1e-3, 1e-6,
     circuit([
         comp("V1", "vsource", ["in", "0"], 3.0, "v", waveform={"type": "step", "delay_s": 0.0}),
         comp("R1", "resistor", ["in", "out"], 10.0, "ohm"),
         comp("L1", "inductor", ["out", "0"], 1.0, "mh"),
     ], "tran rl step", {"type": "tran", "stop_s": 5e-3, "step_s": 1e-6})),
    ("tran rlc step", {"stop_s": 5e-3, "step_s": 1e-6},
     [("m1", "out", 0.5e-3), ("m2", "out", 2.0e-3), ("m3", "out", 5.0e-3)], 5e-3, 1e-6,
     circuit([
         comp("V1", "vsource", ["in", "0"], 1.0, "v", waveform={"type": "step", "delay_s": 0.0}),
         comp("R1", "resistor", ["in", "a"], 5.0, "ohm"),
         comp("L1", "inductor", ["a", "out"], 1.0, "mh"),
         comp("C1", "capacitor", ["out", "0"], 1.0, "uf"),
     ], "tran rlc step", {"type": "tran", "stop_s": 5e-3, "step_s": 1e-6})),
]


def find_ngspice(explicit: str | None) -> str | None:
    for candidate in (explicit, shutil.which("ngspice"), shutil.which("ngspice_con")):
        if candidate and Path(candidate).is_file():
            return str(candidate)
    return None


def _run(exe: str, deck_path: Path, log_path: Path) -> str:
    proc = subprocess.run(
        [exe, "-b", "-o", str(log_path), str(deck_path)],
        capture_output=True, text=True, timeout=120, cwd=str(deck_path.parent),
    )
    text = ""
    if log_path.exists():
        text = log_path.read_text(encoding="utf-8", errors="replace")
    return text + "\n" + proc.stdout + "\n" + proc.stderr


def _write_deck(directory: Path, stem: str, deck: str) -> tuple[Path, Path]:
    deck_path = directory / f"{stem}.cir"
    deck_path.write_text(deck, encoding="utf-8")
    return deck_path, directory / f"{stem}.log"


def _compare(pairs: list[tuple[str, float, float]], rtol: float, atol: float) -> tuple[float, list[str]]:
    scale = max((max(abs(py), abs(sp)) for _, py, sp in pairs), default=0.0)
    denominator = max(scale, 1e-12)
    worst = 0.0
    failures: list[str] = []
    for name, py, sp in pairs:
        error = abs(py - sp)
        worst = max(worst, error / denominator)
        if error > atol + rtol * max(abs(py), abs(sp)):
            failures.append(f"{name}: python {py:.12g} ngspice {sp:.12g} (|d| {error:.3g})")
    return worst, failures


def check(exe: str, workdir: Path) -> tuple[list[tuple[str, int, float, bool, str]], list[str]]:
    rows: list[tuple[str, int, float, bool, str]] = []
    failures: list[str] = []
    for name, rtol, atol, body in DC_CASES:
        circuit_obj = load_circuit(body)
        deck_path, log_path = _write_deck(workdir, name.replace(" ", "_"), build_deck(circuit_obj, "dc"))
        printed = parse_ngspice_output(_run(exe, deck_path, log_path))
        solution = solve_dc(circuit_obj)
        pairs = [(f"v({node})", solution.node_voltages.get(node, 0.0), printed[f"v({index})"])
                 for index, node in enumerate(circuit_obj.node_names, start=1) if f"v({index})" in printed]
        worst, failed = _compare(pairs, rtol, atol)
        rows.append((name, len(pairs), worst, not failed, "; ".join(failed)))
        failures.extend(f"{name}: {item}" for item in failed)

    for name, frequencies, rtol, atol, body in AC_CASES:
        circuit_obj = load_circuit(body)
        analyzer = AcAnalyzer(circuit_obj)
        pairs: list[tuple[str, float, float]] = []
        for frequency in frequencies:
            stem = f"{name.replace(' ', '_')}_f{int(round(frequency))}"
            deck_path, log_path = _write_deck(workdir, stem, build_deck(circuit_obj, "ac", frequency_hz=frequency))
            printed = parse_ngspice_output(_run(exe, deck_path, log_path))
            solution = analyzer.solve(frequency)
            for index, node in enumerate(circuit_obj.node_names, start=1):
                value = solution.get(node, 0.0)
                pairs.append((f"re(v({node}))@{frequency:g}Hz", value.real, printed[f"re(v({index}))"]))
                pairs.append((f"im(v({node}))@{frequency:g}Hz", value.imag, printed[f"im(v({index}))"]))
        worst, failed = _compare(pairs, rtol, atol)
        rows.append((name, len(pairs), worst, not failed, "; ".join(failed)))
        failures.extend(f"{name}: {item}" for item in failed)

    for name, timing, measures, rtol, atol, body in TRAN_CASES:
        circuit_obj = load_circuit(body)
        deck = build_deck(
            circuit_obj, "tran",
            tran_step_s=timing["step_s"], tran_stop_s=timing["stop_s"], measures=measures,
        )
        deck_path, log_path = _write_deck(workdir, name.replace(" ", "_"), deck)
        printed = parse_ngspice_output(_run(exe, deck_path, log_path))
        solution = transient(circuit_obj, stop_s=timing["stop_s"], step_s=timing["step_s"])
        pairs = []
        for measure_name, node, time_s in measures:
            pairs.append((f"v({node})@{time_s:g}s", solution.value_at(node, time_s), printed[measure_name]))
        worst, failed = _compare(pairs, rtol, atol)
        rows.append((name, len(pairs), worst, not failed, "; ".join(failed)))
        failures.extend(f"{name}: {item}" for item in failed)

    return rows, failures


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--ngspice", help="path to the ngspice executable (default: PATH)")
    parser.add_argument("--keep", action="store_true", help="keep the generated decks")
    args = parser.parse_args(argv)

    exe = find_ngspice(args.ngspice)
    if exe is None:
        print("ngspice cross-check: skipped (ngspice not found; install it or pass --ngspice)")
        return 0
    try:
        version = subprocess.run([exe, "--version"], capture_output=True, text=True, timeout=30)
        lines = [line.strip() for line in (version.stdout or version.stderr).splitlines() if line.strip()]
        banner = next((line for line in lines if "ngspice" in line.lower()), lines[0] if lines else exe)
        print(f"ngspice cross-check: {banner}")
    except (OSError, subprocess.TimeoutExpired) as exc:
        print(f"ngspice cross-check: skipped (cannot run {exe}: {exc})")
        return 0

    with tempfile.TemporaryDirectory(prefix="helqix-spice-") as temp:
        workdir = Path(temp)
        try:
            rows, failures = check(exe, workdir)
        except KeyError as exc:
            print(f"ngspice cross-check: FAIL (ngspice did not print {exc})")
            return 1
        if args.keep:
            final = ROOT / "build" / "ngspice"
            shutil.copytree(workdir, final, dirs_exist_ok=True)
            print(f"decks kept in {final}")

    header = f"{'CIRCUIT':<22}{'POINTS':>7}{'MAX ERR/SCALE':>16}  STATUS"
    print(header)
    print("-" * len(header))
    all_ok = True
    for name, points, worst, ok, detail in rows:
        all_ok = all_ok and ok
        print(f"{name:<22}{points:>7}{worst:>14.3e}  {'PASS' if ok else 'FAIL'}")
        if detail:
            print(f"    {detail}")
    print("-" * len(header))
    passed = sum(1 for row in rows if row[3])
    print(f"{passed}/{len(rows)} circuits within tolerance")
    print("OVERALL", "PASS" if all_ok else "FAIL")
    return 0 if all_ok else 1


if __name__ == "__main__":
    raise SystemExit(main())

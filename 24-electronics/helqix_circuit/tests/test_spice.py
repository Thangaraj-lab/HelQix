"""Export tests. No ngspice is needed; the deck text and the parser are checked."""

import importlib.util
from pathlib import Path

import pytest

from helqix_circuit import InputError, load_circuit
from helqix_circuit.spice import build_deck, parse_ngspice_output


def comp(cid, kind, nodes, value=None, unit=None, **extra):
    body = {"id": cid, "kind": kind, "nodes": nodes}
    if value is not None:
        body["value"] = {"value": value, "unit": unit}
    body.update(extra)
    return body


def circuit(components, analysis=None):
    return load_circuit({
        "version": "circuit/0.1",
        "title": "export test",
        "components": components,
        "analysis": analysis or {"type": "dc"},
    })


def test_every_exported_kind_appears_in_the_deck():
    model = circuit([
        comp("V1", "vsource", ["in", "0"], 5.0, "v"),
        comp("R1", "resistor", ["in", "a"], 1000.0, "ohm"),
        comp("I1", "isource", ["0", "a"], 1.0, "ma"),
        comp("E1", "vcvs", ["b", "0", "a", "0"], 2.0, "1"),
        comp("G1", "vccs", ["a", "b", "in", "0"], 1.0, "ms"),
        comp("SW1", "switch", ["a", "b"], state="closed"),
        comp("C1", "capacitor", ["b", "0"], 1.0, "uf"),
        comp("L1", "inductor", ["a", "0"], 1.0, "mh"),
        comp("D1", "diode", ["b", "0"], parameters={"is_a": 1e-12, "n": 1.0, "rs_ohm": 0.0, "t_deg_c": 27.0}),
    ])
    deck = build_deck(model, "dc")
    for token in (
        "V1 1 0 DC 5.0", "R2 1 2 1000.0", "I3 0 2 DC 0.001",
        "E4 3 0 2 0 2.0", "G5 2 3 1 0 0.001",
        "R6 2 3 0.001", "C7 3 0 1e-06", "L8 2 0 0.001",
        "D9 3 0 DM9", ".model DM9 D(IS=1e-12 N=1.0 RS=0.0)",
        ".options temp=27.0", ".options numdgt=15", "op", "print v(1) v(2) v(3)",
    ):
        assert token in deck, token


def test_switch_open_exports_a_large_resistance_not_a_short():
    model = circuit([
        comp("V1", "vsource", ["in", "0"], 5.0, "v"),
        comp("SW1", "switch", ["in", "out"], state="open"),
        comp("R1", "resistor", ["out", "0"], 1000.0, "ohm"),
    ])
    deck = build_deck(model, "dc")
    assert "R2 1 2 1e+12" in deck


def test_ac_deck_uses_the_source_phasor_and_prints_re_and_im():
    model = circuit([
        comp("V1", "vsource", ["in", "0"], 1.0, "v",
             waveform={"type": "sine", "amplitude": 2.0, "frequency_hz": 1000.0, "offset": 0.5, "phase_deg": 90.0}),
        comp("R1", "resistor", ["in", "out"], 1000.0, "ohm"),
        comp("C1", "capacitor", ["out", "0"], 100.0, "nf"),
    ], {"type": "ac", "start_hz": 100.0, "stop_hz": 1000.0})
    deck = build_deck(model, "ac", frequency_hz=1000.0)
    assert "DC 0.5 AC 2.0 90.0" in deck
    assert "ac lin 1 1000.0 1000.0" in deck
    assert "re(v(1)) im(v(1)) re(v(2)) im(v(2))" in deck


def test_transient_deck_carries_its_measures():
    model = circuit([
        comp("V1", "vsource", ["in", "0"], 5.0, "v", waveform={"type": "step", "delay_s": 0.0}),
        comp("R1", "resistor", ["in", "out"], 1000.0, "ohm"),
        comp("C1", "capacitor", ["out", "0"], 1.0, "uf"),
    ], {"type": "tran", "stop_s": 5e-3, "step_s": 1e-6})
    deck = build_deck(model, "tran", tran_step_s=1e-6, tran_stop_s=5e-3,
                      measures=[("m1", "out", 1e-3)])
    assert "PULSE(0 5.0 0.0 1n 1n 1 2)" in deck
    assert "tran 1e-06 0.005" in deck
    assert "meas tran m1 find v(2) at=0.001" in deck


def test_unknown_kind_and_nonzero_initial_are_refused():
    mosfet = circuit([
        comp("M1", "mosfet", ["d", "g", "0", "0"], parameters={"channel": "n", "vth": 1.0, "k": 1e-3}),
        comp("V1", "vsource", ["g", "0"], 5.0, "v"),
        comp("R1", "resistor", ["d", "0"], 1000.0, "ohm"),
    ])
    with pytest.raises(InputError, match="cannot export component 'M1'"):
        build_deck(mosfet, "dc")
    charged = circuit([
        comp("V1", "vsource", ["in", "0"], 5.0, "v"),
        comp("C1", "capacitor", ["in", "0"], 1.0, "uf", initial={"value": 2.0, "unit": "v"}),
    ])
    with pytest.raises(InputError, match="non-zero initial condition"):
        build_deck(charged, "tran", tran_step_s=1e-6, tran_stop_s=1e-3)


def test_diodes_at_different_temperatures_are_refused():
    model = circuit([
        comp("V1", "vsource", ["in", "0"], 1.0, "v"),
        comp("D1", "diode", ["in", "0"], parameters={"is_a": 1e-12, "n": 1.0, "rs_ohm": 0.0, "t_deg_c": 27.0}),
        comp("D2", "diode", ["in", "0"], parameters={"is_a": 1e-12, "n": 1.0, "rs_ohm": 0.0, "t_deg_c": 100.0}),
    ])
    with pytest.raises(InputError, match="different temperatures"):
        build_deck(model, "dc")


def test_parser_reads_print_and_measure_lines():
    text = """
    Circuit: export test
    v(1) = 5.000000000000000e+00
    v(2) = -1.250000000000000e-03
    re(v(2)) = 9.999999999999999e-01
    im(v(2)) = -1.000000000000000e-04
    m1 = 3.160602794142802e+00
    * a comment line 7 = 8 is ignored
    """
    values = parse_ngspice_output(text)
    assert values["v(1)"] == 5.0
    assert values["v(2)"] == pytest.approx(-1.25e-3)
    assert values["re(v(2))"] == pytest.approx(1.0)
    assert values["m1"] == pytest.approx(3.160602794142802)
    assert "a comment line 7 " not in values


def test_crosscheck_case_list_builds_every_deck():
    script = Path(__file__).resolve().parents[2] / "scripts" / "ngspice_crosscheck.py"
    spec = importlib.util.spec_from_file_location("ngspice_crosscheck", script)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    assert len(module.DC_CASES) + len(module.AC_CASES) + len(module.TRAN_CASES) >= 15
    for name, rtol, atol, body in module.DC_CASES:
        deck = build_deck(load_circuit(body), "dc")
        assert deck.startswith("* helqix cross-check")
    for name, frequencies, rtol, atol, body in module.AC_CASES:
        for frequency in frequencies:
            build_deck(load_circuit(body), "ac", frequency_hz=frequency)
    for name, timing, measures, rtol, atol, body in module.TRAN_CASES:
        deck = build_deck(
            load_circuit(body), "tran",
            tran_step_s=timing["step_s"], tran_stop_s=timing["stop_s"], measures=measures,
        )
        assert "meas tran" in deck

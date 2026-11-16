"""Export a circuit to a SPICE deck and parse ngspice's printed values.

The exporter covers the element kinds the cross-check uses: resistor, vsource,
isource, vcvs, vccs, switch, capacitor, inductor and diode.  Any other kind is
refused with InputError rather than silently omitted, so a deck can never be
missing a component.  Capacitors and inductors are exported with zero initial
state; a non-zero initial condition is refused for the same reason.

The deck keeps the ngspice commands in a .control block and prints values as
``name = number`` lines, which is what ``parse_ngspice_output`` reads.  The
printed precision is raised with ``.options numdgt=15``.
"""

from __future__ import annotations

import math
import re

from .devices import source_dc, source_phasor
from .errors import InputError
from .netlist import Circuit, Component

EXPORTED_KINDS = {
    "resistor", "vsource", "isource", "vcvs", "vccs", "switch",
    "capacitor", "inductor", "diode",
}

_LETTER = {
    "resistor": "R",
    "vsource": "V",
    "isource": "I",
    "vcvs": "E",
    "vccs": "G",
    "capacitor": "C",
    "inductor": "L",
    "diode": "D",
}

PRINTED = re.compile(
    r"^\s*([A-Za-z_][A-Za-z0-9_()]*)\s*=\s*"
    r"([-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?)\s*$"
)


def _fmt(value: float) -> str:
    return repr(float(value))


def node_map(circuit: Circuit) -> dict[str, str]:
    mapping = {name: str(index + 1) for index, name in enumerate(circuit.node_names)}
    mapping[circuit.ground] = "0"
    return mapping


def _switch_line(index: int, component: Component, node) -> str:
    # ngspice needs a control voltage for its own switch; a resistor with the
    # same limit behaviour keeps the deck independent of an extra source.
    ohms = 1e-3 if component.state == "closed" else 1e12
    return f"R{index} {node(component.nodes[0])} {node(component.nodes[1])} {ohms:g}"


def _waveform_source(component: Component) -> str:
    waveform = component.waveform
    if waveform is None or waveform["type"] == "dc":
        return f"DC {_fmt(source_dc(component))}"
    kind = waveform["type"]
    if kind == "step":
        return f"PULSE(0 {_fmt(component.value)} {_fmt(waveform['delay_s'])} 1n 1n 1 2)"
    if kind == "sine":
        return (
            f"SIN({_fmt(waveform['offset'])} {_fmt(waveform['amplitude'])} "
            f"{_fmt(waveform['frequency_hz'])} 0 0 {_fmt(waveform['phase_deg'])})"
        )
    width = waveform["period_s"] * waveform["duty"]
    return (
        f"PULSE(0 {_fmt(component.value)} {_fmt(waveform['delay_s'])} 1n 1n "
        f"{_fmt(width)} {_fmt(waveform['period_s'])})"
    )


def _ac_source(component: Component) -> str:
    phasor = source_phasor(component)
    return f"DC {_fmt(source_dc(component))} AC {_fmt(abs(phasor))} {_fmt(math.degrees(math.atan2(phasor.imag, phasor.real)))}"


def element_lines(circuit: Circuit, deck_kind: str, nodes: dict[str, str] | None = None) -> list[str]:
    if deck_kind not in {"dc", "ac", "tran"}:
        raise InputError(f"SPICE deck kind must be 'dc', 'ac' or 'tran', got {deck_kind!r}")
    names = nodes or node_map(circuit)

    def node(name: str) -> str:
        try:
            return names[name]
        except KeyError:
            raise InputError(f"node {name!r} is not connected to anything in this deck") from None

    lines: list[str] = []
    for index, component in enumerate(circuit.components, start=1):
        kind = component.kind
        if kind not in EXPORTED_KINDS:
            raise InputError(f"cannot export component {component.id!r} of kind {kind!r} to SPICE")
        if kind in {"capacitor", "inductor"} and (component.initial or 0.0) != 0.0:
            raise InputError(f"cannot export component {component.id!r} with a non-zero initial condition")
        letter = _LETTER.get(kind)
        if kind == "resistor":
            lines.append(f"R{index} {node(component.nodes[0])} {node(component.nodes[1])} {_fmt(component.value)}")
        elif kind == "switch":
            lines.append(_switch_line(index, component, node))
        elif kind == "vsource":
            spec = _ac_source(component) if deck_kind == "ac" else _waveform_source(component)
            lines.append(f"V{index} {node(component.nodes[0])} {node(component.nodes[1])} {spec}")
        elif kind == "isource":
            spec = _ac_source(component) if deck_kind == "ac" else _waveform_source(component)
            lines.append(f"I{index} {node(component.nodes[0])} {node(component.nodes[1])} {spec}")
        elif kind == "vcvs":
            lines.append(
                f"{letter}{index} {node(component.nodes[0])} {node(component.nodes[1])} "
                f"{node(component.nodes[2])} {node(component.nodes[3])} {_fmt(component.value)}"
            )
        elif kind == "vccs":
            lines.append(
                f"{letter}{index} {node(component.nodes[0])} {node(component.nodes[1])} "
                f"{node(component.nodes[2])} {node(component.nodes[3])} {_fmt(component.value)}"
            )
        elif kind == "capacitor":
            lines.append(f"C{index} {node(component.nodes[0])} {node(component.nodes[1])} {_fmt(component.value)}")
        elif kind == "inductor":
            lines.append(f"L{index} {node(component.nodes[0])} {node(component.nodes[1])} {_fmt(component.value)}")
        elif kind == "diode":
            params = component.params
            lines.append(f"D{index} {node(component.nodes[0])} {node(component.nodes[1])} DM{index}")
            lines.append(
                f".model DM{index} D(IS={_fmt(params['is_a'])} N={_fmt(params['n'])} "
                f"RS={_fmt(params['rs_ohm'])})"
            )
    return lines


def build_deck(
    circuit: Circuit,
    deck_kind: str,
    *,
    frequency_hz: float | None = None,
    tran_step_s: float | None = None,
    tran_stop_s: float | None = None,
    measures: list[tuple[str, str, float]] | None = None,
) -> str:
    """Return a self-contained deck. ``measures`` is (name, node, time_s)."""
    nodes = node_map(circuit)
    title = " ".join(circuit.title.split()) or "circuit"
    lines = [f"* helqix cross-check: {title}"]
    lines.extend(element_lines(circuit, deck_kind, nodes))
    diode_temps = {c.params["t_deg_c"] for c in circuit.components if c.kind == "diode"}
    if len(diode_temps) > 1:
        raise InputError(f"cannot export diodes at different temperatures {sorted(diode_temps)} to one deck")
    if diode_temps:
        lines.append(f".options temp={_fmt(diode_temps.pop())}")
    lines.append(".options numdgt=15")
    lines.append(".control")
    if deck_kind == "dc":
        lines.append("op")
        lines.append("print " + " ".join(f"v({nodes[name]})" for name in circuit.node_names))
    elif deck_kind == "ac":
        if frequency_hz is None or not frequency_hz > 0:
            raise InputError("an AC deck needs a positive frequency_hz")
        lines.append(f"ac lin 1 {_fmt(frequency_hz)} {_fmt(frequency_hz)}")
        lines.append("print " + " ".join(
            f"re(v({nodes[name]})) im(v({nodes[name]}))" for name in circuit.node_names
        ))
    elif deck_kind == "tran":
        if tran_step_s is None or tran_stop_s is None or tran_step_s <= 0 or tran_stop_s <= 0:
            raise InputError("a transient deck needs positive tran_step_s and tran_stop_s")
        lines.append(f"tran {_fmt(tran_step_s)} {_fmt(tran_stop_s)}")
        for name, output_node, time_s in measures or []:
            lines.append(f"meas tran {name} find v({nodes[output_node]}) at={_fmt(time_s)}")
    lines.append(".endc")
    lines.append(".end")
    return "\n".join(lines) + "\n"


def parse_ngspice_output(text: str) -> dict[str, float]:
    values: dict[str, float] = {}
    for line in text.splitlines():
        match = PRINTED.match(line)
        if match:
            values[match.group(1)] = float(match.group(2))
    return values

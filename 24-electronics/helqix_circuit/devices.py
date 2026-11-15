"""Device equations. Constants are CODATA 2018 exact values (NIST)."""

from __future__ import annotations

import cmath
import math

BOLTZMANN_J_PER_K = 1.380649e-23  # CODATA 2018 exact, NIST
ELEMENTARY_CHARGE_C = 1.602176634e-19  # CODATA 2018 exact, NIST
CELSIUS_OFFSET = 273.15
EXP_LIMIT = 40.0


def thermal_voltage(t_deg_c: float) -> float:
    return BOLTZMANN_J_PER_K * (t_deg_c + CELSIUS_OFFSET) / ELEMENTARY_CHARGE_C


def diode_eval(vd: float, params: dict) -> tuple[float, float]:
    """Shockley junction: I = Is*(exp(Vd/(n*Vt)) - 1), g = dI/dVd."""
    vt = thermal_voltage(params["t_deg_c"])
    arg = vd / (params["n"] * vt)
    limited = min(arg, EXP_LIMIT)
    exp_term = math.exp(limited)
    current = params["is_a"] * (exp_term - 1.0)
    conductance = params["is_a"] * exp_term / (params["n"] * vt)
    return current, conductance


def mosfet_eval(vd: float, vg: float, vs: float, params: dict) -> tuple[float, dict[str, float]]:
    """Level-1 square-law model. Returns Id (drain to source) and dId/dV."""
    polarity = 1.0 if params["channel"] == "n" else -1.0
    vgs = polarity * (vg - vs)
    vds = polarity * (vd - vs)
    vth = params["vth"]
    k = params["k"]
    lam = params["lambda"]
    if vgs <= vth:
        return 0.0, {"g": 0.0, "d": 0.0, "s": 0.0}
    overdrive = vgs - vth
    if vds < overdrive:
        id_n = k * (overdrive * vds - 0.5 * vds * vds) * (1.0 + lam * vds)
        gm = k * vds * (1.0 + lam * vds)
        gds = k * (overdrive - vds) * (1.0 + lam * vds) + k * (overdrive * vds - 0.5 * vds * vds) * lam
    else:
        id_n = 0.5 * k * overdrive * overdrive * (1.0 + lam * vds)
        gm = k * overdrive * (1.0 + lam * vds)
        gds = 0.5 * k * overdrive * overdrive * lam
    current = polarity * id_n
    return current, {"g": gm, "d": gds, "s": -(gm + gds)}


def source_value(component, time_s: float) -> float:
    """Instantaneous value for transient and DC operating point."""
    waveform = component.waveform
    if waveform is None or waveform["type"] == "dc":
        return float(component.value)
    kind = waveform["type"]
    if kind == "step":
        return float(component.value) if time_s >= waveform["delay_s"] else 0.0
    if kind == "sine":
        phase = math.radians(waveform["phase_deg"])
        return waveform["offset"] + waveform["amplitude"] * math.sin(2.0 * math.pi * waveform["frequency_hz"] * time_s + phase)
    if kind == "pulse":
        t = time_s - waveform["delay_s"]
        if t < 0.0:
            return 0.0
        return float(component.value) if (t % waveform["period_s"]) < waveform["duty"] * waveform["period_s"] else 0.0
    return float(component.value)


def source_dc(component) -> float:
    waveform = component.waveform
    if waveform is None or waveform["type"] == "dc":
        return float(component.value)
    if waveform["type"] == "sine":
        return waveform["offset"]
    return 0.0


def source_phasor(component) -> complex:
    waveform = component.waveform
    if waveform is not None and waveform["type"] == "sine":
        return cmath.rect(waveform["amplitude"], math.radians(waveform["phase_deg"]))
    return complex(source_dc(component) if waveform is not None else float(component.value), 0.0)

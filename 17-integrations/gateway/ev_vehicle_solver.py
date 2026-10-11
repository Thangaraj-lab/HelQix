"""Bounded longitudinal EV load construction for the PyBaMM SPM adapter."""
from __future__ import annotations

import math
from typing import Any


def build_drive_cycle_load(config: dict[str, Any]) -> list[dict[str, float]]:
    """Convert a prescribed speed/grade trace to cell-current forcing inputs.

    This is a forward load calculation only. Negative wheel power is clipped
    (no regenerative braking model) and accessory demand remains positive.
    """
    points = config["drive_cycle"]
    pack = config["battery_pack"]
    mass = float(config["vehicle_mass_kg"])
    cda = float(config["drag_coefficient"]) * float(config["frontal_area_m2"])
    crr = float(config["rolling_resistance_coefficient"])
    efficiency = float(config["drivetrain_efficiency"])
    accessories = float(config["accessory_power_w"])
    nominal_voltage = float(pack["series_cells"]) * float(pack["cell_nominal_voltage_v"])
    parallel = int(pack["parallel_cells"])
    current_limit = float(pack["cell_max_continuous_current_a"])
    if nominal_voltage <= 0 or parallel <= 0:
        raise ValueError("Battery pack voltage and parallel count must be positive")

    output: list[dict[str, float]] = []
    distance = 0.0
    previous_time = float(points[0]["time_s"])
    previous_speed = float(points[0]["speed_m_s"])
    g = 9.80665
    rho = 1.225
    for index, point in enumerate(points):
        t = float(point["time_s"])
        speed = float(point["speed_m_s"])
        grade = float(point.get("road_grade_percent", 0.0)) / 100.0
        dt = t - previous_time if index else 0.0
        if dt < 0:
            raise ValueError("Drive-cycle times must be increasing")
        if index:
            acceleration = (speed - previous_speed) / dt
            distance += (speed + previous_speed) * 0.5 * dt
        else:
            acceleration = 0.0
        rolling = mass * g * crr
        aerodynamic = 0.5 * rho * cda * speed * speed
        grade_force = mass * g * grade
        wheel_force = mass * acceleration + rolling + aerodynamic + grade_force
        wheel_power = wheel_force * speed
        pack_power = max(0.0, wheel_power) / efficiency + accessories
        pack_current = pack_power / nominal_voltage
        cell_current = pack_current / parallel
        if not math.isfinite(cell_current):
            raise ValueError("Computed cell current is not finite")
        if cell_current > current_limit:
            raise ValueError(
                f"Drive cycle requires {cell_current:.1f} A per cell at {t:g} s, "
                f"above the configured {current_limit:.1f} A continuous limit"
            )
        output.append({
            "time_s": t,
            "speed_m_s": speed,
            "distance_m": distance,
            "wheel_force_n": wheel_force,
            "wheel_power_w": wheel_power,
            "pack_power_w": pack_power,
            "cell_current_a": cell_current,
        })
        previous_time, previous_speed = t, speed
    return output

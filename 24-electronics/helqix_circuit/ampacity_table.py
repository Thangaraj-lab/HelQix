"""Ampacity rows transcribed from a published copy of IEC 60364-5-52.

Table B.52.2 covers PVC-insulated copper and aluminium conductors with two
loaded conductors, a 70 degC conductor temperature and a 30 degC ambient
temperature, for installation methods A1, A2, B1, B2, C, E and F.

Only the copper columns have been transcribed.  Any size or material that is
not present here returns UNKNOWN and receives no ampacity verdict.
"""

from __future__ import annotations

METHODS = ("A1", "A2", "B1", "B2", "C", "E", "F")

TABLE_SOURCE = (
    "IEC 60364-5-52, Annex B, Table B.52.2 (PVC insulation, two loaded "
    "conductors, copper, conductor 70 degC, ambient 30 degC, methods A1-F); "
    "rows transcribed from the ECalPro published copy of the table "
    "(ecalpro.com, accessed 2026-11-14) and spot-checked against a second "
    "published copy (2.5 mm^2 method C = 27 A)"
)

# size_mm2 -> method -> current in amperes
PVC_COPPER: dict[float, dict[str, float]] = {
    1.5: {"A1": 14.5, "A2": 14.0, "B1": 17.5, "B2": 16.5, "C": 19.5, "E": 22.0, "F": 24.0},
    2.5: {"A1": 19.5, "A2": 18.5, "B1": 24.0, "B2": 23.0, "C": 27.0, "E": 30.0, "F": 33.0},
    4.0: {"A1": 26.0, "A2": 25.0, "B1": 32.0, "B2": 30.0, "C": 36.0, "E": 40.0, "F": 45.0},
    6.0: {"A1": 34.0, "A2": 32.0, "B1": 41.0, "B2": 38.0, "C": 46.0, "E": 51.0, "F": 57.0},
    10.0: {"A1": 46.0, "A2": 43.0, "B1": 57.0, "B2": 52.0, "C": 63.0, "E": 70.0, "F": 76.0},
    16.0: {"A1": 61.0, "A2": 57.0, "B1": 76.0, "B2": 69.0, "C": 85.0, "E": 94.0, "F": 101.0},
    25.0: {"A1": 80.0, "A2": 75.0, "B1": 101.0, "B2": 90.0, "C": 112.0, "E": 119.0, "F": 131.0},
    35.0: {"A1": 99.0, "A2": 92.0, "B1": 125.0, "B2": 111.0, "C": 138.0, "E": 148.0, "F": 162.0},
    50.0: {"A1": 119.0, "A2": 110.0, "B1": 151.0, "B2": 133.0, "C": 168.0, "E": 180.0, "F": 196.0},
    70.0: {"A1": 151.0, "A2": 139.0, "B1": 192.0, "B2": 168.0, "C": 213.0, "E": 232.0, "F": 251.0},
    95.0: {"A1": 182.0, "A2": 167.0, "B1": 232.0, "B2": 201.0, "C": 258.0, "E": 282.0, "F": 304.0},
    120.0: {"A1": 210.0, "A2": 192.0, "B1": 269.0, "B2": 232.0, "C": 299.0, "E": 328.0, "F": 352.0},
    150.0: {"A1": 240.0, "A2": 219.0, "B1": 300.0, "B2": 258.0, "C": 344.0, "E": 379.0, "F": 406.0},
    185.0: {"A1": 273.0, "A2": 248.0, "B1": 341.0, "B2": 294.0, "C": 392.0, "E": 434.0, "F": 463.0},
    240.0: {"A1": 320.0, "A2": 291.0, "B1": 400.0, "B2": 344.0, "C": 461.0, "E": 514.0, "F": 546.0},
}

SIZE_MATCH_RELATIVE = 0.01


def nominal_size_mm2(area_m2: float) -> float | None:
    """Return the tabulated nominal size within 1 % of the computed area."""
    area_mm2 = area_m2 * 1e6
    for size in PVC_COPPER:
        if abs(area_mm2 - size) <= SIZE_MATCH_RELATIVE * size:
            return size
    return None


def copper_ampacity(area_m2: float, method: str) -> tuple[float | None, float | None]:
    size = nominal_size_mm2(area_m2)
    if size is None:
        return None, None
    return PVC_COPPER[size][method], size

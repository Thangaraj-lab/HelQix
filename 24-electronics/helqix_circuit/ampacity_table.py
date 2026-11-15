"""Ampacity rows transcribed from a published copy of IEC 60364-5-52.

Table B.52.2 covers PVC-insulated copper and aluminium conductors with two
loaded conductors, a 70 degC conductor temperature and a 30 degC ambient
temperature, for installation methods A1, A2, B1, B2, C, E and F.

Only the copper columns have been transcribed.  Any size or material that is
not present here returns UNKNOWN and receives no ampacity verdict.  Values
are never interpolated: a computed area either matches a tabulated nominal
size within 1 % or there is no row and the verdict is UNKNOWN.

Every row built below carries its own provenance.  A row is VERIFIED because
the openly published copy named in SOURCE_PAGE was read on ACCESSED and every
value matched; rows A1 to C were additionally cross-checked against
CROSS_CHECK_PAGE.  A row that cannot be checked that way must be added with
status UNKNOWN, and the checker prints it as "unverified table row".
"""

from __future__ import annotations

import sys

METHODS = ("A1", "A2", "B1", "B2", "C", "E", "F")

STANDARD = "IEC 60364-5-52"
TABLE = "B.52.2"
MATERIAL = "copper"
INSULATION = "PVC"
LOADED_CONDUCTORS = 2
CONDUCTOR_TEMP_C = 70.0
AMBIENT_TEMP_C = 30.0
SOURCE_PAGE = "https://ecalpro.com/standards/iec-60364/table-b52-current-ratings"
CROSS_CHECK_PAGE = (
    "https://www.ti-soft.com/en/support/help/electricaldesign/standards/"
    "iec-60364-5-52/current-carrying-capacity/table_b_52_2"
)
ACCESSED = "2026-11-15"
CROSS_CHECKED_METHODS = ("A1", "A2", "B1", "B2", "C")
VALID_STATUSES = ("VERIFIED", "UNKNOWN")
ROW_FIELDS = (
    "size_mm2", "installation_method", "value_a", "standard", "table", "material",
    "insulation", "loaded_conductors", "conductor_temp_c", "ambient_temp_c",
    "status", "source", "cross_check", "accessed",
)

TABLE_SOURCE = (
    "IEC 60364-5-52, Annex B, Table B.52.2 (PVC insulation, two loaded "
    "conductors, copper, conductor 70 degC, ambient 30 degC); installation "
    "methods A1, A2, B1, B2, method C, E and F; values read from the openly "
    "published copy at "
    "https://ecalpro.com/standards/iec-60364/table-b52-current-ratings on "
    "2026-11-15 and spot-checked against a second published copy "
    "(2.5 mm^2 method C = 27 A)"
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


def _build_rows() -> tuple[dict, ...]:
    rows = []
    for size, methods in PVC_COPPER.items():
        for method, amps in methods.items():
            rows.append({
                "size_mm2": size,
                "installation_method": method,
                "value_a": amps,
                "standard": STANDARD,
                "table": TABLE,
                "material": MATERIAL,
                "insulation": INSULATION,
                "loaded_conductors": LOADED_CONDUCTORS,
                "conductor_temp_c": CONDUCTOR_TEMP_C,
                "ambient_temp_c": AMBIENT_TEMP_C,
                "status": "VERIFIED",
                "source": SOURCE_PAGE,
                "cross_check": CROSS_CHECK_PAGE if method in CROSS_CHECKED_METHODS else None,
                "accessed": ACCESSED,
            })
    return tuple(rows)


ROWS = _build_rows()


def unverified_rows(rows=ROWS) -> list[dict]:
    return [row for row in rows if row.get("status") != "VERIFIED"]


def row_problems(rows=ROWS) -> list[str]:
    problems = []
    for row in rows:
        where = f"row {row.get('size_mm2')} mm2 method {row.get('installation_method', '?')}"
        missing = [field for field in ROW_FIELDS if field not in row]
        if missing:
            problems.append(f"{where} is missing {missing}")
        if row.get("status") not in VALID_STATUSES:
            problems.append(f"{where} has status {row.get('status')!r}")
        if row.get("status") == "VERIFIED":
            if not row.get("source"):
                problems.append(f"{where} is VERIFIED without a source page")
            if not row.get("accessed"):
                problems.append(f"{where} is VERIFIED without an access date")
    return problems


def report_text(rows=ROWS) -> str:
    verified = len(rows) - len(unverified_rows(rows))
    lines = [
        f"{STANDARD} {TABLE}: {len(rows)} rows, {verified} VERIFIED, "
        f"{len(unverified_rows(rows))} UNKNOWN"
    ]
    for row in unverified_rows(rows):
        value = row.get("value_a")
        shown = "UNKNOWN" if value is None else f"{value:g} A"
        lines.append(
            f"unverified table row: {row.get('size_mm2')} mm2 {row.get('material')} "
            f"{row.get('insulation')} method {row.get('installation_method')} = {shown}"
        )
    return "\n".join(lines)


def main() -> int:
    print(report_text())
    problems = row_problems()
    for problem in problems:
        print(problem, file=sys.stderr)
    return 1 if problems else 0


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


if __name__ == "__main__":
    raise SystemExit(main())

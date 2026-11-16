"""harness/0.1 document: connectors, wires, bundles and computed routes.

The document is mechanical data only.  A wire names its two circuit nodes and
the two connector pins it runs between; the route polyline is computed from
the pin positions and the obstacles by ``routing.route`` and is never read
from the input.  Colours are looked up in a transcribed IEC 60757 table and
carry the IEC 60445 conductor role where one exists, labelled REPORTED.
"""

from __future__ import annotations

import math
from dataclasses import dataclass

from .errors import InputError
from .harness import MATERIALS, awg_diameter_mm
from .routing import Box, Route, route

VERSION = "harness/0.1"
TOP_KEYS = ("bundles", "connectors", "obstacles", "version", "wires")
WIRE_KEYS = (
    "bend_radius_mm",
    "bundle",
    "diameter_mm",
    "from",
    "from_pin",
    "gauge_awg",
    "id",
    "insulation",
    "length_mm",
    "material",
    "slack_mm",
    "to",
    "to_pin",
)
COLOUR_SOURCE = (
    "IEC 60757 code letters and IEC 60445 conductor colour identification, "
    "transcribed from the published tables and not consulted online in this "
    "offline environment, so REPORTED"
)
ROUTE_NOTE = (
    "routes are computed from the two pin positions, the wire bend radius and "
    "the obstacles; the input has no polyline field and a supplied polyline is refused"
)
COLOURS = {
    "black": ("BK", "L2 phase"),
    "brown": ("BN", "L1 phase"),
    "red": ("RD", None),
    "orange": ("OG", None),
    "yellow": ("YE", None),
    "green": ("GN", None),
    "blue": ("BU", "neutral"),
    "violet": ("VT", None),
    "grey": ("GY", "L3 phase"),
    "white": ("WH", None),
    "pink": ("PK", None),
    "turquoise": ("TQ", None),
    "green-yellow": ("GNYE", "protective earth"),
}
_CODES = {code.lower(): name for name, (code, _) in COLOURS.items()}


def _number(value, what: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise InputError(f"{what} must be a number")
    number = float(value)
    if not math.isfinite(number):
        raise InputError(f"{what} must be finite")
    return number


def _point(value, what: str) -> tuple[float, float]:
    try:
        x, y = value
    except (TypeError, ValueError):
        raise InputError(f"{what} must be a pair of numbers") from None
    return (_number(x, f"{what} x"), _number(y, f"{what} y"))


def _text(value, what: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise InputError(f"{what} must be a non-empty string")
    return value.strip()


def _mm(value: float) -> dict:
    return {"value": value, "unit": "mm", "label": "GIVEN"}


def _given(value: float) -> dict:
    return {"value": value, "label": "GIVEN"}


def _calculated(value: float, unit: str) -> dict:
    return {"value": value, "unit": unit, "label": "CALCULATED"}


def colour_entry(raw) -> dict:
    """Look a colour name or IEC 60757 letter up; unknown colours are refused."""
    text = _text(raw, "insulation colour")
    key = text.lower()
    name = key if key in COLOURS else _CODES.get(key)
    if name is None:
        known = ", ".join(sorted(COLOURS))
        raise InputError(f"insulation colour {text!r} is unknown; use one of {known} or an IEC 60757 code")
    code, role = COLOURS[name]
    return {"name": name, "code": code, "role": role, "label": "REPORTED"}


@dataclass(frozen=True)
class Connector:
    id: str
    at_mm: tuple[float, float]
    pins: dict[str, tuple[float, float]]

    def to_dict(self) -> dict:
        return {
            "id": self.id,
            "at_mm": _mm_pair(self.at_mm),
            "pins": {pin: _mm_pair(point) for pin, point in self.pins.items()},
        }


def _mm_pair(point: tuple[float, float]) -> dict:
    return {"value": [point[0], point[1]], "unit": "mm", "label": "GIVEN"}


@dataclass(frozen=True)
class HarnessWire:
    id: str
    from_node: str
    to_node: str
    from_pin: str
    to_pin: str
    from_at: tuple[float, float]
    to_at: tuple[float, float]
    gauge_awg: float | None
    diameter_mm: float
    area_mm2: float
    material: str
    colour: dict
    insulation_rating_c: float
    length_mm: float
    slack_mm: float
    bend_radius_mm: float
    bundle: str | None
    route: Route

    def to_dict(self) -> dict:
        blocked = self.route.blocked
        if blocked:
            margin = {"value": None, "unit": "mm", "label": "UNKNOWN"}
            fits = None
        else:
            margin = _calculated(self.length_mm - self.route.cut_length_mm, "mm")
            fits = self.length_mm + 1e-9 >= self.route.cut_length_mm
        return {
            "id": self.id,
            "from": self.from_node,
            "to": self.to_node,
            "from_pin": {"ref": self.from_pin, "at_mm": _mm_pair(self.from_at)},
            "to_pin": {"ref": self.to_pin, "at_mm": _mm_pair(self.to_at)},
            "gauge_awg": _given(self.gauge_awg) if self.gauge_awg is not None else {"value": None, "label": "GIVEN"},
            "diameter_mm": _calculated(self.diameter_mm, "mm"),
            "area_mm2": _calculated(self.area_mm2, "mm^2"),
            "material": self.material,
            "insulation": {"colour": dict(self.colour), "rating_c": {"value": self.insulation_rating_c, "unit": "degC", "label": "GIVEN"}},
            "length_mm": _mm(self.length_mm),
            "slack_mm": _mm(self.slack_mm),
            "bend_radius_mm": _mm(self.bend_radius_mm),
            "bundle": self.bundle,
            "route": self.route.to_dict(),
            "length_margin_mm": margin,
            "route_fits": fits,
        }


@dataclass(frozen=True)
class HarnessDoc:
    connectors: tuple[Connector, ...]
    wires: tuple[HarnessWire, ...]
    bundles: tuple[dict, ...]
    obstacles: tuple[dict, ...]

    def to_dict(self) -> dict:
        return {
            "version": VERSION,
            "route_note": ROUTE_NOTE,
            "colour_source": COLOUR_SOURCE,
            "connectors": [connector.to_dict() for connector in self.connectors],
            "wires": [wire.to_dict() for wire in self.wires],
            "bundles": [dict(bundle) for bundle in self.bundles],
            "obstacles": [dict(obstacle) for obstacle in self.obstacles],
        }


def _connectors(raw, what: str) -> tuple[dict[str, Connector], list[Connector]]:
    if not isinstance(raw, list):
        raise InputError(f"{what} must be a list")
    by_id: dict[str, Connector] = {}
    ordered: list[Connector] = []
    for index, body in enumerate(raw):
        where = f"connector #{index + 1}"
        if not isinstance(body, dict):
            raise InputError(f"{where} must be an object")
        cid = _text(body.get("id"), f"{where} id")
        if "." in cid:
            raise InputError(f"{where} id {cid!r} must not contain a dot; pin refs use 'connector.pin'")
        if cid in by_id:
            raise InputError(f"duplicate connector id {cid!r}")
        at = _point(body.get("at_mm"), f"{where} at_mm")
        pins = body.get("pins")
        if not isinstance(pins, dict) or not pins:
            raise InputError(f"{where} needs a non-empty pins object")
        points: dict[str, tuple[float, float]] = {}
        for pin, value in pins.items():
            pin_id = _text(pin, f"{where} pin id")
            if "." in pin_id:
                raise InputError(f"{where} pin id {pin_id!r} must not contain a dot")
            points[pin_id] = _point(value, f"{where} pin {pin_id!r} at_mm")
        connector = Connector(cid, at, points)
        by_id[cid] = connector
        ordered.append(connector)
    return by_id, ordered


def _resolve_pin(connectors, ref, what: str) -> tuple[str, str, tuple[float, float]]:
    text = _text(ref, what)
    connector_id, _, pin_id = text.partition(".")
    connector = connectors.get(connector_id)
    if connector is None or not pin_id or pin_id not in connector.pins:
        raise InputError(f"{what} {text!r} is an unknown pin; use the form 'connector.pin'")
    return text, connector_id, connector.pins[pin_id]


def _bundles(raw) -> tuple[dict[str, dict], list[dict]]:
    if raw is None:
        raw = []
    if not isinstance(raw, list):
        raise InputError("bundles must be a list")
    by_id: dict[str, dict] = {}
    ordered: list[dict] = []
    for index, body in enumerate(raw):
        where = f"bundle #{index + 1}"
        if not isinstance(body, dict):
            raise InputError(f"{where} must be an object")
        bid = _text(body.get("id"), f"{where} id")
        if bid in by_id:
            raise InputError(f"duplicate bundle id {bid!r}")
        entry = {"id": bid}
        if "diameter_mm" in body:
            diameter = _number(body["diameter_mm"], f"{where} diameter_mm")
            if diameter <= 0:
                raise InputError(f"{where} diameter_mm must be greater than zero")
            entry["diameter_mm"] = _mm(diameter)
        by_id[bid] = entry
        ordered.append(entry)
    return by_id, ordered


def _obstacles(raw) -> tuple[list[Box], list[dict]]:
    if raw is None:
        raw = []
    if not isinstance(raw, list):
        raise InputError("obstacles must be a list")
    seen: set[str] = set()
    boxes: list[Box] = []
    ordered: list[dict] = []
    for index, body in enumerate(raw):
        where = f"obstacle #{index + 1}"
        if not isinstance(body, dict):
            raise InputError(f"{where} must be an object")
        oid = _text(body.get("id"), f"{where} id")
        if oid in seen:
            raise InputError(f"duplicate obstacle id {oid!r}")
        seen.add(oid)
        corners = body.get("box_mm")
        try:
            x0, y0, x1, y1 = corners
        except (TypeError, ValueError):
            raise InputError(f"{where} box_mm must be four numbers x0, y0, x1, y1") from None
        box = Box(
            _number(x0, f"{where} x0"),
            _number(y0, f"{where} y0"),
            _number(x1, f"{where} x1"),
            _number(y1, f"{where} y1"),
        )
        boxes.append(box)
        ordered.append({"id": oid, "box_mm": [box.x0, box.y0, box.x1, box.y1]})
    return boxes, ordered


def _wire(index: int, body, connectors, bundles, boxes) -> HarnessWire:
    where = f"wire #{index + 1}"
    if not isinstance(body, dict):
        raise InputError(f"{where} must be an object")
    unknown = sorted(set(body) - set(WIRE_KEYS))
    if unknown:
        raise InputError(f"{where} has unknown wire key(s) {unknown}; allowed keys: {list(WIRE_KEYS)}")
    wid = _text(body.get("id"), f"{where} id")
    where = f"wire {wid!r}"
    from_node = _text(body.get("from"), f"{where} from")
    to_node = _text(body.get("to"), f"{where} to")
    if from_node == to_node:
        raise InputError(f"{where} from and to must be different circuit nodes")
    from_pin, _, from_at = _resolve_pin(connectors, body.get("from_pin"), f"{where} from_pin")
    to_pin, _, to_at = _resolve_pin(connectors, body.get("to_pin"), f"{where} to_pin")

    gauge_body = body.get("gauge_awg")
    diameter_body = body.get("diameter_mm")
    if (gauge_body is None) == (diameter_body is None):
        raise InputError(f"{where} needs exactly one of gauge_awg or diameter_mm")
    gauge: float | None = None
    if gauge_body is not None:
        gauge = _number(gauge_body, f"{where} gauge_awg")
        if not -20.0 < gauge < 60.0:
            raise InputError(f"{where} gauge_awg must be between -20 and 60")
        diameter = awg_diameter_mm(gauge)
    else:
        diameter = _number(diameter_body, f"{where} diameter_mm")
        if diameter <= 0:
            raise InputError(f"{where} diameter_mm must be greater than zero")
    area = math.pi * (diameter / 2.0) ** 2

    material = body.get("material")
    if material not in MATERIALS:
        raise InputError(f"{where} material must be one of {sorted(MATERIALS)}, got {material!r}")
    insulation = body.get("insulation")
    if not isinstance(insulation, dict):
        raise InputError(f"{where} insulation must be an object with colour and rating_c")
    colour = colour_entry(insulation.get("colour"))
    rating = _number(insulation.get("rating_c"), f"{where} insulation rating_c")
    if rating <= 0:
        raise InputError(f"{where} insulation rating_c must be greater than zero")

    length = _number(body.get("length_mm"), f"{where} length_mm")
    if length <= 0:
        raise InputError(f"{where} length_mm must be greater than zero")
    slack = _number(body.get("slack_mm", 0.0), f"{where} slack_mm")
    if slack < 0:
        raise InputError(f"{where} slack_mm must be zero or greater")
    radius = _number(body.get("bend_radius_mm"), f"{where} bend_radius_mm")
    if radius <= 0:
        raise InputError(f"{where} bend_radius_mm must be greater than zero")
    bundle = body.get("bundle")
    if bundle is not None:
        bundle = _text(bundle, f"{where} bundle")
        if bundle not in bundles:
            raise InputError(f"{where} bundle {bundle!r} is an unknown bundle; declare it in bundles")

    return HarnessWire(
        id=wid,
        from_node=from_node,
        to_node=to_node,
        from_pin=from_pin,
        to_pin=to_pin,
        from_at=from_at,
        to_at=to_at,
        gauge_awg=gauge,
        diameter_mm=diameter,
        area_mm2=area,
        material=material,
        colour=colour,
        insulation_rating_c=rating,
        length_mm=length,
        slack_mm=slack,
        bend_radius_mm=radius,
        bundle=bundle,
        route=route(from_at, to_at, boxes, bend_radius_mm=radius),
    )


def load_harness(body) -> HarnessDoc:
    """Validate a harness/0.1 object and compute every wire route."""
    if not isinstance(body, dict):
        raise InputError("harness document must be a JSON object")
    if body.get("version") != VERSION:
        raise InputError(f"harness version must be {VERSION!r}, got {body.get('version')!r}")
    unknown = sorted(set(body) - set(TOP_KEYS))
    if unknown:
        raise InputError(f"unknown harness key(s) {unknown}; allowed keys: {list(TOP_KEYS)}")

    connectors, connector_list = _connectors(body.get("connectors"), "connectors")
    bundles, bundle_list = _bundles(body.get("bundles"))
    boxes, obstacle_list = _obstacles(body.get("obstacles"))

    raw_wires = body.get("wires")
    if not isinstance(raw_wires, list) or not raw_wires:
        raise InputError("wires must be a non-empty list")
    wires: list[HarnessWire] = []
    seen: set[str] = set()
    for index, wire_body in enumerate(raw_wires):
        wire = _wire(index, wire_body, connectors, bundles, boxes)
        if wire.id in seen:
            raise InputError(f"duplicate wire id {wire.id!r}")
        seen.add(wire.id)
        wires.append(wire)
    return HarnessDoc(tuple(connector_list), tuple(wires), tuple(bundle_list), tuple(obstacle_list))

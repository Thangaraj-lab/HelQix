"""Deterministic 2D harness routing around axis-aligned boxes.

A route is a polyline of axis-aligned segments whose corners are replaced by
quarter-circle arcs of the wire's minimum bend radius, so the cut length is
shorter than the sharp polyline.  A clear direct shot stays a single straight
segment even when it is diagonal.  Lengths are in millimetres.  The router
tries, in order, the direct segment, an L through one of the two corners, and
a shortest path on a visibility grid built from the start, the end and the
inflated box edges.  Equal-cost grid paths resolve toward the lower y then
the lower x, so the same input always gives the same route.  Anything it
cannot route within the bend radius is returned as blocked with a plain-word
reason, never drawn sharp.  Legs are checked against the boxes inflated by
bend radius + clearance; the corner arc needs no separate test because it
lies in an R x R square bounded by the two legs and every inflated box is at
least 2R wide in both axes, so a box that could reach the arc must cross one
of those legs first.
"""

from __future__ import annotations

import heapq
import math
from dataclasses import dataclass

from .errors import InputError

EPS = 1e-9
BEND_PENALTY_MM = 1e-6


class _Infeasible(Exception):
    pass


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


@dataclass(frozen=True)
class Box:
    """Axis-aligned obstacle in the routing plane, edges at x0 < x1, y0 < y1."""

    x0: float
    y0: float
    x1: float
    y1: float

    def __post_init__(self) -> None:
        if not (self.x0 < self.x1 and self.y0 < self.y1):
            raise InputError("an obstacle box needs x0 < x1 and y0 < y1")

    def inflate(self, margin: float) -> "Box":
        return Box(self.x0 - margin, self.y0 - margin, self.x1 + margin, self.y1 + margin)

    def contains_strict(self, point: tuple[float, float]) -> bool:
        x, y = point
        return self.x0 + EPS < x < self.x1 - EPS and self.y0 + EPS < y < self.y1 - EPS

    def hits_segment(self, a: tuple[float, float], b: tuple[float, float]) -> bool:
        """Liang-Barsky against the box shrunk by EPS, so a segment on the edge is clear."""
        x0, y0, x1, y1 = self.x0 + EPS, self.y0 + EPS, self.x1 - EPS, self.y1 - EPS
        if x0 >= x1 or y0 >= y1:
            return False
        dx, dy = b[0] - a[0], b[1] - a[1]
        low, high = 0.0, 1.0
        for p, q in ((-dx, a[0] - x0), (dx, x1 - a[0]), (-dy, a[1] - y0), (dy, y1 - a[1])):
            if p == 0.0:
                if q < 0.0:
                    return False
                continue
            t = q / p
            if p < 0.0:
                if t > high:
                    return False
                low = max(low, t)
            else:
                if t < low:
                    return False
                high = min(high, t)
        return high > low


@dataclass(frozen=True)
class Route:
    points: tuple[tuple[float, float], ...] = ()
    polyline_length_mm: float = 0.0
    cut_length_mm: float = 0.0
    bends: int = 0
    blocked: bool = False
    reason: str = ""

    def to_dict(self) -> dict:
        points = [[x, y] for x, y in self.points]
        if self.blocked:
            length = {"value": None, "unit": "mm", "label": "UNKNOWN"}
            return {
                "points_mm": points,
                "cut_length_mm": length,
                "polyline_length_mm": length,
                "bends": None,
                "blocked": True,
                "reason": self.reason,
            }
        return {
            "points_mm": points,
            "cut_length_mm": {"value": self.cut_length_mm, "unit": "mm", "label": "CALCULATED"},
            "polyline_length_mm": {"value": self.polyline_length_mm, "unit": "mm", "label": "CALCULATED"},
            "bends": self.bends,
            "blocked": False,
            "reason": "",
        }


def _clear(a: tuple[float, float], b: tuple[float, float], boxes) -> bool:
    return not any(box.hits_segment(a, b) for box in boxes)


def _compress(points) -> list[tuple[float, float]]:
    out: list[tuple[float, float]] = []
    for point in points:
        if out and point == out[-1]:
            continue
        if len(out) >= 2:
            a, b = out[-2], out[-1]
            if (a[0] == b[0] == point[0]) or (a[1] == b[1] == point[1]):
                out[-1] = point
                continue
        out.append(point)
    return out


def _finish(points, radius: float) -> Route:
    path = _compress(points)
    if len(path) < 2:
        raise _Infeasible("the path collapses to a single point")
    legs = [math.dist(a, b) for a, b in zip(path, path[1:])]
    bends = len(path) - 2
    for index, leg in enumerate(legs):
        inner = 0 < index < len(legs) - 1
        need = 2.0 * radius if inner else radius
        if leg < need - EPS:
            raise _Infeasible(
                f"a straight section is {leg:.3f} mm but the {radius:g} mm bend radius needs "
                f"at least {need:g} mm there"
            )
    polyline = sum(legs)
    cut = polyline - 2.0 * radius * bends + bends * math.pi * radius / 2.0
    return Route(tuple(path), polyline, cut, bends)


def _grid_path(start, end, boxes) -> list[tuple[float, float]] | None:
    xs = sorted({start[0], end[0]} | {box.x0 for box in boxes} | {box.x1 for box in boxes})
    ys = sorted({start[1], end[1]} | {box.y0 for box in boxes} | {box.y1 for box in boxes})
    node_ids: dict[tuple[int, int], int] = {}
    coords: list[tuple[float, float]] = []
    for iy, y in enumerate(ys):
        for ix, x in enumerate(xs):
            if any(box.contains_strict((x, y)) for box in boxes):
                continue
            node_ids[(ix, iy)] = len(coords)
            coords.append((x, y))
    start_id = node_ids.get((xs.index(start[0]), ys.index(start[1])))
    end_id = node_ids.get((xs.index(end[0]), ys.index(end[1])))
    if start_id is None or end_id is None:
        return None

    count = len(coords) * 3
    best = [math.inf] * count
    parent: list[int | None] = [None] * count
    cache: dict[tuple[int, int], bool] = {}
    heap: list[tuple[float, float, float, int, int]] = []
    sequence = 0
    heapq.heappush(heap, (0.0, coords[start_id][1], coords[start_id][0], sequence, start_id * 3))
    best[start_id * 3] = 0.0

    def clear(a: int, b: int) -> bool:
        key = (min(a, b), max(a, b))
        if key not in cache:
            cache[key] = _clear(coords[a], coords[b], boxes)
        return cache[key]

    end_state = None
    while heap:
        cost, _, _, _, state = heapq.heappop(heap)
        if cost > best[state] + 1e-12:
            continue
        node, direction = divmod(state, 3)
        if node == end_id and direction != 0:
            end_state = state
            break
        ix = xs.index(coords[node][0])
        iy = ys.index(coords[node][1])
        for nix, niy, next_direction in ((ix + 1, iy, 1), (ix - 1, iy, 1), (ix, iy + 1, 2), (ix, iy - 1, 2)):
            other = node_ids.get((nix, niy))
            if other is None or not clear(node, other):
                continue
            step = math.dist(coords[node], coords[other])
            extra = 0.0 if direction in (0, next_direction) else BEND_PENALTY_MM
            next_state = other * 3 + next_direction
            if cost + step + extra < best[next_state] - 1e-12:
                best[next_state] = cost + step + extra
                parent[next_state] = state
                sequence += 1
                heapq.heappush(
                    heap,
                    (best[next_state], coords[other][1], coords[other][0], sequence, next_state),
                )
    if end_state is None:
        return None
    path = [coords[end_state // 3]]
    state = parent[end_state]
    while state is not None:
        path.append(coords[state // 3])
        state = parent[state]
    path.reverse()
    return path


def route(start, end, obstacles, *, bend_radius_mm: float, clearance_mm: float = 0.0) -> Route:
    """Route from start to end around boxes, honouring the minimum bend radius."""
    radius = _number(bend_radius_mm, "bend_radius_mm")
    if radius <= 0.0:
        raise InputError("bend_radius_mm must be greater than zero")
    clearance = _number(clearance_mm, "clearance_mm")
    if clearance < 0.0:
        raise InputError("clearance_mm must be zero or greater")
    a = _point(start, "start")
    b = _point(end, "end")
    if math.dist(a, b) <= EPS:
        raise InputError("start and end are the same point")
    if not isinstance(obstacles, (list, tuple)):
        raise InputError("obstacles must be a list of boxes")
    boxes = list(obstacles)
    for index, box in enumerate(boxes):
        if not isinstance(box, Box):
            raise InputError(f"obstacle #{index + 1} is not a Box")
    inflated = [box.inflate(radius + clearance) for box in boxes]
    for name, point in (("start", a), ("end", b)):
        if any(box.contains_strict(point) for box in inflated):
            raise InputError(f"{name} point lies inside an obstacle inflated by the bend radius")

    if _clear(a, b, inflated):
        return _finish([a, b], radius)

    for corner in ((b[0], a[1]), (a[0], b[1])):
        if corner in (a, b):
            continue
        legs = (math.dist(a, corner), math.dist(corner, b))
        if min(legs) < radius - EPS:
            continue
        if not (_clear(a, corner, inflated) and _clear(corner, b, inflated)):
            continue
        try:
            return _finish([a, corner, b], radius)
        except _Infeasible:
            continue

    path = _grid_path(a, b, inflated)
    if path is None:
        return Route(blocked=True, reason="no clear path between the endpoints; the inflated obstacles seal the way")
    try:
        return _finish(path, radius)
    except _Infeasible as exc:
        return Route(blocked=True, reason=f"no clear path between the endpoints; {exc}")

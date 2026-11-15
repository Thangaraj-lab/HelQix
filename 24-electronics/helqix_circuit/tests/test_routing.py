"""Routing helper: straight, one right-angle, around boxes, blocked."""

import math
import random

import pytest

from helqix_circuit import InputError
from helqix_circuit.routing import Box, route


def test_straight_route_has_no_bends_and_exact_cut_length():
    result = route((0.0, 0.0), (100.0, 0.0), [], bend_radius_mm=5.0)
    assert result.blocked is False
    assert result.points == ((0.0, 0.0), (100.0, 0.0))
    assert result.bends == 0
    assert result.cut_length_mm == pytest.approx(100.0, abs=1e-9)
    assert result.polyline_length_mm == pytest.approx(100.0, abs=1e-9)


def test_diagonal_free_route_keeps_the_direct_segment():
    result = route((0.0, 0.0), (30.0, 40.0), [], bend_radius_mm=5.0)
    assert result.points == ((0.0, 0.0), (30.0, 40.0))
    assert result.cut_length_mm == pytest.approx(50.0, abs=1e-9)


def test_one_right_angle_is_cut_by_the_bend_radius():
    blocker = Box(45.0, 25.0, 55.0, 35.0)
    result = route((0.0, 0.0), (100.0, 60.0), [blocker], bend_radius_mm=5.0)
    assert result.blocked is False
    assert result.points == ((0.0, 0.0), (100.0, 0.0), (100.0, 60.0))
    assert result.bends == 1
    assert result.polyline_length_mm == pytest.approx(160.0, abs=1e-9)
    expected = 160.0 - 2.0 * 5.0 + math.pi * 5.0 / 2.0
    assert result.cut_length_mm == pytest.approx(expected, abs=1e-9)


def test_route_goes_around_a_central_obstacle_with_one_radius_clearance():
    blocker = Box(40.0, -10.0, 60.0, 10.0)
    result = route((0.0, 0.0), (100.0, 0.0), [blocker], bend_radius_mm=5.0)
    assert result.blocked is False
    assert result.points == ((0.0, 0.0), (0.0, -15.0), (100.0, -15.0), (100.0, 0.0))
    assert result.bends == 2
    assert result.polyline_length_mm == pytest.approx(130.0, abs=1e-9)
    expected = 130.0 - 4.0 * 5.0 + 2.0 * (math.pi * 5.0 / 2.0)
    assert result.cut_length_mm == pytest.approx(expected, abs=1e-9)


def test_sealed_destination_is_blocked_with_a_plain_reason():
    frame = [
        Box(90.0, 10.0, 110.0, 12.0),
        Box(90.0, -12.0, 110.0, -10.0),
        Box(88.0, -12.0, 90.0, 12.0),
        Box(110.0, -12.0, 112.0, 12.0),
    ]
    result = route((0.0, 0.0), (100.0, 0.0), frame, bend_radius_mm=5.0)
    assert result.blocked is True
    assert result.points == ()
    assert result.reason.startswith("no clear path")


def test_corner_shorter_than_the_bend_radius_is_not_routed():
    blocker = Box(0.5, 8.0, 1.5, 12.0)
    result = route((0.0, 0.0), (2.0, 20.0), [blocker], bend_radius_mm=5.0)
    assert result.blocked is True
    assert "bend radius" in result.reason


def test_wire_keeps_its_radius_from_a_box_near_the_chosen_corner():
    central = Box(45.0, 25.0, 55.0, 35.0)
    near_corner = Box(101.0, 6.0, 104.0, 9.0)
    result = route((0.0, 0.0), (100.0, 60.0), [central, near_corner], bend_radius_mm=5.0)
    assert result.blocked is False
    assert result.points == ((0.0, 0.0), (0.0, 60.0), (100.0, 60.0))
    assert result.bends == 1
    expected = 160.0 - 2.0 * 5.0 + math.pi * 5.0 / 2.0
    assert result.cut_length_mm == pytest.approx(expected, abs=1e-9)


def test_bad_input_is_refused():
    with pytest.raises(InputError, match="bend_radius_mm"):
        route((0.0, 0.0), (10.0, 0.0), [], bend_radius_mm=0.0)
    with pytest.raises(InputError, match="inside an obstacle"):
        route((0.0, 0.0), (10.0, 0.0), [Box(-1.0, -1.0, 1.0, 1.0)], bend_radius_mm=5.0)
    with pytest.raises(InputError, match="same point"):
        route((0.0, 0.0), (0.0, 0.0), [], bend_radius_mm=5.0)


@pytest.mark.nightly
@pytest.mark.parametrize("seed", list(range(20)))
def test_route_properties_hold_for_seeded_cases(seed):
    rng = random.Random(seed)
    start = (0.0, 0.0)
    end = (rng.uniform(40.0, 160.0), rng.uniform(0.0, 120.0))
    boxes = []
    for _ in range(3):
        x0 = rng.uniform(10.0, end[0] - 10.0)
        y0 = rng.uniform(-20.0, end[1] + 20.0)
        boxes.append(Box(x0, y0, x0 + rng.uniform(4.0, 20.0), y0 + rng.uniform(4.0, 20.0)))
    radius = rng.uniform(1.0, 8.0)
    first = route(start, end, boxes, bend_radius_mm=radius)
    second = route(start, end, boxes, bend_radius_mm=radius)
    assert first == second, "the same input must give the same route"
    if first.blocked:
        return
    straight = math.dist(start, end)
    assert first.cut_length_mm >= straight - 1e-9, "a cut path is never shorter than a straight line"
    assert first.points[0] == start and first.points[-1] == end
    if first.bends == 0:
        assert len(first.points) == 2, "a bend-free route is the direct segment"
    else:
        for a, b in zip(first.points, first.points[1:]):
            assert a[0] == b[0] or a[1] == b[1], "every segment is axis-aligned"
    assert first.cut_length_mm <= first.polyline_length_mm + 1e-9

"""Source identity is portable across independent JSON payloads and processes."""
import json
import math
import operator
import subprocess
import sys

import numpy as np
import pytest

from concorde import Quantity, UncertainValue, UnitRegistry


def restored(value):
    return UncertainValue.from_json(value.to_json())


def test_separate_payloads_reconnect_shared_sources():
    x = UncertainValue(3., .2)
    y = 2 * x
    a, b = restored(x), restored(y)
    assert a.value == x.value
    assert b.value == y.value
    assert a.uncertainty == pytest.approx(.2)
    assert b.uncertainty == pytest.approx(.4)
    assert (b - 2*a).uncertainty == pytest.approx(0., abs=1e-12)
    assert a.variable_id == x.variable_id
    assert a.is_independent()
    assert not b.is_independent()
    assert (restored(x) - restored(x)).uncertainty == pytest.approx(0.)


def test_equal_numbers_do_not_merge_independent_sources():
    a, b = restored(UncertainValue(3., .2)), restored(UncertainValue(3., .2))
    assert (a-b).uncertainty == pytest.approx(math.sqrt(2)*.2)


def test_dependencies_survive_a_fresh_process():
    x = UncertainValue(3., .2)
    y = 2*x
    code = """
import sys
from concorde import UncertainValue
x = UncertainValue.from_json(sys.stdin.readline())
y = UncertainValue.from_json(sys.stdin.readline())
assert (y - 2*x).uncertainty == 0
print(y.to_json())
"""
    result = subprocess.run([sys.executable, "-c", code], input=x.to_json()+"\n"+y.to_json()+"\n", text=True, capture_output=True, check=True)
    assert (UncertainValue.from_json(result.stdout) - y).uncertainty == pytest.approx(0.)


def test_multiple_sources_nonlinearity_and_zero_uncertainty():
    x, y = UncertainValue(3., .2), UncertainValue(5., .3)
    z = (x*y + x) / y
    loaded = restored(z)
    assert loaded.value == z.value
    assert loaded.uncertainty == pytest.approx(z.uncertainty)
    assert (loaded - z).uncertainty == pytest.approx(0., abs=1e-12)
    assert restored(UncertainValue(3., 0.)).uncertainty == 0.
    assert restored(x * 0).uncertainty == 0.


def test_indexed_source_remains_connected_to_parent():
    x = UncertainValue(np.array([1., 2.]), np.array([.1, .2]))
    assert (restored(x[1]) - x[1]).uncertainty == pytest.approx(0.)
    np.testing.assert_allclose((x - restored(x[0])).uncertainty, [0., math.hypot(.1, .2)])


@pytest.mark.parametrize("op", [operator.add, operator.sub, operator.mul, operator.truediv])
def test_conflicting_separately_loaded_sources_are_rejected(op):
    x = UncertainValue(3., .2)
    payload = json.loads(x.to_json())
    payload["sources"][0]["uncertainty"] = .9
    inconsistent = UncertainValue.from_json(json.dumps(payload))
    with pytest.raises(ValueError, match="Conflicting"):
        op(x, inconsistent)
    registry = UnitRegistry()
    with pytest.raises(ValueError, match="Conflicting"):
        op(Quantity(x, registry.parse("m")), Quantity(inconsistent, registry.parse("m")))


def test_indexed_source_conflict_with_parent():
    x = UncertainValue(np.array([1., 2.]), .1)
    payload = json.loads(x[0].to_json())
    payload["sources"][0]["uncertainty"] = .9
    loaded = UncertainValue.from_json(json.dumps(payload))
    with pytest.raises(ValueError, match="Conflicting"):
        x + loaded


@pytest.mark.parametrize("value", [math.pi, .6000000000000001, -0., 1e-300, 1e300])
def test_float_round_trip(value):
    x = UncertainValue(value, .1)
    assert restored(x).value.hex() == value.hex()


@pytest.mark.parametrize("change", [
    lambda p: p.update(version=999),
    lambda p: p.update(extra=True),
    lambda p: p.update(value=None),
    lambda p: p["sources"][0].update(id="invalid"),
    lambda p: p["sources"][0].update(uncertainty=-1.),
    lambda p: p["sources"].append(p["sources"][0].copy()),
    lambda p: p["derivatives"][0].update(source=10),
    lambda p: p["derivatives"].append(p["derivatives"][0].copy()),
    lambda p: p.update(derivatives=[]),
    lambda p: p["derivatives"][0].update(coefficient=2.),
])
def test_malformed_payload_rejected(change):
    payload = json.loads(UncertainValue(3., .2).to_json())
    change(payload)
    with pytest.raises(ValueError):
        UncertainValue.from_json(json.dumps(payload))


@pytest.mark.parametrize("payload", ["not json", "{}", '{"version": 1, "version": 1}', '[1, 2]', 'null'])
def test_invalid_json(payload):
    with pytest.raises(ValueError):
        UncertainValue.from_json(payload)


@pytest.mark.parametrize("value,uncertainty", [(math.inf, .1), (math.nan, .1), (1., math.inf), (1., -.1)])
def test_non_json_numbers_rejected(value, uncertainty):
    with pytest.raises(ValueError):
        UncertainValue(value, uncertainty).to_json()


def test_v1_fixture_and_deterministic_encoding():
    payload = '{"version":1,"value":3.0,"sources":[{"id":"01900000-0000-7000-8000-000000000001","index":null,"uncertainty":0.2}],"derivatives":[{"source":0,"coefficient":1.0}],"variable_id":"01900000-0000-7000-8000-000000000001"}'
    loaded = UncertainValue.from_json(payload)
    assert loaded.to_json() == payload
    assert restored(loaded).to_json() == payload

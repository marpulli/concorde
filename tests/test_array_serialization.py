import json
import math
import subprocess
import sys

import numpy as np
import pytest

from concorde import UncertainValue


def restored(value):
    return UncertainValue.from_json(value.to_json())


def array():
    return UncertainValue(np.array([1., 2., 4.]), np.array([.1, .2, .3]))


@pytest.mark.parametrize("values,uncertainty", [([], []), ([1.], [.1]), ([1., 2., 4.], [.1, .2, .3]), ([1., 2., 4.], .1)])
def test_array_round_trip(values, uncertainty):
    uncertainty = np.array(uncertainty) if isinstance(uncertainty, list) else uncertainty
    x = UncertainValue(np.array(values), uncertainty)
    loaded = restored(x)
    np.testing.assert_array_equal(loaded.value, x.value)
    np.testing.assert_allclose(loaded.uncertainty, x.uncertainty)
    np.testing.assert_allclose((loaded - x).uncertainty, np.zeros(len(values)), atol=1e-12)
    assert loaded.is_independent()
    assert loaded.variable_id == x.variable_id
    assert loaded.to_json() == x.to_json()


def test_separate_payloads_and_equal_but_independent_arrays():
    x, independent = array(), array()
    loaded_x, loaded_y = restored(x), restored(2*x)
    np.testing.assert_allclose((loaded_y - 2*loaded_x).uncertainty, 0., atol=1e-12)
    np.testing.assert_allclose((loaded_x - restored(independent)).uncertainty, np.sqrt(2)*x.uncertainty)
    assert not loaded_y.is_independent()
    assert (loaded_x[0] - loaded_x[1]).uncertainty == pytest.approx(math.hypot(.1, .2))


@pytest.mark.parametrize("selection", [slice(None, None, -1), slice(None, None, 2), slice(1, 2), slice(0, 0)])
def test_slices_keep_original_source_coordinates(selection):
    x = array()
    selected, parent = restored(x[selection]), restored(x)
    np.testing.assert_allclose((selected - parent[selection]).uncertainty, 0., atol=1e-12)
    assert selected.to_json() == x[selection].to_json()
    assert not selected.is_independent()


def test_independently_loaded_scalar_selection_matches_loaded_array():
    x = array()
    parent, selected = restored(x), restored(x[1])
    assert (parent[1] - selected).uncertainty == pytest.approx(0.)
    np.testing.assert_allclose((parent - selected).uncertainty, [math.hypot(.1, .2), 0., math.hypot(.3, .2)])


def test_shared_scalar_source_across_array_elements():
    x, offset = array(), UncertainValue(5., .4)
    shifted = restored(x + offset)
    assert (shifted[0] - shifted[1]).uncertainty == pytest.approx(math.hypot(.1, .2))
    assert (shifted[0] + shifted[1]).uncertainty == pytest.approx(math.sqrt(.1**2 + .2**2 + .8**2))
    np.testing.assert_allclose((shifted - restored(offset) - restored(x)).uncertainty, 0., atol=1e-12)


def test_repeated_source_and_nonlinear_dependencies():
    x = array()
    # Every row shares x[0], and also depends on its own original coordinate.
    computed = (x + x[0]) * x[::-1]
    loaded = restored(computed)
    np.testing.assert_allclose(loaded.value, computed.value)
    np.testing.assert_allclose(loaded.uncertainty, computed.uncertainty)
    np.testing.assert_allclose((loaded - computed).uncertainty, 0., atol=1e-12)
    assert loaded.to_json() == computed.to_json()


def test_length_one_broadcast_source_is_not_mistaken_for_distinct_coordinates():
    x = UncertainValue(np.array([2.]), .5)
    y = array()
    result = restored(x + y)
    np.testing.assert_allclose((result - restored(x) - restored(y)).uncertainty, 0., atol=1e-12)
    assert (result[0] - result[1]).uncertainty == pytest.approx(math.hypot(.1, .2))


def test_source_table_is_shared_and_derivative_rows_are_sparse():
    x = UncertainValue(np.arange(100, dtype=float), .1)
    shifted = x + UncertainValue(1., .3)
    payload = json.loads(shifted.to_json())
    assert len(payload["sources"]) == 2
    assert len(payload["derivatives"]) == 100
    assert all(len(row) == 2 for row in payload["derivatives"])
    source = next(source for source in payload["sources"] if "indices" in source)
    assert source["indices"] == list(range(100))
    assert len(source["uncertainty"]) == 100


@pytest.mark.parametrize("change", [
    lambda p: p["derivatives"].pop(),
    lambda p: p.update(derivatives=p["derivatives"][0]),
    lambda p: p["derivatives"][0][0].update(coordinate=999),
    lambda p: p["derivatives"][0][0].update(coordinate=None),
    lambda p: p["derivatives"][0].append(p["derivatives"][0][0].copy()),
    lambda p: p["sources"][0]["uncertainty"].pop(),
    lambda p: p["sources"][0]["indices"].__setitem__(1, 0),
    lambda p: p["sources"][0].update(index=0),
    lambda p: p["sources"][0].pop("indices"),
    lambda p: p["sources"][0]["uncertainty"].__setitem__(0, -1.),
    lambda p: p["derivatives"][0][0].update(coefficient=2.),
])
def test_invalid_array_shapes_and_references(change):
    payload = json.loads(array().to_json())
    change(payload)
    with pytest.raises(ValueError):
        UncertainValue.from_json(json.dumps(payload))


@pytest.mark.parametrize("values,uncertainty", [([float("nan")], [.1]), ([1.], [float("inf")])])
def test_nonfinite_array_numbers_rejected(values, uncertainty):
    with pytest.raises(ValueError):
        UncertainValue(np.array(values), np.array(uncertainty)).to_json()


def test_arrays_reconnect_in_fresh_process():
    x = array()
    code = """
import sys
import numpy as np
from concorde import UncertainValue
parent = UncertainValue.from_json(sys.stdin.readline())
selected = UncertainValue.from_json(sys.stdin.readline())
np.testing.assert_allclose((selected - parent[::-1]).uncertainty, 0.)
print(selected.to_json())
"""
    result = subprocess.run([sys.executable, "-c", code], input=x.to_json()+"\n"+x[::-1].to_json()+"\n", text=True, capture_output=True, check=True)
    np.testing.assert_allclose((UncertainValue.from_json(result.stdout) - x[::-1]).uncertainty, 0.)


def test_fixed_v1_array_fixture():
    payload = {"version": 1, "value": [1., 2.], "sources": [{"id": "01900000-0000-7000-8000-000000000001", "index": None, "uncertainty": [.1, .2], "indices": [0, 1]}], "derivatives": [[{"source": 0, "coefficient": 1., "coordinate": 0}], [{"source": 0, "coefficient": 1., "coordinate": 1}]], "variable_id": "01900000-0000-7000-8000-000000000001"}
    loaded = UncertainValue.from_json(json.dumps(payload))
    assert json.loads(loaded.to_json()) == payload
    np.testing.assert_allclose(loaded.uncertainty, [.1, .2])

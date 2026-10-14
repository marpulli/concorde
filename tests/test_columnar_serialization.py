import copy
import json
import numpy as np
import pytest
from concorde import UncertainValue


def payload():
    return json.loads(UncertainValue(np.array([1., 2.]), np.array([.1, .2])).to_json())


@pytest.mark.parametrize('change', [
    lambda p: p['derivatives'][0]['outputs'].pop(),
    lambda p: p['derivatives'][0]['coefficients'].pop(),
    lambda p: p['derivatives'][0]['coordinates'].pop(),
    lambda p: p['derivatives'][0].update(source=99),
    lambda p: p['derivatives'][0]['outputs'].__setitem__(0, 99),
    lambda p: p['derivatives'][0]['coordinates'].__setitem__(0, 99),
    lambda p: p['derivatives'].append(copy.deepcopy(p['derivatives'][0])),
    lambda p: p['derivatives'][0].pop('coordinates'),
    lambda p: p['derivatives'][0].update(extra=True),
    lambda p: p.update(version=99),
])
def test_malformed_columns(change):
    p = payload()
    change(p)
    with pytest.raises(ValueError):
        UncertainValue.from_json(json.dumps(p))


def test_duplicate_output_and_source_coordinate():
    p = payload()
    p['variable_id'] = None
    p['derivatives'][0]['outputs'] = [0, 0]
    p['derivatives'][0]['coordinates'] = [0, 0]
    with pytest.raises(ValueError, match='Duplicate derivative source'):
        UncertainValue.from_json(json.dumps(p))


def test_repeated_output_with_different_coordinates_is_valid():
    x = UncertainValue(np.array([1., 2.]), .1)
    y = x + x[::-1]
    p = json.loads(y.to_json())
    assert p['derivatives'][0]['outputs'] == [0, 0, 1, 1]
    loaded = UncertainValue.from_json(json.dumps(p))
    np.testing.assert_allclose((loaded - y).uncertainty, 0.)
    assert (loaded[0] - loaded[1]).uncertainty == pytest.approx(0.)


def test_scalar_source_does_not_need_coordinate_vector():
    x = UncertainValue(3., .1)
    p = json.loads((2*x).to_json())
    assert 'coordinates' not in p['derivatives'][0]
    assert p['derivatives'][0]['outputs'] == [0]
    assert (UncertainValue.from_json(json.dumps(p)) - 2*x).uncertainty == pytest.approx(0.)


def test_separate_scalar_selection_payload_reconnects():
    x = UncertainValue(np.array([1., 2.]), np.array([.1, .2]))
    a = UncertainValue.from_json(x.to_json())
    b = UncertainValue.from_json((x[1]**2).to_json())
    assert (a[1]**2 - b).uncertainty == pytest.approx(0.)

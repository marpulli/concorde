import numpy as np
import pytest

from concorde import Quantity, UncertainValue, UnitRegistry


@pytest.mark.parametrize("values", [
    [3.], [3., 1.], [3., 1., 2.], [4., 1., 3., 2.],
    [2., 2., 2.], [2., 2., 2., 4.], [-0., 0., 0.],
])
def test_median_uses_stable_nominal_order_and_retains_sources(values):
    x = UncertainValue(np.array(values), np.arange(1., len(values)+1)/10)
    order = sorted(range(len(values)), key=lambda i: values[i])
    middle = len(values)//2
    expected = x[order[middle]]
    if len(values) % 2 == 0:
        expected = (expected+x[order[middle-1]])/2
    result = x.median()
    assert result.value == pytest.approx(np.median(values))
    assert (result-expected).uncertainty == pytest.approx(0.)
    restored = UncertainValue.from_json(result.to_json())
    assert (restored-expected).uncertainty == pytest.approx(0.)
    np.testing.assert_array_equal(x.value, values)


def test_shared_sources_reordering_filtering_and_assignment():
    x = UncertainValue(np.array([3., 1., 4., 2.]), .2)
    shared = UncertainValue(5., .4)
    assert ((x+shared).median()-x.median()-shared).uncertainty == pytest.approx(0.)
    assert (x[::-1].median()-x.median()).uncertainty == pytest.approx(0.)
    assert ((x*-2).median()+2*x.median()).uncertainty == pytest.approx(0.)
    selected = x[[True, False, False, True]]
    assert (selected.median()-(x[0]+x[3])/2).uncertainty == pytest.approx(0.)
    x[[True, True, True, True]] = shared
    assert (x.median()-shared).uncertainty == 0.


def test_midpoint_avoids_overflow_and_underflow():
    for value in [1e308, np.nextafter(0., 1.)]:
        x = UncertainValue(np.array([value, value]), .1)
        assert x.median().value == value
        assert x.median().uncertainty == pytest.approx(.1/np.sqrt(2))


def test_scalar_and_invalid_inputs():
    x = UncertainValue(3., .2)
    assert (x.median()-x).uncertainty == 0.
    for values in [np.array([]), np.array([1., np.nan]), np.array([np.inf]), float("nan")]:
        with pytest.raises(ValueError):
            UncertainValue(values, .2).median()


@pytest.mark.parametrize("unit,target", [("m", "cm"), ("degC", "degF"), ("delta_degC", "delta_degF")])
@pytest.mark.parametrize("values", [[10., 20., 40.], [10., 20., 40., 80.]])
def test_quantity_median_commutes_with_conversion(unit, target, values):
    registry = UnitRegistry()
    q = Quantity(UncertainValue(np.array(values), .5), registry.parse(unit))
    result = q.median()
    assert result.unit == q.unit
    difference = q.to(registry.parse(target)).median()-result
    assert difference.value.value == pytest.approx(0., abs=1e-12)
    assert difference.value.uncertainty == pytest.approx(0., abs=1e-12)
    restored = Quantity.from_json(result.to_json(), registry)
    assert (restored-result).value.uncertainty == 0.

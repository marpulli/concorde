import numpy as np
import pytest

from concorde import Quantity, UncertainValue, UnitRegistry


@pytest.mark.parametrize("selection", [slice(None), slice(None, None, -1), slice(None, None, 2), [True, False, True, False]])
def test_mean_preserves_dependencies(selection):
    x = UncertainValue(np.array([1., 2., 4., 8.]), np.array([.1, .2, .3, .4]))
    shared = UncertainValue(2., .5)
    selected = (x+shared)[selection]
    result = selected.mean()
    assert result.value == pytest.approx(np.mean(selected.value))
    assert (result-selected.sum()/len(selected.value)).uncertainty == pytest.approx(0., abs=1e-12)
    loaded = UncertainValue.from_json(result.to_json())
    assert (loaded-result).uncertainty == 0.


def test_shared_uncertainty_is_not_reduced_by_averaging():
    x = UncertainValue(np.array([1., 2., 3.]), .3)
    shared = UncertainValue(5., .4)
    mean = (x+shared).mean()
    assert mean.uncertainty == pytest.approx(np.sqrt(.3**2/3+.4**2))
    assert (mean-x.mean()-shared).uncertainty == pytest.approx(0., abs=1e-12)
    x[[True, True, True]] = shared
    assert (x.mean()-shared).uncertainty == pytest.approx(0., abs=1e-12)


def test_singleton_scalar_and_empty():
    x = UncertainValue(3., .2)
    assert (x.mean()-x).uncertainty == 0.
    array = UncertainValue(np.array([3.]), .2)
    assert (array.mean()-array[0]).uncertainty == 0.
    with pytest.raises(ValueError, match="at least one"):
        array[:0].mean()


def test_mean_avoids_overflowing_an_intermediate_sum():
    x = UncertainValue(np.array([1e308, 1e308]), .1)
    assert x.mean().value == pytest.approx(1e308)


@pytest.mark.parametrize("unit,target", [("m", "cm"), ("degC", "degF"), ("delta_degC", "delta_degF")])
def test_quantity_mean_commutes_with_conversion(unit, target):
    registry = UnitRegistry()
    q = Quantity(UncertainValue(np.array([10., 20., 30.]), .5), registry.parse(unit))
    result = q.mean()
    assert result.unit == q.unit
    delta = q.to(registry.parse(target)).mean()-result
    assert delta.value.value == pytest.approx(0., abs=1e-12)
    assert delta.value.uncertainty == pytest.approx(0., abs=1e-12)
    with pytest.raises(ValueError):
        q[:0].mean()

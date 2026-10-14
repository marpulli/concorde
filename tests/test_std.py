import numpy as np
import pytest
from uncertainties import ufloat, unumpy

from concorde import Quantity, UncertainValue, UnitRegistry


def reference_std(values, ddof):
    mean = sum(values) / len(values)
    return (sum((v-mean)**2 for v in values) / (len(values)-ddof))**.5


@pytest.mark.parametrize("ddof", [-1., 0., 1., 1.5])
@pytest.mark.parametrize("selection", [slice(None), slice(None, None, -1), slice(None, None, 2)])
def test_std_agrees_with_uncertainties(ddof, selection):
    values = np.array([2., 4., 6., 9.])
    sigmas = np.array([.1, .2, .3, .4])
    x = UncertainValue(values, sigmas)
    shared = UncertainValue(3., .5)
    reference = unumpy.uarray(values, sigmas)
    for actual, expected in [
        ((x+shared)[selection], (reference+ufloat(3., .5))[selection]),
        ((x*x[::-1])[selection], (reference*reference[::-1])[selection]),
    ]:
        result = actual.std(ddof=ddof)
        expected = reference_std(expected, ddof)
        assert result.value == pytest.approx(expected.nominal_value)
        assert result.uncertainty == pytest.approx(expected.std_dev)
        restored = UncertainValue.from_json(result.to_json())
        assert (restored-result).uncertainty == 0.


def test_shared_translation_and_scaling_correlations():
    x = UncertainValue(np.array([1., 2., 5., 10.]), .2)
    shared = UncertainValue(3., 1e12)
    assert ((x+shared).std()-x.std()).uncertainty == pytest.approx(0., abs=1e-12)
    assert ((x-x[0]).std()-x.std()).uncertainty == pytest.approx(0., abs=1e-12)
    assert ((x*2).std()-2*x.std()).uncertainty == pytest.approx(0., abs=1e-12)
    assert ((x*-2).std()-2*x.std()).uncertainty == pytest.approx(0., abs=1e-12)
    assert (x[::-1].std()-x.std()).uncertainty == pytest.approx(0., abs=1e-12)


def test_zero_spread_distinguishes_independent_and_shared_uncertainty():
    x = UncertainValue(np.ones(3), .2)
    with pytest.raises(ValueError, match="undefined at zero spread"):
        x.std()
    tiny = UncertainValue(np.ones(3), 1e-300)
    with pytest.raises(ValueError, match="undefined at zero spread"):
        tiny.std()
    shared = UncertainValue(1., .2)
    x[[True, True, True]] = shared
    assert x.std().value == 0.
    assert x.std().uncertainty == 0.
    assert UncertainValue(np.ones(3), 0.).std().uncertainty == 0.
    assert x[:1].std().uncertainty == 0.
    assert shared.std().uncertainty == 0.


@pytest.mark.parametrize("values", [np.array([]), np.array([1., np.nan]), np.array([1., np.inf])])
def test_invalid_values(values):
    with pytest.raises(ValueError):
        UncertainValue(values, .1).std()


@pytest.mark.parametrize("ddof", [3., 4., np.nan, np.inf, -np.inf])
def test_invalid_ddof(ddof):
    with pytest.raises(ValueError):
        UncertainValue(np.array([1., 2., 4.]), .1).std(ddof)


def test_stability_for_large_offsets_and_extreme_magnitudes():
    values = np.array([1., 2., 4.])
    assert UncertainValue(values+1e12, .1).std().value == pytest.approx(values.std(), rel=1e-14)
    assert UncertainValue(np.array([-1e308, 1e308]), .1).std().value == pytest.approx(1e308)
    assert UncertainValue(np.array([1e-300, 2e-300]), .1).std().value == pytest.approx(5e-301, abs=0., rel=1e-14)


@pytest.mark.parametrize("unit,target", [("m", "cm"), ("degC", "degF"), ("delta_degC", "delta_degF")])
def test_quantity_std_units(unit, target):
    registry = UnitRegistry()
    q = Quantity(UncertainValue(np.array([10., 20., 40.]), .5), registry.parse(unit))
    result = q.std()
    assert result.unit.is_delta == (unit == "degC" or unit == "delta_degC")
    assert not result.unit.is_affine
    difference = q.to(registry.parse(target)).std()-result
    assert difference.value.value == pytest.approx(0., abs=1e-12)
    assert difference.value.uncertainty == pytest.approx(0., abs=1e-12)
    assert result.value.value == pytest.approx(np.std(q.value.value))

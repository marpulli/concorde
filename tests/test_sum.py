import numpy as np
import pytest
from uncertainties import unumpy

from concorde import AffineUnitError, Quantity, UncertainValue, UnitRegistry


@pytest.mark.parametrize("selection", [slice(None), slice(None, None, -1), slice(None, None, 2), [True, False, True, False]])
def test_sum_retains_original_sources(selection):
    x = UncertainValue(np.array([1., 2., 3., 4.]), np.array([.1, .2, .3, .4]))
    selected = x[selection]
    expected = sum((selected[i] for i in range(len(selected.value))), UncertainValue(0., 0.))
    result = selected.sum()
    assert result.value == pytest.approx(expected.value)
    assert (result-expected).uncertainty == pytest.approx(0., abs=1e-12)
    loaded = UncertainValue.from_json(result.to_json())
    assert (loaded-result).uncertainty == 0.


def test_compact_and_reordered_sums_agree_with_uncertainties():
    values = np.array([2., 4., 6.])
    sigmas = np.array([.1, .2, .3])
    x = UncertainValue(values, sigmas)
    reference = unumpy.uarray(values, sigmas)
    for actual, expected in [
        (x.sum(), sum(reference)),
        ((x-x[0]).sum(), sum(reference-reference[0])),
        ((x+x[::-1]).sum(), sum(reference+reference[::-1])),
        ((x**2).sum(), sum(reference**2)),
    ]:
        assert actual.value == pytest.approx(expected.nominal_value)
        assert actual.uncertainty == pytest.approx(expected.std_dev)
    assert (x.sum()-x[::-1].sum()).uncertainty == 0.
    assert ((x+x[::-1]).sum()-2*x.sum()).uncertainty == 0.


def test_shared_broadcast_and_assigned_sources():
    x = UncertainValue(np.array([1., 2., 3.]), .2)
    s = UncertainValue(10., .5)
    result = (x+s).sum()
    assert (result-x.sum()-3*s).uncertainty == pytest.approx(0.)
    assert ((x+x[:1]).sum()-x.sum()-3*x[0]).uncertainty == pytest.approx(0.)
    x[[True, False, True]] = s
    assert (x.sum()-2*s-x[1]).uncertainty == pytest.approx(0.)


def test_sum_correlates_when_broadcast_back_over_parent():
    x = UncertainValue(np.array([1., 2., 3.]), .2)
    result = x-x.sum()
    np.testing.assert_allclose(result.uncertainty, np.sqrt(2)*.2)


@pytest.mark.parametrize("values", [np.array([]), np.array([1.]), np.arange(1000.)])
def test_sum_shapes(values):
    x = UncertainValue(values, .2)
    assert isinstance(x.sum().value, float)
    assert x.sum().value == pytest.approx(values.sum())
    assert x.sum().uncertainty == pytest.approx(np.sqrt(len(values))*.2)
    assert (x.sum()-x.sum()).uncertainty == 0.


def test_scalar_and_units():
    x = UncertainValue(2., .1)
    assert (x.sum()-x).uncertainty == 0.
    registry = UnitRegistry()
    q = Quantity(UncertainValue(np.array([1., 2.]), .1), registry.parse("m"))
    assert q.sum().unit == q.unit
    assert (q.to(registry.parse("cm")).sum()-q.sum()).value.uncertainty == pytest.approx(0.)
    for unit in ["degC", "degF"]:
        with pytest.raises(AffineUnitError):
            Quantity(q.value, registry.parse(unit)).sum()
    delta = Quantity(q.value, registry.parse("delta_degC"))
    assert delta.sum().unit.is_delta

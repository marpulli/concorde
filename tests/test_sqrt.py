import numpy as np
import pytest
from concorde import AffineUnitError, Quantity, UncertainValue, UnitRegistry


def test_sqrt_sources_and_zero():
    x = UncertainValue(np.array([0., 4., 9.]), np.array([0., .2, .3]))
    result = x.sqrt()
    np.testing.assert_allclose(result.value, [0., 2., 3.])
    np.testing.assert_allclose(result.uncertainty, [0., .05, .05])
    np.testing.assert_allclose((result**2-x).uncertainty, 0., atol=1e-12)
    np.testing.assert_allclose((result[::-1]-x[::-1].sqrt()).uncertainty, 0.)
    assert x[:0].sqrt().value.shape == (0,)


@pytest.mark.parametrize("value,sigma", [(-1., 0.), (0., .1), (float("nan"), 0.)])
def test_invalid_square_roots(value, sigma):
    with pytest.raises(ValueError):
        UncertainValue(value, sigma).sqrt()


def test_quantity_sqrt():
    registry = UnitRegistry()
    q = Quantity(UncertainValue(4., .2), registry.parse("m^2"))
    result = q.sqrt()
    assert result.unit == registry.parse("m")
    assert (result**2-q).value.uncertainty == pytest.approx(0.)
    assert UncertainValue(0., 0.).sqrt().uncertainty == 0.
    with pytest.raises(AffineUnitError):
        Quantity(q.value, registry.parse("degC")).sqrt()

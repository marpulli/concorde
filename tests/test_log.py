import numpy as np
import pytest
from concorde import IncompatibleUnitError, Quantity, UncertainValue, UnitRegistry


def test_log_sources_and_shape():
    x = UncertainValue(np.array([1., 2., 4.]), .2)
    result = x.log()
    np.testing.assert_allclose(result.value, np.log(x.value))
    np.testing.assert_allclose(result.uncertainty, .2/x.value)
    np.testing.assert_allclose((result[::-1]-x[::-1].log()).uncertainty, 0.)
    assert (result[0]-x[0].log()).uncertainty == 0.
    assert x[:0].log().value.shape == (0,)
    assert (UncertainValue.from_json(result.to_json())-result)[0].uncertainty == 0.


@pytest.mark.parametrize("value", [0., -1., float("nan")])
def test_log_domain(value):
    with pytest.raises(ValueError):
        UncertainValue(value, .1).log()


def test_quantity_log_normalizes_scaled_dimensionless_units():
    registry = UnitRegistry()
    q = Quantity(UncertainValue(50., 1.), registry.parse("%"))
    result = q.log()
    assert result.unit == registry.parse("1")
    assert result.value.value == pytest.approx(np.log(.5))
    assert result.value.uncertainty == pytest.approx(.02)
    with pytest.raises(IncompatibleUnitError):
        Quantity(q.value, registry.parse("m")).log()

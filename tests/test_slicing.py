import operator

import numpy as np
import pytest

from concorde import Quantity, UncertainValue, UnitRegistry


@pytest.mark.parametrize("selection", [slice(None), slice(None, None, -1), slice(1, 4, 2), slice(-3, None), slice(9, 12), slice(4, 0, -2), slice(-100, 100)])
def test_slices(selection):
    values = np.arange(1.0, 6.0)
    uncertainties = values / 10
    x = UncertainValue(values, uncertainties)
    selected = x[selection]
    np.testing.assert_array_equal(selected.value, values[selection])
    np.testing.assert_allclose(selected.uncertainty, uncertainties[selection])
    for i, source in enumerate(np.arange(5)[selection]):
        assert (selected[i] - x[source]).uncertainty == pytest.approx(0.0)
    np.testing.assert_array_equal(x.value, values)


@pytest.mark.parametrize("op", [operator.add, operator.sub, operator.mul, operator.truediv])
def test_reordered_arithmetic(op):
    x = UncertainValue(np.array([2., 4., 8.]), np.array([.1, .2, .3]))
    result = op(x, x[::-1])
    for i in range(3):
        expected = op(x[i], x[2 - i])
        assert result[i].value == pytest.approx(expected.value)
        assert (result[i] - expected).uncertainty == pytest.approx(0., abs=1e-12)
    assert ((x[::-1] - x)[1]).uncertainty == pytest.approx(0.)


def test_sliced_scalar_sources_and_unary_operations():
    x = UncertainValue(np.array([-2., 4., 8.]), .2)
    s = UncertainValue(1., .3)
    sliced = (x + s)[::-1]
    np.testing.assert_allclose((sliced[::-1] - x - s).uncertainty, 0., atol=1e-12)
    for i in range(3):
        expected = x[2 - i] + s
        assert ((sliced**2)[i] - expected**2).uncertainty == pytest.approx(0.)
        assert ((-sliced)[i] + expected).uncertainty == pytest.approx(0.)
        assert ((sliced * 2 + 3)[i] - (expected * 2 + 3)).uncertainty == pytest.approx(0.)
    registry = UnitRegistry()
    q = Quantity(x, registry.parse("m"))
    np.testing.assert_array_equal(abs(q[::-1]).value.value, [8., 4., 2.])
    np.testing.assert_allclose((q[::-1].to(registry.parse("cm"))[::-1] - q).value.uncertainty, 0.)


def test_empty_slices_and_broadcasting():
    x = UncertainValue(np.array([2., 4.]), .2)
    empty = x[:0]
    for result in [empty, empty + x[:1], x[:1] + empty, empty * 2, empty**2]:
        assert result.value.shape == (0,)
        assert result.uncertainty.shape == (0,)
    result = x + x[:1]
    assert (result[1] - x[1] - x[0]).uncertainty == pytest.approx(0.)
    with pytest.raises(ValueError):
        x[::0]
    with pytest.raises(TypeError):
        UncertainValue(1., .1)[:]

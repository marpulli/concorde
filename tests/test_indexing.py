import operator

import numpy as np
import pytest

from concorde import Quantity, UncertainValue, UnitRegistry


def array(values=(2.0, 4.0, 8.0), uncertainty=(0.1, 0.2, 0.3)):
    return UncertainValue(np.array(values, dtype=np.float64), np.array(uncertainty, dtype=np.float64))


@pytest.mark.parametrize("index,expected,uncertainty", [(0, 2.0, 0.1), (-1, 8.0, 0.3), (-3, 2.0, 0.1), (np.int64(1), 4.0, 0.2)])
def test_integer_indexing(index, expected, uncertainty):
    value = array()
    selected = value[index]
    assert isinstance(selected.value, float)
    assert selected.value == expected
    assert selected.uncertainty == pytest.approx(uncertainty)
    np.testing.assert_array_equal(value.value, [2.0, 4.0, 8.0])


@pytest.mark.parametrize("index", [3, -4, 100])
def test_out_of_bounds(index):
    with pytest.raises(IndexError):
        array()[index]


def test_empty_and_scalar():
    with pytest.raises(IndexError):
        array((), ())[0]
    with pytest.raises(TypeError):
        UncertainValue(1.0, 0.1)[0]


@pytest.mark.parametrize("index", [1.5, "1", slice(None), [0], np.array([True, False, True]), None, (0,)])
def test_only_integer_indexing(index):
    with pytest.raises(TypeError):
        array()[index]


def test_index_protocol():
    class Index:
        def __index__(self):
            return 1

    assert array()[Index()].value == 4.0


def test_same_element_retains_correlation():
    x = array()
    assert (x[0] - x[-3]).uncertainty == pytest.approx(0.0)
    assert ((x * 2)[1] - 2 * x[1]).uncertainty == pytest.approx(0.0)
    assert (x[0] + x[0]).uncertainty == pytest.approx(0.2)


def test_different_elements_are_independent():
    x = array()
    assert (x[0] - x[1]).uncertainty == pytest.approx(np.hypot(0.1, 0.2))
    assert (x[1] - x[0]).uncertainty == pytest.approx(np.hypot(0.1, 0.2))
    assert (x[0] - array()[0]).uncertainty == pytest.approx(np.sqrt(2) * 0.1)


def test_broadcast_scalar_retains_shared_source():
    x = array()
    offset = UncertainValue(10.0, 5.0)
    shifted = x + offset
    assert (shifted[0] - shifted[1]).uncertainty == pytest.approx(np.hypot(0.1, 0.2))
    assert (shifted[0] - offset - x[0]).uncertainty == pytest.approx(0.0)


def test_selected_element_broadcast_back_to_original():
    x = array()
    difference = x - x[0]
    np.testing.assert_allclose(difference.uncertainty, [0.0, np.hypot(0.2, 0.1), np.hypot(0.3, 0.1)])
    assert (difference[1] - (x[1] - x[0])).uncertainty == pytest.approx(0.0)
    np.testing.assert_allclose((difference + x[0] - x).uncertainty, 0.0, atol=1e-12)


@pytest.mark.parametrize("op", [operator.add, operator.sub, operator.mul, operator.truediv])
@pytest.mark.parametrize("index", [0, 1, 2])
def test_indexing_commutes_with_arithmetic(op, index):
    x = array()
    y = array((3.0, 5.0, 9.0), (0.4, 0.5, 0.6))
    for left, right in [(x, y), (x, x), (x, x - x[0] + 1.0)]:
        selected = op(left, right)[index]
        scalar = op(left[index], right[index])
        assert selected.value == pytest.approx(scalar.value)
        assert selected.uncertainty == pytest.approx(scalar.uncertainty)
        assert (selected - scalar).uncertainty == pytest.approx(0.0, abs=1e-12)
    for scalar in (x[0], x[1], y[2]):
        selected = op(x, scalar)[index]
        expected = op(x[index], scalar)
        assert selected.uncertainty == pytest.approx(expected.uncertainty)
        assert (selected - expected).uncertainty == pytest.approx(0.0, abs=1e-12)
        selected = op(scalar, x)[index]
        expected = op(scalar, x[index])
        assert (selected - expected).uncertainty == pytest.approx(0.0, abs=1e-12)


def test_multiple_selected_sources_and_unary_operations():
    x = array()
    computed = (x - x[0]) * (x + x[1])
    for i in range(3):
        expected = (x[i] - x[0]) * (x[i] + x[1])
        assert (computed[i] - expected).uncertainty == pytest.approx(0.0, abs=1e-12)
        assert ((computed**2)[i] - expected**2).uncertainty == pytest.approx(0.0, abs=1e-12)
        assert ((-computed)[i] + expected).uncertainty == pytest.approx(0.0, abs=1e-12)


def test_length_one_broadcasting_uses_same_source_element():
    x = array((2.0,), (0.1,))
    y = array()
    result = x + y
    assert (result[2] - y[2] - x[0]).uncertainty == pytest.approx(0.0)
    np.testing.assert_allclose((result - x[0] - y).uncertainty, 0.0, atol=1e-12)
    assert (result[0] - result[2]).uncertainty == pytest.approx(np.hypot(0.1, 0.3))


def test_quantity_indexing_retains_unit_and_conversion_correlations():
    registry = UnitRegistry()
    quantity = Quantity(array(), registry.parse("degC"))
    selected = quantity[-1]
    assert selected.unit == quantity.unit
    assert selected.value.value == 8.0
    assert selected.value.uncertainty == pytest.approx(0.3)
    converted = quantity.to(registry.parse("K"))
    assert (converted[1] - quantity[1].to(converted.unit)).value.uncertainty == pytest.approx(0.0)
    with pytest.raises(IndexError):
        quantity[3]
    with pytest.raises(TypeError):
        selected[0]
    with pytest.raises(TypeError):
        quantity[:]

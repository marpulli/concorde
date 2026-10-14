import numpy as np
import pytest

from concorde import Quantity, UncertainValue, UnitRegistry


@pytest.mark.parametrize("mask", [
    [True, False, True], (True, False, True), np.array([True, False, True]),
    np.array([True, False, False, False, True, False])[::2],
    [False, False, False], [True, True, True],
])
def test_boolean_selection(mask):
    x = UncertainValue(np.array([1., 2., 3.]), np.array([.1, .2, .3]))
    shared = UncertainValue(2., .5)
    y = x + shared
    selected = y[mask]
    indices = np.flatnonzero(mask)
    np.testing.assert_array_equal(selected.value, y.value[np.asarray(mask)])
    np.testing.assert_allclose(selected.uncertainty, y.uncertainty[np.asarray(mask)])
    for i, j in enumerate(indices):
        assert (selected[i] - x[j] - shared).uncertainty == pytest.approx(0.)
    restored = UncertainValue.from_json(selected.to_json())
    np.testing.assert_allclose((restored-selected).uncertainty, 0.)
    np.testing.assert_array_equal(x.value, [1., 2., 3.])


def test_filtering_slices_and_quantities():
    registry = UnitRegistry()
    q = Quantity(UncertainValue(np.array([1., 2., 3.]), .1), registry.parse("m"))
    selected = q[::-1][[True, False, True]].to(registry.parse("cm"))
    assert (selected[0]-q[2]).value.uncertainty == pytest.approx(0.)
    assert (selected[1]-q[0]).value.uncertainty == pytest.approx(0.)
    assert selected.unit == registry.parse("cm")


@pytest.mark.parametrize("mask", [[1, 0, 1], [True, None, False], np.array([1, 0, 1]), np.array([[True, False, True]]), np.array([True, False, True], dtype=object)])
def test_invalid_mask_types(mask):
    x = UncertainValue(np.ones(3), .1)
    with pytest.raises(TypeError):
        x[mask]


@pytest.mark.parametrize("mask", [[], [True], np.array([True, False])])
def test_mask_length_must_match(mask):
    with pytest.raises(IndexError):
        UncertainValue(np.ones(3), .1)[mask]


def test_empty_arrays_and_scalar_rejection():
    x = UncertainValue(np.array([]), .1)
    for mask in [[], (), np.array([], dtype=bool)]:
        assert x[mask].value.shape == (0,)
        assert x[mask].uncertainty.shape == (0,)
    with pytest.raises(TypeError):
        UncertainValue(1., .1)[[True]]

import numpy as np
import pytest

from concorde import AffineUnitError, IncompatibleUnitError, Quantity, UncertainValue, UnitRegistry


def array():
    return UncertainValue(np.array([1., 2., 3.]), np.array([.1, .2, .3]))


def test_assignment_keeps_old_sources_and_existing_results_unchanged():
    x = array()
    old = x[:]
    derived = x * 2
    replacement = UncertainValue(10., .5)
    x[[True, False, True]] = replacement
    np.testing.assert_array_equal(x.value, [10., 2., 10.])
    np.testing.assert_allclose(x.uncertainty, [.5, .2, .5])
    assert (x[0]-x[2]).uncertainty == 0.
    assert (x[0]-replacement).uncertainty == 0.
    assert (x[1]-old[1]).uncertainty == 0.
    np.testing.assert_allclose((derived-2*old).uncertainty, 0.)
    np.testing.assert_array_equal(old.value, [1., 2., 3.])
    loaded = UncertainValue.from_json(x.to_json())
    np.testing.assert_allclose((loaded-x).uncertainty, 0.)


def test_assignment_from_overlapping_self_and_length_one_array():
    x = array()
    old = x[:]
    x[[True, True, True]] = x
    np.testing.assert_allclose((x-old).uncertainty, 0.)
    x[[True, True, True]] = x[::-1]
    np.testing.assert_allclose((x-old[::-1]).uncertainty, 0.)
    x[[True, False, True]] = x[1:2]
    assert (x[0]-old[1]).uncertainty == 0.
    assert (x[2]-old[1]).uncertainty == 0.


@pytest.mark.parametrize("replacement", [8., np.array([8.]), np.array([8., 8.])])
def test_exact_replacements_remove_old_uncertainty(replacement):
    x = array()
    x[np.array([True, False, True])] = replacement
    np.testing.assert_array_equal(x.value, [8., 2., 8.])
    np.testing.assert_allclose(x.uncertainty, [0., .2, 0.])


def test_quantities_convert_units_and_preserve_sources():
    registry = UnitRegistry()
    q = Quantity(array(), registry.parse("m"))
    rhs = Quantity(UncertainValue(250., 10.), registry.parse("cm"))
    q[[True, False, True]] = rhs
    np.testing.assert_array_equal(q.value.value, [2.5, 2., 2.5])
    assert (q[0]-rhs).value.uncertainty == pytest.approx(0.)
    old = q[:]
    q[[True, True, True]] = q
    np.testing.assert_allclose((q-old).value.uncertainty, 0.)


def test_affine_assignment_and_invalid_units_are_atomic():
    registry = UnitRegistry()
    q = Quantity(array(), registry.parse("degC"))
    rhs = Quantity(UncertainValue(68., .9), registry.parse("degF"))
    q[[True, False, False]] = rhs
    assert q[0].value.value == pytest.approx(20.)
    assert (q[0]-rhs).value.uncertainty == pytest.approx(0.)
    before = q.to_json()
    for unit, error in [("m", IncompatibleUnitError), ("delta_degC", AffineUnitError)]:
        with pytest.raises(error):
            q[[True, False, False]] = Quantity(array()[0], registry.parse(unit))
        assert q.to_json() == before


@pytest.mark.parametrize("mask, replacement, error", [
    ([True], 1., IndexError),
    ([1, 0, 1], 1., TypeError),
    (0, 1., TypeError),
    (slice(None), 1., TypeError),
    ([True, False, True], np.ones(3), ValueError),
    ([False, False, False], np.ones(2), ValueError),
    ([True, False, True], np.array([1, 2]), TypeError),
    ([True, False, True], np.ones((1, 2)), TypeError),
])
def test_invalid_assignments_do_not_mutate(mask, replacement, error):
    x = array()
    before = x.to_json()
    with pytest.raises(error):
        x[mask] = replacement
    assert x.to_json() == before


def test_empty_masks_and_scalar_rejection():
    x = array()
    old = x[:]
    x[[False, False, False]] = np.array([])
    np.testing.assert_allclose((x-old).uncertainty, 0.)
    empty = x[:0]
    empty[[]] = UncertainValue(2., .5)
    assert empty.value.shape == (0,)
    empty[np.array([], dtype=bool)] = np.array([])
    with pytest.raises(TypeError):
        UncertainValue(1., .1)[[True]] = 2.

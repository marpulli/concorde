import numpy as np
import pytest

from concorde import Quantity, UncertainValue, UnitRegistry


@pytest.mark.parametrize(
    "source,target,value,uncertainty,expected,expected_uncertainty",
    [
        ("cm", "m", 250.0, 2.0, 2.5, 0.02),
        ("g", "kg", 500.0, 5.0, 0.5, 0.005),
        ("kg", "kg", 2.0, 0.1, 2.0, 0.1),
        ("g / cm^3", "kg / m^3", 1.0, 0.01, 1000.0, 10.0),
        ("N", "kg * m / s^2", 2.0, 0.1, 2.0, 0.1),
        ("percent", "1", 50.0, 2.0, 0.5, 0.02),
        ("cm / m", "1", 50.0, 2.0, 0.5, 0.02),
        ("1", "1", 2.0, 0.1, 2.0, 0.1),
        ("degC", "K", 20.0, 0.5, 293.15, 0.5),
        ("degF", "K", 32.0, 1.8, 273.15, 1.0),
        ("delta_degF", "K", 18.0, 1.8, 10.0, 1.0),
        ("delta_degF / cm", "K / m", 18.0, 1.8, 1000.0, 100.0),
        ("cm^(1/2)", "m^(1/2)", 4.0, 0.2, 0.4, 0.02),
        ("mA * mmol / cd", "A * mol / cd", 2.0, 0.1, 2e-6, 1e-7),
    ],
)
def test_scalar_base_units(source, target, value, uncertainty, expected, expected_uncertainty):
    registry = UnitRegistry()
    original = Quantity(UncertainValue(value, uncertainty), registry.parse(source))
    converted = original.to_base_unit()
    assert converted.unit == registry.parse(target)
    assert converted.value.value == pytest.approx(expected)
    assert converted.value.uncertainty == pytest.approx(expected_uncertainty)
    assert original.value.value == value
    assert original.unit == registry.parse(source)

    repeated = converted.to_base_unit()
    assert repeated.unit == converted.unit
    assert repeated.value.value == pytest.approx(expected)
    assert repeated.value.uncertainty == pytest.approx(expected_uncertainty)

    round_trip = converted.to(original.unit)
    assert round_trip.value.value == pytest.approx(value)
    assert round_trip.value.uncertainty == pytest.approx(uncertainty)
    assert (round_trip - original).value.uncertainty == pytest.approx(0.0, abs=1e-12)


@pytest.mark.parametrize("values", [[], [0.0, 20.0, 100.0]])
def test_array_base_units_preserve_correlations(values):
    registry = UnitRegistry()
    values = np.array(values, dtype=np.float64)
    original = Quantity(UncertainValue(values, 0.5), registry.parse("degC"))
    converted = original.to_base_unit()
    assert converted.unit == registry.parse("K")
    np.testing.assert_allclose(converted.value.value, values + 273.15)
    np.testing.assert_allclose(converted.value.uncertainty, np.full_like(values, 0.5))
    np.testing.assert_array_equal(original.value.value, values)
    difference = converted.to(original.unit) - original
    np.testing.assert_allclose(difference.value.uncertainty, np.zeros_like(values), atol=1e-12)


def test_custom_units_without_defaults():
    registry = UnitRegistry(load_defaults=False)
    registry.define_unit("custom", [1, 1, -2, 0, 0, 0, 0], 4.0)
    quantity = Quantity(UncertainValue(3.0, 0.2), registry.parse("custom"))
    converted = quantity.to_base_unit()
    assert converted.unit == registry.parse("kg * m / s^2")
    assert converted.value.value == pytest.approx(12.0)
    assert converted.value.uncertainty == pytest.approx(0.8)

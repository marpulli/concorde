"""Tests for Quantity Python bindings."""

import pytest
from concorde import UncertainValue, UnitRegistry, Quantity, IncompatibleUnitError


class TestQuantityCreation:
    """Tests for creating Quantity instances."""

    def test_create_quantity(self):
        reg = UnitRegistry()
        kg = reg.parse("kg")
        val = UncertainValue(5.0, 0.1)
        q = Quantity(val, kg)
        assert q.value.value == 5.0
        assert q.unit.get_exponent("kg") == 1.0

    def test_value_property(self):
        reg = UnitRegistry()
        m = reg.parse("m")
        val = UncertainValue(10.0, 0.5)
        q = Quantity(val, m)
        assert q.value.value == 10.0

    def test_unit_property(self):
        reg = UnitRegistry()
        s = reg.parse("s")
        val = UncertainValue(3.0, 0.2)
        q = Quantity(val, s)
        assert q.unit.get_exponent("s") == 1.0


class TestQuantityArithmetic:
    """Tests for Quantity arithmetic."""

    def test_multiply_quantities(self):
        reg = UnitRegistry()
        kg = reg.parse("kg")
        m = reg.parse("m")

        q1 = Quantity(UncertainValue(5.0, 0.1), kg)
        q2 = Quantity(UncertainValue(3.0, 0.2), m)

        result = q1 * q2
        assert result.value.value == 15.0
        assert result.unit.get_exponent("kg") == 1.0
        assert result.unit.get_exponent("m") == 1.0

    def test_multiply_same_unit(self):
        reg = UnitRegistry()
        m = reg.parse("m")

        q1 = Quantity(UncertainValue(2.0, 0.1), m)
        q2 = Quantity(UncertainValue(3.0, 0.2), m)

        result = q1 * q2
        assert result.value.value == 6.0
        assert result.unit.get_exponent("m") == 2.0

    def test_add_same_unit(self):
        reg = UnitRegistry()
        kg = reg.parse("kg")

        q1 = Quantity(UncertainValue(5.0, 0.3), kg)
        q2 = Quantity(UncertainValue(3.0, 0.4), kg)

        result = q1 + q2
        assert result.value.value == 8.0
        assert result.unit.get_exponent("kg") == 1.0

    def test_add_uncertainty_propagation(self):
        reg = UnitRegistry()
        m = reg.parse("m")

        q1 = Quantity(UncertainValue(5.0, 0.3), m)
        q2 = Quantity(UncertainValue(3.0, 0.4), m)

        result = q1 + q2
        assert abs(result.value.uncertainty - 0.5) < 1e-10

    def test_add_incompatible_units_raises(self):
        reg = UnitRegistry()
        kg = reg.parse("kg")
        m = reg.parse("m")

        q1 = Quantity(UncertainValue(5.0, 0.1), kg)
        q2 = Quantity(UncertainValue(3.0, 0.2), m)

        with pytest.raises(IncompatibleUnitError):
            q1 + q2

    def test_sub_same_unit(self):
        reg = UnitRegistry()
        kg = reg.parse("kg")

        q1 = Quantity(UncertainValue(5.0, 0.3), kg)
        q2 = Quantity(UncertainValue(3.0, 0.4), kg)

        result = q1 - q2
        assert result.value.value == 2.0
        assert result.unit.get_exponent("kg") == 1.0

    def test_sub_uncertainty_propagation(self):
        reg = UnitRegistry()
        m = reg.parse("m")

        q1 = Quantity(UncertainValue(5.0, 0.3), m)
        q2 = Quantity(UncertainValue(3.0, 0.4), m)

        result = q1 - q2
        assert abs(result.value.uncertainty - 0.5) < 1e-10

    def test_sub_incompatible_units_raises(self):
        reg = UnitRegistry()
        kg = reg.parse("kg")
        m = reg.parse("m")

        q1 = Quantity(UncertainValue(5.0, 0.1), kg)
        q2 = Quantity(UncertainValue(3.0, 0.2), m)

        with pytest.raises(IncompatibleUnitError):
            q1 - q2

    def test_divide_quantities(self):
        reg = UnitRegistry()
        kg = reg.parse("kg")
        s = reg.parse("s")

        q1 = Quantity(UncertainValue(10.0, 0.1), kg)
        q2 = Quantity(UncertainValue(2.0, 0.2), s)

        result = q1 / q2
        assert result.value.value == 5.0
        assert result.unit.get_exponent("kg") == 1.0
        assert result.unit.get_exponent("s") == -1.0

    def test_divide_same_unit(self):
        reg = UnitRegistry()
        m = reg.parse("m")

        q1 = Quantity(UncertainValue(6.0, 0.1), m)
        q2 = Quantity(UncertainValue(3.0, 0.2), m)

        result = q1 / q2
        assert result.value.value == 2.0
        assert result.unit.get_exponent("m") is None


class TestQuantityRepr:
    """Tests for Quantity string representation."""

    def test_repr(self):
        reg = UnitRegistry()
        kg = reg.parse("kg")
        val = UncertainValue(5.0, 0.1)
        q = Quantity(val, kg)
        r = repr(q)
        assert "Quantity" in r
        assert "5" in r
        assert "kg" in r

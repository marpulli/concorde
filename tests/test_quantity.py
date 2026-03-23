"""Tests for Quantity Python bindings."""

import pytest
from concorde import UncertainValue, UnitRegistry, Quantity


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
    """Tests for Quantity multiplication."""

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

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


class TestQuantityConversion:
    """Tests for Quantity.to() unit conversion."""

    def _km_and_m(self):
        reg = UnitRegistry()
        reg.define_unit("km", [1, 0, 0, 0, 0, 0, 0], 1000.0)
        km = reg.parse("km")
        m = reg.parse("m")
        return km, m

    def test_to_converts_scalar_value(self):
        km, m = self._km_and_m()
        d = Quantity(UncertainValue(1.5, 0.1), km)

        converted = d.to(m)
        assert converted.value.value == 1500.0
        assert converted.unit.get_exponent("m") == 1.0

    def test_to_converts_uncertainty(self):
        km, m = self._km_and_m()
        d = Quantity(UncertainValue(1.5, 0.1), km)

        converted = d.to(m)
        assert abs(converted.value.uncertainty - 100.0) < 1e-10

    def test_to_incompatible_units_raises(self):
        km, m = self._km_and_m()
        reg = UnitRegistry()
        s = reg.parse("s")

        d = Quantity(UncertainValue(1.5, 0.1), km)
        with pytest.raises(IncompatibleUnitError):
            d.to(s)

    def test_to_round_trip(self):
        km, m = self._km_and_m()
        d = Quantity(UncertainValue(1.5, 0.1), km)

        round_tripped = d.to(m).to(km)
        assert abs(round_tripped.value.value - 1.5) < 1e-10


class TestQuantityScalarArithmetic:
    """Tests for Quantity ergonomics: scalar arithmetic, negation, equality, plain-float construction."""

    def test_multiply_by_scalar(self):
        reg = UnitRegistry()
        kg = reg.parse("kg")
        q = Quantity(UncertainValue(5.0, 0.1), kg)

        result = q * 2.0
        assert result.value.value == 10.0
        assert abs(result.value.uncertainty - 0.2) < 1e-10
        assert result.unit.get_exponent("kg") == 1.0

    def test_rmultiply_by_scalar(self):
        reg = UnitRegistry()
        kg = reg.parse("kg")
        q = Quantity(UncertainValue(5.0, 0.1), kg)

        result = 2.0 * q
        assert result.value.value == 10.0
        assert result.unit.get_exponent("kg") == 1.0

    def test_divide_by_scalar(self):
        reg = UnitRegistry()
        kg = reg.parse("kg")
        q = Quantity(UncertainValue(5.0, 0.1), kg)

        result = q / 2.0
        assert result.value.value == 2.5
        assert abs(result.value.uncertainty - 0.05) < 1e-10
        assert result.unit.get_exponent("kg") == 1.0

    def test_multiply_by_quantity_still_works(self):
        reg = UnitRegistry()
        kg = reg.parse("kg")
        m = reg.parse("m")

        q1 = Quantity(UncertainValue(5.0, 0.1), kg)
        q2 = Quantity(UncertainValue(3.0, 0.2), m)

        result = q1 * q2
        assert result.value.value == 15.0

    def test_negate(self):
        reg = UnitRegistry()
        kg = reg.parse("kg")
        q = Quantity(UncertainValue(5.0, 0.5), kg)

        result = -q
        assert result.value.value == -5.0
        assert abs(result.value.uncertainty - 0.5) < 1e-10

    def test_construct_from_plain_float(self):
        reg = UnitRegistry()
        kg = reg.parse("kg")
        q = Quantity(5.0, kg)

        assert q.value.value == 5.0
        assert q.value.uncertainty == 0.0

    def test_equal_quantities(self):
        reg = UnitRegistry()
        kg = reg.parse("kg")
        q1 = Quantity(UncertainValue(5.0, 0.1), kg)
        q2 = Quantity(UncertainValue(5.0, 0.1), kg)
        assert q1 == q2

    def test_unequal_values(self):
        reg = UnitRegistry()
        kg = reg.parse("kg")
        q1 = Quantity(UncertainValue(5.0, 0.1), kg)
        q2 = Quantity(UncertainValue(6.0, 0.1), kg)
        assert q1 != q2

    def test_unequal_units(self):
        reg = UnitRegistry()
        kg = reg.parse("kg")
        m = reg.parse("m")
        q1 = Quantity(UncertainValue(5.0, 0.1), kg)
        q2 = Quantity(UncertainValue(5.0, 0.1), m)
        assert q1 != q2

    def test_invalid_multiplication_operand_raises(self):
        reg = UnitRegistry()
        kg = reg.parse("kg")
        q = Quantity(UncertainValue(5.0, 0.1), kg)

        with pytest.raises(TypeError):
            q * "not a number"

    def test_invalid_construction_value_raises(self):
        reg = UnitRegistry()
        kg = reg.parse("kg")

        with pytest.raises(TypeError):
            Quantity("not a number", kg)


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

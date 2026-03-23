"""Tests for Unit and UnitRegistry Python bindings."""

import pytest
from concorde import UnitRegistry


class TestUnitRegistry:
    """Tests for UnitRegistry creation and parsing."""

    def test_create_registry(self):
        reg = UnitRegistry()
        assert reg is not None

    def test_parse_simple_unit(self):
        reg = UnitRegistry()
        kg = reg.parse("kg")
        assert kg.get_exponent("kg") == 1.0

    def test_parse_compound_unit(self):
        reg = UnitRegistry()
        unit = reg.parse("kg * m / s^2")
        assert unit.get_exponent("kg") == 1.0
        assert unit.get_exponent("m") == 1.0
        assert unit.get_exponent("s") == -2.0

    def test_parse_unknown_unit_raises(self):
        reg = UnitRegistry()
        with pytest.raises(ValueError):
            reg.parse("foo")

    def test_parse_bad_syntax_raises(self):
        reg = UnitRegistry()
        with pytest.raises(ValueError):
            reg.parse("kg @@ m")

    def test_implicit_multiplication(self):
        reg = UnitRegistry()
        unit = reg.parse("kg m")
        assert unit.get_exponent("kg") == 1.0
        assert unit.get_exponent("m") == 1.0

    def test_define_custom_unit(self):
        reg = UnitRegistry()
        reg.define_unit("N", [1.0, 1.0, -2.0, 0.0, 0.0, 0.0, 0.0], 1.0)
        newton = reg.parse("N")
        assert newton.get_exponent("N") == 1.0

    def test_define_custom_unit_with_aliases(self):
        reg = UnitRegistry()
        reg.define_unit(
            "N",
            [1.0, 1.0, -2.0, 0.0, 0.0, 0.0, 0.0],
            1.0,
            aliases=["newton"],
        )
        newton = reg.parse("N")
        assert newton.get_exponent("N") == 1.0


class TestUnitArithmetic:
    """Tests for Unit arithmetic operations."""

    def test_multiply_units(self):
        reg = UnitRegistry()
        kg = reg.parse("kg")
        m = reg.parse("m")
        result = kg * m
        assert result.get_exponent("kg") == 1.0
        assert result.get_exponent("m") == 1.0

    def test_divide_units(self):
        reg = UnitRegistry()
        m = reg.parse("m")
        s = reg.parse("s")
        result = m / s
        assert result.get_exponent("m") == 1.0
        assert result.get_exponent("s") == -1.0

    def test_power_unit(self):
        reg = UnitRegistry()
        m = reg.parse("m")
        result = m**3
        assert result.get_exponent("m") == 3.0

    def test_unit_cancellation(self):
        reg = UnitRegistry()
        m = reg.parse("m")
        result = m / m
        assert result.get_exponent("m") is None

    def test_compound_arithmetic(self):
        reg = UnitRegistry()
        kg = reg.parse("kg")
        m = reg.parse("m")
        s = reg.parse("s")
        # energy = kg * m^2 / s^2
        energy = kg * m**2 / s**2
        assert energy.get_exponent("kg") == 1.0
        assert energy.get_exponent("m") == 2.0
        assert energy.get_exponent("s") == -2.0


class TestUnitProperties:
    """Tests for Unit properties and display."""

    def test_repr(self):
        reg = UnitRegistry()
        kg = reg.parse("kg")
        r = repr(kg)
        assert "Unit" in r
        assert "kg" in r

    def test_components(self):
        reg = UnitRegistry()
        unit = reg.parse("kg * m")
        components = unit.components
        assert isinstance(components, dict)
        assert components["kg"] == 1.0
        assert components["m"] == 1.0

    def test_get_exponent_missing(self):
        reg = UnitRegistry()
        kg = reg.parse("kg")
        assert kg.get_exponent("m") is None

    def test_equality(self):
        reg = UnitRegistry()
        u1 = reg.parse("kg * m")
        u2 = reg.parse("m * kg")
        assert u1 == u2

    def test_hash_equal_units(self):
        reg = UnitRegistry()
        u1 = reg.parse("kg * m")
        u2 = reg.parse("m * kg")
        assert hash(u1) == hash(u2)

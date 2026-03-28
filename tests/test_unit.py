"""Tests for Unit and UnitRegistry Python bindings."""

import os
import tempfile

import pytest
from concorde import UnitRegistry

FIXTURES_DIR = os.path.join(os.path.dirname(__file__), "fixtures")


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


class TestLoadDefinitions:
    """Tests for loading unit definitions from TOML files."""

    def test_load_definitions_from_file(self):
        reg = UnitRegistry()
        path = os.path.join(FIXTURES_DIR, "units.toml")
        count = reg.load_definitions(path)
        assert count == 9  # 9 units defined in the fixture

    def test_loaded_units_are_parseable(self):
        reg = UnitRegistry()
        reg.load_definitions(os.path.join(FIXTURES_DIR, "units.toml"))
        newton = reg.parse("N")
        assert newton.get_exponent("N") == 1.0

    def test_loaded_units_in_compound_expressions(self):
        reg = UnitRegistry()
        reg.load_definitions(os.path.join(FIXTURES_DIR, "units.toml"))
        # Energy: J = kg * m^2 / s^2, so N * m should parse
        unit = reg.parse("N * m")
        assert unit.get_exponent("N") == 1.0
        assert unit.get_exponent("m") == 1.0

    def test_load_definitions_from_string(self):
        reg = UnitRegistry()
        toml_str = """
[[unit]]
name = "furlong"
dimensions = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
scale = 201.168
"""
        count = reg.load_definitions_from_string(toml_str)
        assert count == 1
        furlong = reg.parse("furlong")
        assert furlong.get_exponent("furlong") == 1.0

    def test_load_definitions_file_not_found(self):
        reg = UnitRegistry()
        with pytest.raises(ValueError, match="Failed to read file"):
            reg.load_definitions("/nonexistent/path.toml")

    def test_load_definitions_invalid_toml(self):
        reg = UnitRegistry()
        with pytest.raises(ValueError, match="Failed to parse TOML"):
            reg.load_definitions_from_string("this is {{ not valid toml")

    def test_load_definitions_preserves_existing_units(self):
        reg = UnitRegistry()
        # SI units like kg should still work after loading extra definitions
        reg.load_definitions(os.path.join(FIXTURES_DIR, "units.toml"))
        kg = reg.parse("kg")
        assert kg.get_exponent("kg") == 1.0

    def test_load_definitions_with_minimal_fields(self):
        """scale and aliases should default when omitted."""
        reg = UnitRegistry()
        toml_str = """
[[unit]]
name = "thing"
dimensions = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
"""
        count = reg.load_definitions_from_string(toml_str)
        assert count == 1
        thing = reg.parse("thing")
        assert thing.get_exponent("thing") == 1.0

    def test_load_multiple_files(self):
        reg = UnitRegistry()
        reg.load_definitions(os.path.join(FIXTURES_DIR, "units.toml"))

        # Load a second set from a string
        extra = """
[[unit]]
name = "ly"
dimensions = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
scale = 9.461e15
aliases = ["lightyear"]
"""
        reg.load_definitions_from_string(extra)
        assert reg.parse("ly").get_exponent("ly") == 1.0
        # Previously loaded units should still work
        assert reg.parse("N").get_exponent("N") == 1.0


class TestDerivedUnitDefinitions:
    """Tests for defining units in terms of existing units."""

    def test_derived_unit_simple_scale(self):
        """Define a unit as a scaled version of an existing one."""
        reg = UnitRegistry()
        toml_str = """
[[unit]]
name = "km"
unit = "m"
scale = 1000.0
"""
        reg.load_definitions_from_string(toml_str)
        km = reg.parse("km")
        assert km.get_exponent("km") == 1.0

    def test_derived_unit_compound_expression(self):
        """Define a unit from a compound expression like kg * m / s^2."""
        reg = UnitRegistry()
        toml_str = """
[[unit]]
name = "N"
unit = "kg * m / s^2"
"""
        reg.load_definitions_from_string(toml_str)
        newton = reg.parse("N")
        assert newton.get_exponent("N") == 1.0

    def test_derived_unit_chain(self):
        """Derived units can reference other derived units defined earlier."""
        reg = UnitRegistry()
        toml_str = """
[[unit]]
name = "N"
unit = "kg * m / s^2"

[[unit]]
name = "J"
unit = "N * m"

[[unit]]
name = "kJ"
unit = "J"
scale = 1000.0
"""
        reg.load_definitions_from_string(toml_str)
        kj = reg.parse("kJ")
        assert kj.get_exponent("kJ") == 1.0

    def test_derived_unit_in_compound_expression(self):
        """Derived units can be used in further compound expressions."""
        reg = UnitRegistry()
        toml_str = """
[[unit]]
name = "N"
unit = "kg * m / s^2"
"""
        reg.load_definitions_from_string(toml_str)
        # Use N in a compound expression
        torque = reg.parse("N * m")
        assert torque.get_exponent("N") == 1.0
        assert torque.get_exponent("m") == 1.0

    def test_derived_unit_scale_default(self):
        """Scale defaults to 1.0 when omitted on a derived unit."""
        reg = UnitRegistry()
        toml_str = """
[[unit]]
name = "N"
unit = "kg * m / s^2"
"""
        reg.load_definitions_from_string(toml_str)
        assert reg.parse("N") is not None

    def test_derived_unit_unknown_base_errors(self):
        """Referencing a non-existent unit should raise ValueError."""
        reg = UnitRegistry()
        toml_str = """
[[unit]]
name = "bad"
unit = "nonexistent"
"""
        with pytest.raises(ValueError, match="Failed to resolve unit"):
            reg.load_definitions_from_string(toml_str)

    def test_both_dimensions_and_unit_errors(self):
        """Specifying both dimensions and unit should raise ValueError."""
        reg = UnitRegistry()
        toml_str = """
[[unit]]
name = "bad"
dimensions = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
unit = "m"
"""
        with pytest.raises(ValueError, match="not both"):
            reg.load_definitions_from_string(toml_str)

    def test_neither_dimensions_nor_unit_errors(self):
        """Specifying neither dimensions nor unit should raise ValueError."""
        reg = UnitRegistry()
        toml_str = """
[[unit]]
name = "bad"
"""
        with pytest.raises(ValueError, match="must specify"):
            reg.load_definitions_from_string(toml_str)

    def test_fixture_file_uses_derived_units(self):
        """The fixture file defines derived units (N, J, W, Pa) from SI bases."""
        reg = UnitRegistry()
        reg.load_definitions(os.path.join(FIXTURES_DIR, "units.toml"))

        # All derived units should be parseable
        for name in ["N", "J", "W", "Pa", "km", "cm", "mm", "min", "hr"]:
            unit = reg.parse(name)
            assert unit.get_exponent(name) == 1.0, f"Failed to parse {name}"

    def test_mixed_base_and_derived_in_same_file(self):
        """A file can mix base (dimensions) and derived (unit) definitions."""
        reg = UnitRegistry()
        toml_str = """
[[unit]]
name = "furlong"
dimensions = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
scale = 201.168

[[unit]]
name = "kfurlong"
unit = "furlong"
scale = 1000.0
"""
        reg.load_definitions_from_string(toml_str)
        assert reg.parse("furlong").get_exponent("furlong") == 1.0
        assert reg.parse("kfurlong").get_exponent("kfurlong") == 1.0

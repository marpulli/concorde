"""Unit metadata through the public Python interface."""

import pytest

from concorde import AffineUnitError, Quantity, UncertainValue, UnitRegistry


@pytest.fixture
def registry():
    return UnitRegistry()


@pytest.mark.parametrize("source,target", [("degC", "delta_degC"), ("delta_degC", "degF")])
def test_dimensionally_compatible_does_not_override_affine_restrictions(registry, source, target):
    source_unit = registry.parse(source)
    target_unit = registry.parse(target)
    assert source_unit.is_compatible(target_unit)
    with pytest.raises(AffineUnitError):
        Quantity(UncertainValue(1.0, 0.0), source_unit).to(target_unit)


@pytest.mark.parametrize(
    "expression,affine,delta",
    [
        ("degC", True, False),
        ("degree_Fahrenheit", True, False),
        ("delta_degC", False, True),
        ("millidelta_degC / m", False, True),
        ("delta_degC / delta_degC", False, False),
        ("K", False, False),
        ("m", False, False),
        ("1", False, False),
    ],
)
def test_unit_flags(registry, expression, affine, delta):
    unit = registry.parse(expression)
    assert unit.is_affine is affine
    assert unit.is_delta is delta


def test_explicit_zero_offset_is_affine(registry):
    registry.define_unit("origin", [0, 0, 0, 0, 1, 0, 0], 2.0, offset=0.0)
    assert registry.parse("origin").is_affine
    assert registry.parse("delta_origin").is_delta


@pytest.mark.parametrize(
    "expression,dimensions",
    [
        ("1", (0, 0, 0, 0, 0, 0, 0)),
        ("kg * m / s^2", (1, 1, -2, 0, 0, 0, 0)),
        ("degC", (0, 0, 0, 0, 1, 0, 0)),
        ("delta_degC", (0, 0, 0, 0, 1, 0, 0)),
        ("m^(1/2) * A * mol / cd", (0.5, 0, 0, 1, 0, 1, -1)),
    ],
)
def test_dimensions(registry, expression, dimensions):
    assert registry.parse(expression).dimensions == dimensions


@pytest.mark.parametrize("attribute", ["is_affine", "is_delta", "dimensions"])
def test_metadata_is_read_only(registry, attribute):
    with pytest.raises(AttributeError):
        setattr(registry.parse("m"), attribute, None)


def test_compatibility_ignores_scale_and_component_names(registry):
    assert registry.parse("N").is_compatible(registry.parse("kg * m / s^2"))
    assert registry.parse("percent").is_compatible(registry.parse("1"))
    assert not registry.parse("m").is_compatible(registry.parse("s"))

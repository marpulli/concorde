"""Affine policy is shared by parsing, unit arithmetic and quantities."""

import operator

import numpy as np
import pytest

from concorde import (
    AffineUnitError,
    IncompatibleUnitError,
    Quantity,
    UnitRegistry,
    UncertainValue,
)


@pytest.fixture
def reg():
    return UnitRegistry()


def quantity(reg, value, unit, uncertainty=0.0):
    return Quantity(UncertainValue(value, uncertainty), reg.parse(unit))


def assert_quantity(result, reg, value, unit, uncertainty=None):
    assert result.value.value == pytest.approx(value)
    assert result.unit == reg.parse(unit)
    if uncertainty is not None:
        assert result.value.uncertainty == pytest.approx(uncertainty)


@pytest.mark.parametrize(
    "source,target,value,expected",
    [
        ("degC", "K", 20, 293.15),
        ("K", "degC", 273.15, 0),
        ("degC", "degF", 20, 68),
        ("degF", "degC", 32, 0),
        ("degF", "K", 32, 273.15),
        ("delta_degC", "delta_degF", 5, 9),
        ("delta_degF", "delta_degC", 9, 5),
        ("delta_degC", "K", 5, 5),
        ("K", "delta_degC", 5, 5),
    ],
)
def test_conversion(reg, source, target, value, expected):
    assert_quantity(
        quantity(reg, value, source).to(reg.parse(target)), reg, expected, target
    )


@pytest.mark.parametrize(
    "source,target",
    [
        ("degC", "delta_degC"),
        ("delta_degC", "degC"),
        ("degC", "delta_degF"),
        ("delta_degF", "degC"),
        ("millidelta_degC", "degC"),
    ],
)
def test_direct_absolute_delta_conversion_fails(reg, source, target):
    with pytest.raises(AffineUnitError):
        quantity(reg, 1, source).to(reg.parse(target))


@pytest.mark.parametrize(
    "lhs,rhs,operation,value,unit",
    [
        ((30, "degC"), (68, "degF"), operator.sub, 10, "delta_degC"),
        ((68, "degF"), (30, "degC"), operator.sub, -18, "delta_degF"),
        ((20, "degC"), (9, "delta_degF"), operator.add, 25, "degC"),
        ((9, "delta_degF"), (20, "degC"), operator.add, 25, "degC"),
        ((20, "degC"), (9, "delta_degF"), operator.sub, 15, "degC"),
        ((9, "delta_degF"), (20, "degC"), operator.sub, -15, "degC"),
        ((20, "degC"), (10, "K"), operator.sub, 283.15, "delta_degC"),
        ((10, "K"), (20, "degC"), operator.sub, -283.15, "K"),
        ((10, "K"), (20, "K"), operator.add, 30, "K"),
        ((10, "K"), (20, "K"), operator.sub, -10, "K"),
        ((5, "delta_degC"), (9, "delta_degF"), operator.add, 10, "delta_degC"),
        ((5, "delta_degC"), (9, "delta_degF"), operator.sub, 0, "delta_degC"),
        ((5, "delta_degC"), (10, "K"), operator.add, 15, "delta_degC"),
        ((10, "K"), (5, "delta_degC"), operator.add, 15, "K"),
    ],
)
def test_additive_plans(reg, lhs, rhs, operation, value, unit):
    result = operation(quantity(reg, *lhs), quantity(reg, *rhs))
    assert_quantity(result, reg, value, unit)


@pytest.mark.parametrize(
    "lhs,rhs", [("degC", "degC"), ("degC", "degF"), ("degC", "K"), ("K", "degC")]
)
def test_absolute_addition_fails(reg, lhs, rhs):
    with pytest.raises(AffineUnitError):
        quantity(reg, 20, lhs) + quantity(reg, 10, rhs)


@pytest.mark.parametrize(
    "operation",
    [
        lambda q: q * 2,
        lambda q: 2 * q,
        lambda q: q / 2,
        lambda q: 2 / q,
        lambda q: q * q,
        lambda q: q / q,
        lambda q: q**0,
        lambda q: q**1,
        lambda q: q**2,
        lambda q: q**-1,
        lambda q: q**0.5,
        operator.pos,
        operator.neg,
        abs,
    ],
)
def test_forbidden_quantity_operations(reg, operation):
    with pytest.raises(AffineUnitError):
        operation(quantity(reg, 20, "degC"))


@pytest.mark.parametrize("operation", [operator.mul, operator.truediv])
@pytest.mark.parametrize("other", ["K", "delta_degC", "m", "dimensionless"])
def test_affine_multiplication_both_positions(reg, operation, other):
    absolute = quantity(reg, 20, "degC")
    ordinary = quantity(reg, 2, other)
    for lhs, rhs in [(absolute, ordinary), (ordinary, absolute)]:
        with pytest.raises(AffineUnitError):
            operation(lhs, rhs)
        with pytest.raises(AffineUnitError):
            operation(lhs.unit, rhs.unit)


@pytest.mark.parametrize(
    "expression",
    [
        "degC / degC",
        "degC * m",
        "m / degC",
        "degC m",
        "degC * 1",
        "degC ** 0",
        "degC ** 1",
        "degC ** 2",
        "degC ** -1",
        "degC ** 0.5",
    ],
)
def test_parser_rejects_affine_operations_before_simplifying(reg, expression):
    with pytest.raises(AffineUnitError):
        reg.parse(expression)


@pytest.mark.parametrize("exponent", [0, 1, 2, -1, 0.5])
def test_bare_unit_powers_fail(reg, exponent):
    with pytest.raises(AffineUnitError):
        reg.parse("degC") ** exponent


def test_delta_and_kelvin_normal_arithmetic(reg):
    for unit in ["delta_degC", "K"]:
        q = quantity(reg, -5, unit, 0.2)
        assert_quantity(+q, reg, -5, unit, 0.2)
        assert_quantity(-q, reg, 5, unit, 0.2)
        assert_quantity(abs(q), reg, 5, unit, 0.2)
        assert_quantity(q * 2, reg, -10, unit, 0.4)
        assert_quantity(2 * q, reg, -10, unit, 0.4)
        assert_quantity(q / 2, reg, -2.5, unit, 0.1)
        assert_quantity(2 / q, reg, -0.4, f"1 / {unit}", 0.016)
        assert_quantity(q**0, reg, 1, "dimensionless", 0)
        assert_quantity(q**1, reg, -5, unit, 0.2)
        assert_quantity(q**2, reg, 25, f"{unit} ** 2", 2)
        assert_quantity(q * q, reg, 25, f"{unit} ** 2", 2)
        assert_quantity(q / q, reg, 1, "dimensionless", 0)
    gradient = quantity(reg, 5, "delta_degC / m")
    assert_quantity(gradient.to(reg.parse("K / m")), reg, 5, "K / m")


def test_uncertainty_and_correlation(reg):
    q = quantity(reg, 20, "degC", 0.5)
    fahrenheit = q.to(reg.parse("degF"))
    assert_quantity(fahrenheit, reg, 68, "degF", 0.9)
    assert_quantity(q - q, reg, 0, "delta_degC", 0)
    assert_quantity(q - fahrenheit, reg, 0, "delta_degC", 0)
    independent = quantity(reg, 68, "degF", 0.9)
    assert_quantity(q - independent, reg, 0, "delta_degC", np.sqrt(0.5))


def test_array_values(reg):
    q = quantity(reg, np.array([0.0, 20.0, 100.0]), "degC", np.array([0.1, 0.2, 0.3]))
    converted = q.to(reg.parse("degF"))
    np.testing.assert_allclose(converted.value.value, [32, 68, 212])
    np.testing.assert_allclose(converted.value.uncertainty, [0.18, 0.36, 0.54])
    difference = q - converted
    np.testing.assert_allclose(difference.value.value, 0, atol=1e-12)
    np.testing.assert_allclose(difference.value.uncertainty, 0, atol=1e-12)
    delta = quantity(
        reg, np.array([-2.0, 0.0, 3.0]), "delta_degC", np.array([0.1, 0.2, 0.3])
    )
    np.testing.assert_allclose(abs(delta).value.value, [2, 0, 3])
    np.testing.assert_allclose(abs(delta).value.uncertainty, [0.1, 0.2, 0.3])


def test_dimensions_still_checked(reg):
    q = quantity(reg, 20, "degC")
    for operation in [operator.add, operator.sub]:
        with pytest.raises(IncompatibleUnitError):
            operation(q, quantity(reg, 1, "m"))
    with pytest.raises(IncompatibleUnitError):
        q.to(reg.parse("m"))


def test_aliases_and_prefixes(reg):
    assert reg.parse("degC") == reg.parse("degree_Celsius")
    assert reg.parse("delta_degC") == reg.parse("delta_degree_Celsius")
    assert_quantity(
        quantity(reg, 1000, "millidelta_degC").to(reg.parse("delta_degC")),
        reg,
        1,
        "delta_degC",
    )
    with pytest.raises(ValueError):
        reg.parse("millidegC")


def test_python_definition_with_zero_offset_is_still_affine():
    reg = UnitRegistry(load_defaults=False)
    reg.define_unit("origin", [0, 0, 0, 0, 1, 0, 0], 2, ["o"], offset=0)
    q = quantity(reg, 3, "o")
    assert_quantity(q.to(reg.parse("K")), reg, 6, "K")
    assert_quantity(q - q, reg, 0, "delta_o")
    with pytest.raises(AffineUnitError):
        q**1


def test_toml_definition():
    reg = UnitRegistry(load_defaults=False)
    assert (
        reg.load_definitions_from_string("""
[[unit]]
name = "origin"
unit = "K"
scale = 2
offset = 10
aliases = ["o"]
""")
        == 1
    )
    assert_quantity(quantity(reg, 3, "o").to(reg.parse("K")), reg, 16, "K")
    assert_quantity(quantity(reg, 3, "delta_o").to(reg.parse("K")), reg, 6, "K")


@pytest.mark.parametrize("collision", ["delta_origin", "delta_o", "o"])
def test_generated_pair_registration_is_atomic(collision):
    reg = UnitRegistry(load_defaults=False)
    reg.define_unit(collision, [0, 0, 0, 0, 1, 0, 0], 1)
    cached = reg.parse("K")
    with pytest.raises(ValueError):
        reg.define_unit("origin", [0, 0, 0, 0, 1, 0, 0], 1, ["o"], offset=10)
    with pytest.raises(ValueError):
        reg.parse("origin")
    assert reg.parse("K") == cached


def test_derived_delta_keeps_its_kind(reg):
    reg.load_definitions_from_string("""
[[unit]]
name = "double_delta"
unit = "delta_degC"
scale = 2
""")
    delta = quantity(reg, 3, "double_delta")
    assert_quantity(quantity(reg, 20, "degC") + delta, reg, 26, "degC")
    with pytest.raises(AffineUnitError):
        delta.to(reg.parse("degC"))


@pytest.mark.parametrize("name", ["degree_Celsius", "delta_degree_Celsius"])
def test_pair_cannot_be_redefined(reg, name):
    original = reg.parse(name)
    with pytest.raises(ValueError):
        reg.define_unit(name, [0, 0, 0, 0, 1, 0, 0], 2)
    assert reg.parse(name) == original


@pytest.mark.parametrize(
    "scale,offset", [(0, 0), (-1, 0), (float("inf"), 0), (1, float("nan"))]
)
def test_invalid_affine_definition_is_rejected(reg, scale, offset):
    with pytest.raises(ValueError):
        reg.define_unit("invalid", [0, 0, 0, 0, 1, 0, 0], scale, offset=offset)
    with pytest.raises(ValueError):
        reg.parse("delta_invalid")


def test_affine_derived_definition_rejected(reg):
    with pytest.raises(ValueError, match="derived definition"):
        reg.load_definitions_from_string("""
[[unit]]
name = "bad"
unit = "degC"
""")
    with pytest.raises(ValueError):
        reg.parse("bad")

"""Alias registration rejects ambiguous names without partially applying a unit."""

import pytest
from concorde import Quantity, UncertainValue, UnitRegistry


DIMENSIONS = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]


@pytest.mark.parametrize(
    "name,aliases,message",
    [
        ("bar", ["fresh", "shared"], "Duplicate alias 'shared'"),
        ("bar", ["fresh", "fresh"], "Duplicate alias 'fresh'"),
        ("bar", ["m"], "conflicts with a unit name"),
        ("bar", ["bar"], "conflicts with a unit name"),
        ("shared", [], "already registered as an alias"),
        ("foo", ["shared"], "Duplicate alias 'shared'"),
    ],
)
def test_rejected_definition_is_atomic(name, aliases, message):
    registry = UnitRegistry()
    registry.define_unit("foo", DIMENSIONS, 1.0, aliases=["shared"])
    assert registry.parse("shared").components == {"foo": 1.0}

    with pytest.raises(ValueError, match=message):
        registry.define_unit(name, DIMENSIONS, 2.0, aliases=aliases)

    assert registry.parse("shared").components == {"foo": 1.0}
    converted = Quantity(UncertainValue(1.0, 0.0), registry.parse("foo")).to(registry.parse("m"))
    assert converted.value.value == 1.0
    for unknown in ("bar", "fresh"):
        with pytest.raises(ValueError):
            registry.parse(unknown)


def test_toml_aliases_resolve_in_later_definitions():
    registry = UnitRegistry()
    assert registry.load_definitions_from_string('''
[[unit]]
name = "foo"
unit = "m"
aliases = ["shared"]
[[unit]]
name = "bar"
unit = "shared / s"
''') == 2
    assert registry.parse("shared / s").components == {"foo": 1.0, "s": -1.0}
    assert registry.parse("bar").components == {"bar": 1.0}


def test_toml_duplicate_alias_propagates_error():
    registry = UnitRegistry()
    registry.define_unit("foo", DIMENSIONS, 1.0, aliases=["shared"])
    with pytest.raises(ValueError, match="Duplicate alias 'shared'"):
        registry.load_definitions_from_string('''
[[unit]]
name = "bar"
unit = "m"
aliases = ["fresh", "shared"]
''')
    assert registry.parse("shared").components == {"foo": 1.0}
    for unknown in ("bar", "fresh"):
        with pytest.raises(ValueError):
            registry.parse(unknown)

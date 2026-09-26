"""Prefix behavior through the public Python interface."""

from pathlib import Path

import pytest
from concorde import Quantity, UncertainValue, UnitRegistry

LENGTH = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
MASS = [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0]
TIME = [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0]

SI_PREFIXES = [
    ("q", "quecto", -30),
    ("r", "ronto", -27),
    ("y", "yocto", -24),
    ("z", "zepto", -21),
    ("a", "atto", -18),
    ("f", "femto", -15),
    ("p", "pico", -12),
    ("n", "nano", -9),
    ("µ", "micro", -6),
    ("m", "milli", -3),
    ("c", "centi", -2),
    ("d", "deci", -1),
    ("da", "deca", 1),
    ("h", "hecto", 2),
    ("k", "kilo", 3),
    ("M", "mega", 6),
    ("G", "giga", 9),
    ("T", "tera", 12),
    ("P", "peta", 15),
    ("E", "exa", 18),
    ("Z", "zetta", 21),
    ("Y", "yotta", 24),
    ("R", "ronna", 27),
    ("Q", "quetta", 30),
]


@pytest.fixture
def registry():
    registry = UnitRegistry(load_defaults=False)
    registry.load_definitions(str(Path(__file__).parent / "fixtures" / "units.toml"))
    registry.load_definitions_from_string("""
[[unit]]
name = "L"
unit = "m^3"
scale = 0.001
aliases = ["liter"]

[[unit]]
name = "Wh"
unit = "W * hr"
""")
    return registry


def assert_conversion(registry, source, target, factor):
    quantity = Quantity(UncertainValue(1.0, 0.1), registry.parse(source))
    converted = quantity.to(registry.parse(target))
    assert converted.value.value == pytest.approx(factor, rel=1e-12, abs=0)
    assert converted.value.uncertainty == pytest.approx(0.1 * factor, rel=1e-12, abs=0)


@pytest.mark.parametrize("symbol,name,exponent", SI_PREFIXES)
def test_all_si_prefix_symbols_and_names(symbol, name, exponent):
    registry = UnitRegistry()
    for spelling in [symbol + "m", name + "meter", name + "metres"]:
        assert_conversion(registry, spelling, "m", 10.0**exponent)
        unit = registry.parse(spelling)
        assert unit.components == {symbol + "m": 1.0}
        assert unit == registry.parse(symbol + "m")
        assert hash(unit) == hash(registry.parse(symbol + "m"))


@pytest.mark.parametrize("root", ["g", "m", "s", "A", "K", "mol", "cd"])
def test_all_builtin_roots_accept_prefixes(root):
    assert_conversion(UnitRegistry(), "k" + root, root, 1000)


@pytest.mark.parametrize("spelling", ["um", "µm", "μm", "micrometer"])
def test_micro_spellings_share_canonical_identity(spelling):
    registry = UnitRegistry()
    unit = registry.parse(spelling)
    canonical = registry.parse("µm")
    assert str(unit) == "µm"
    assert unit == canonical
    assert hash(unit) == hash(canonical)
    assert registry.parse("um / µm").components == {}
    assert UnitRegistry().parse(str(unit)) == unit


def test_alternative_deca_spelling():
    registry = UnitRegistry()
    assert registry.parse("dekameter") == registry.parse("dam")


@pytest.mark.parametrize("spelling,scale", [("g", 0.001), ("kg", 1), ("mg", 1e-6)])
def test_mass_scales_remain_relative_to_coherent_si(spelling, scale):
    registry = UnitRegistry()
    registry.define_unit("mass_reference", MASS, 1)
    assert_conversion(registry, spelling, "mass_reference", scale)
    assert registry.parse("kilogram") == registry.parse("kg")


def test_custom_units_and_aliases_accept_prefixes():
    registry = UnitRegistry()
    registry.define_unit("smoot", LENGTH, 1.7018, aliases=["sm"])
    assert_conversion(registry, "ksm", "m", 1701.8)
    assert registry.parse("ksm") == registry.parse("kilosmoot")
    assert str(registry.parse("ksm")) == "ksmoot"
    registry.define_unit("credit", [0.0] * 7, 2)
    assert_conversion(registry, "millicredit", "credit", 0.001)


def test_unicode_root_accepts_prefixes():
    registry = UnitRegistry()
    registry.define_unit("Ω", LENGTH, 2)
    assert_conversion(registry, "µΩ", "Ω", 1e-6)
    assert str(registry.parse("uΩ")) == "µΩ"


@pytest.mark.parametrize("alias", [False, True])
def test_exact_names_and_aliases_win_over_prefixes(alias):
    registry = UnitRegistry(load_defaults=False)
    registry.define_unit("in", LENGTH, 0.0254)
    registry.define_unit(
        "minute" if alias else "min", TIME, 60, aliases=["min"] if alias else []
    )
    assert_conversion(registry, "min", "s", 60)


def test_longest_valid_prefix_wins():
    registry = UnitRegistry()
    registry.define_unit("am", LENGTH, 7)
    # Both da + m and d + am are valid. Longest wins, not registration order.
    assert_conversion(registry, "dam", "m", 10)
    registry.define_unit("ax", LENGTH, 7)
    # da + x is unknown, so the shorter valid d + ax is used.
    assert_conversion(registry, "dax", "m", 0.7)


def test_case_sensitive_prefixes(registry):
    assert_conversion(registry, "MPa", "Pa", 1e6)
    assert_conversion(registry, "mPa", "Pa", 1e-3)
    assert registry.parse("MPa") != registry.parse("mPa")
    with pytest.raises(ValueError, match="Unknown unit 'KM'"):
        registry.parse("KM")


@pytest.mark.parametrize(
    "expression", ["kmm", "kilomillimeter", "mkg", "millikilogram", "uum"]
)
@pytest.mark.parametrize("warm_cache", [False, True])
def test_prefixes_never_stack(expression, warm_cache):
    registry = UnitRegistry()
    if warm_cache:
        for unit in ["mm", "millimeter", "kg", "kilogram", "um"]:
            registry.parse(unit)
    with pytest.raises(ValueError, match=f"Unknown unit '{expression}'"):
        registry.parse(expression)


@pytest.mark.parametrize("expression", ["KiB", "kunknown", "k"])
def test_unknown_and_binary_prefixes_fail(expression):
    registry = UnitRegistry()
    registry.define_unit("B", [0.0] * 7, 1)
    with pytest.raises(ValueError, match=f"Unknown unit '{expression}'"):
        registry.parse(expression)


@pytest.mark.parametrize(
    "source,target,factor",
    [
        ("kJ", "J", 1e3),
        ("MJ", "J", 1e6),
        ("GJ", "J", 1e9),
        ("mg/L", "kg/m^3", 0.001),
        ("kWh/kg", "J/kg", 3.6e6),
        ("mm^2", "m^2", 1e-6),
        ("cm^-1", "m^-1", 100),
        ("mm^(1/2)", "m^(1/2)", 0.001**0.5),
        ("N", "kg*m/s^2", 1),
    ],
)
def test_prefixes_in_compounds_and_powers(registry, source, target, factor):
    assert_conversion(registry, source, target, factor)


def test_toml_definitions_can_use_generated_units_and_become_new_roots(registry):
    registry.load_definitions_from_string("""
[[unit]]
name = "journey"
unit = "km"
scale = 2
aliases = ["trip"]
""")
    assert_conversion(registry, "journey", "m", 2000)
    assert_conversion(registry, "ktrip", "m", 2e6)


@pytest.mark.parametrize(
    "expression", ["mg/L", "kWh/kg", "um^2", "centimeters^-1", "kg^(2/5)"]
)
def test_round_trip_in_fresh_registry(registry, expression):
    unit = registry.parse(expression)
    fresh = UnitRegistry(load_defaults=False)
    fresh.load_definitions(str(Path(__file__).parent / "fixtures" / "units.toml"))
    fresh.load_definitions_from_string("""
[[unit]]
name = "L"
unit = "m^3"
scale = 0.001
[[unit]]
name = "Wh"
unit = "W * hr"
""")
    restored = fresh.parse(str(unit))
    assert restored == unit
    assert hash(restored) == hash(unit)
    assert str(restored) == str(unit)


@pytest.mark.parametrize("alias", [False, True])
def test_new_exact_definition_invalidates_parse_cache(alias):
    registry = UnitRegistry()
    assert_conversion(registry, "km/s", "m/s", 1000)
    registry.define_unit(
        "route" if alias else "km", LENGTH, 7, aliases=["km"] if alias else []
    )
    assert_conversion(registry, "km/s", "m/s", 7)
    # A previously unparsed expression also uses the new exact definition.
    assert_conversion(registry, "km*s", "m*s", 7)


def test_redefining_root_invalidates_generated_units():
    registry = UnitRegistry()
    registry.define_unit("span", LENGTH, 2)
    assert_conversion(registry, "kspan/s", "m/s", 2000)
    registry.define_unit("span", LENGTH, 3)
    assert_conversion(registry, "kspan/s", "m/s", 3000)
    assert_conversion(registry, "kilospan*s", "m*s", 3000)


@pytest.mark.parametrize("alias", [False, True])
@pytest.mark.parametrize("warm_cache", [False, True])
def test_canonical_name_collision_uses_an_alternative_spelling(alias, warm_cache):
    registry = UnitRegistry()
    if warm_cache:
        registry.parse("kilometer")
    registry.define_unit(
        "route" if alias else "km", LENGTH, 7, aliases=["km"] if alias else []
    )
    assert_conversion(registry, "km", "m", 7)
    assert_conversion(registry, "kilometer", "m", 1000)
    unit = registry.parse("kilometer")
    assert str(unit) == "kilom"
    assert unit != registry.parse("km")
    for spelling in ["kilom", "kmetre", "kilometers"]:
        assert registry.parse(spelling) == unit
        assert hash(registry.parse(spelling)) == hash(unit)
    fresh = UnitRegistry()
    fresh.define_unit(
        "route" if alias else "km", LENGTH, 7, aliases=["km"] if alias else []
    )
    assert fresh.parse(str(unit)) == unit


def test_new_root_invalidates_cached_longest_prefix_match():
    registry = UnitRegistry()
    registry.define_unit("ax", LENGTH, 7)
    assert_conversion(registry, "dax", "m", 0.7)
    registry.define_unit("x", LENGTH, 3)
    # The new da + x interpretation now beats d + ax.
    assert_conversion(registry, "dax", "m", 30)
    assert_conversion(registry, "deciax", "m", 0.7)
    assert str(registry.parse("deciax")) == "deciax"
    assert registry.parse("deciax") != registry.parse("dax")


def test_exact_definitions_can_resemble_stacked_prefixes():
    registry = UnitRegistry()
    registry.define_unit("route", LENGTH, 7, aliases=["mkg"])
    assert_conversion(registry, "mkg", "m", 7)
    assert_conversion(registry, "kmkg", "m", 7000)


def test_canonical_name_cannot_change_prefix_split():
    registry = UnitRegistry()
    registry.define_unit("am", LENGTH, 7)
    # deci + am would serialize as dam, but dam resolves as deca + m.
    assert_conversion(registry, "deciam", "m", 0.7)
    assert_conversion(registry, "dam", "m", 10)
    unit = registry.parse("deciam")
    assert str(unit) == "deciam"
    assert unit != registry.parse("dam")
    fresh = UnitRegistry()
    fresh.define_unit("am", LENGTH, 7)
    restored = fresh.parse(str(unit))
    assert restored == unit
    assert hash(restored) == hash(unit)


def test_canonical_fallback_uses_sorted_root_aliases():
    def make_registry(reverse):
        registry = UnitRegistry()
        aliases = ["zeta", "alpha", "beta"]
        blockers = ["kwidget", "kilowidget", "kalpha", "kiloalpha"]
        registry.define_unit(
            "widget", LENGTH, 2, aliases=aliases[::-1] if reverse else aliases
        )
        for name in reversed(blockers) if reverse else blockers:
            registry.define_unit(name, LENGTH, 7)
        return registry

    registry = make_registry(False)
    fresh = make_registry(True)
    for spelling in ["kzeta", "kilozeta", "kilobeta", "kbeta"]:
        assert_conversion(registry, spelling, "m", 2000)
        unit = registry.parse(spelling)
        assert str(unit) == "kbeta"
        assert registry.parse("kbeta") == unit
        assert hash(registry.parse("kbeta")) == hash(unit)
        restored = fresh.parse(str(unit))
        assert restored == unit
        assert hash(restored) == hash(unit)
        assert str(fresh.parse(spelling)) == "kbeta"


def test_canonical_fallback_uses_aliases_retained_after_redefinition():
    registry = UnitRegistry()
    registry.define_unit("widget", LENGTH, 2, aliases=["label"])
    registry.define_unit("widget", LENGTH, 3)
    for name in ["kwidget", "kilowidget"]:
        registry.define_unit(name, LENGTH, 7)
    assert_conversion(registry, "kilolabel", "m", 3000)
    assert str(registry.parse("kilolabel")) == "klabel"


def test_micro_falls_back_without_conflating_exact_units():
    registry = UnitRegistry()
    registry.define_unit("µm", LENGTH, 7)
    assert_conversion(registry, "µm", "m", 7)
    for spelling in ["um", "μm", "micrometer"]:
        assert_conversion(registry, spelling, "m", 1e-6)
        unit = registry.parse(spelling)
        assert str(unit) == "μm"
        assert registry.parse(str(unit)) == unit
        assert unit != registry.parse("µm")

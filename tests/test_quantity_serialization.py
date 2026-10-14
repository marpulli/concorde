import json
import subprocess
import sys

import numpy as np
import pytest

from concorde import Quantity, UncertainValue, UnitRegistry


@pytest.mark.parametrize("unit", ["m", "cm", "kg / m^3", "m^(1/2) / s", "1", "degC", "degF", "delta_degC / m"])
@pytest.mark.parametrize("values", [3.0, np.array([1., 2., 3.]), np.array([])])
def test_round_trip_in_fresh_registry(unit, values):
    registry = UnitRegistry()
    q = Quantity(UncertainValue(values, .2), registry.parse(unit))
    loaded = Quantity.from_json(q.to_json(), UnitRegistry())
    assert loaded.unit == q.unit
    np.testing.assert_allclose(loaded.value.value, q.value.value)
    np.testing.assert_allclose(loaded.value.uncertainty, q.value.uncertainty)
    np.testing.assert_allclose((loaded-q).value.uncertainty, 0., atol=1e-12)
    assert loaded.to_json() == q.to_json()


def test_registry_is_required():
    q = Quantity(UncertainValue(3., .2), UnitRegistry().parse("m"))
    with pytest.raises(TypeError):
        Quantity.from_json(q.to_json())


def test_conversion_between_separately_loaded_quantities_preserves_sources():
    registry = UnitRegistry()
    original = Quantity(UncertainValue(np.array([1., 2., 3.]), .2), registry.parse("m"))
    converted = original.to(registry.parse("cm"))
    loaded_original = Quantity.from_json(original.to_json(), UnitRegistry())
    loaded_converted = Quantity.from_json(converted.to_json(), UnitRegistry())
    np.testing.assert_allclose((loaded_original-loaded_converted).value.uncertainty, 0., atol=1e-12)
    selected = Quantity.from_json(converted[::-1].to_json(), UnitRegistry())
    np.testing.assert_allclose((selected-loaded_original[::-1]).value.uncertainty, 0., atol=1e-12)


def test_affine_and_delta_units_survive_loading():
    registry = UnitRegistry()
    original = Quantity(UncertainValue(20., .5), registry.parse("degC"))
    converted = original.to(registry.parse("degF"))
    a = Quantity.from_json(original.to_json(), UnitRegistry())
    b = Quantity.from_json(converted.to_json(), UnitRegistry())
    assert a.unit.is_affine
    assert (b-a).unit.is_delta
    assert (b-a).value.value == pytest.approx(0., abs=1e-12)
    assert (b-a).value.uncertainty == pytest.approx(0., abs=1e-12)
    delta = Quantity.from_json((b-a).to_json(), UnitRegistry())
    assert delta.unit.is_delta
    assert not delta.unit.is_affine


def custom_registry(scale=2.0, dimensions=(1, 0, 0, 0, 0, 0, 0), offset=None):
    registry = UnitRegistry(load_defaults=False)
    registry.define_unit("custom", dimensions, scale, aliases=["my_alias"], offset=offset)
    return registry


def test_custom_units_are_not_installed_by_loading():
    q = Quantity(UncertainValue(3., .2), custom_registry().parse("my_alias"))
    registry = UnitRegistry(load_defaults=False)
    with pytest.raises(ValueError, match="Unknown"):
        Quantity.from_json(q.to_json(), registry)
    with pytest.raises(ValueError):
        registry.parse("custom")
    loaded = Quantity.from_json(q.to_json(), custom_registry())
    assert loaded.to_json() == q.to_json()
    assert loaded.to(registry.parse("m")).value.value == 6.


@pytest.mark.parametrize("registry", [custom_registry(scale=3.), custom_registry(dimensions=(0, 1, 0, 0, 0, 0, 0)), custom_registry(offset=0.)])
def test_registry_definition_mismatch_is_rejected(registry):
    q = Quantity(UncertainValue(3., .2), custom_registry().parse("custom"))
    with pytest.raises(ValueError, match="Unit definitions"):
        Quantity.from_json(q.to_json(), registry)


def test_affine_offset_mismatch_is_rejected():
    q = Quantity(UncertainValue(3., .2), custom_registry(offset=5.).parse("custom"))
    with pytest.raises(ValueError, match="Unit definitions"):
        Quantity.from_json(q.to_json(), custom_registry(offset=6.))


@pytest.mark.parametrize("change", [
    lambda p: p.update(version=2),
    lambda p: p.update(extra=1),
    lambda p: p["value"].update(version=2),
    lambda p: p["unit"].update(expression="unknown_unit"),
    lambda p: p["unit"].update(extra=1),
    lambda p: p["unit"].update(components=[]),
    lambda p: p["unit"]["components"].append(p["unit"]["components"][0].copy()),
    lambda p: p["unit"]["components"][0].update(scale=10.),
    lambda p: p["unit"]["components"][0].update(exponent=2.),
    lambda p: p["unit"]["components"][0].update(kind={"type": "delta"}),
])
def test_invalid_payloads(change):
    registry = UnitRegistry()
    q = Quantity(UncertainValue(3., .2), registry.parse("m"))
    payload = json.loads(q.to_json())
    change(payload)
    with pytest.raises(ValueError):
        Quantity.from_json(json.dumps(payload), registry)


def test_payload_embeds_value_data_not_a_json_string():
    q = Quantity(UncertainValue(3., .2), UnitRegistry().parse("cm"))
    payload = json.loads(q.to_json())
    assert payload["value"] == json.loads(q.value.to_json())
    assert payload["unit"]["expression"] == "cm"
    assert payload["unit"]["components"][0]["scale"] == .01


def test_separate_quantities_reconnect_in_fresh_process():
    registry = UnitRegistry()
    q = Quantity(UncertainValue(3., .2), registry.parse("m"))
    converted = q.to(registry.parse("cm"))
    code = """
import sys
from concorde import Quantity, UnitRegistry
registry = UnitRegistry()
a = Quantity.from_json(sys.stdin.readline(), registry)
b = Quantity.from_json(sys.stdin.readline(), registry)
assert abs((a-b).value.uncertainty) < 1e-12
print(b.to_json())
"""
    result = subprocess.run([sys.executable, "-c", code], input=q.to_json()+"\n"+converted.to_json()+"\n", text=True, capture_output=True, check=True)
    assert (Quantity.from_json(result.stdout, registry)-q).value.uncertainty == pytest.approx(0., abs=1e-12)

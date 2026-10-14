"""Checked by Pyright in CI; keep scalar and array return types precise."""

from __future__ import annotations

from typing import assert_type

import numpy as np
from numpy.typing import NDArray

from concorde import (
    AffineUnitError,
    IncompatibleUnitError,
    Quantity,
    UncertainValue,
    Unit,
    UnitRegistry,
)

registry = UnitRegistry(load_defaults=True)
metre = registry.parse("m")
centimetre = registry.parse("cm")
assert_type(metre, Unit)
assert_type(metre.is_affine, bool)
assert_type(metre.is_delta, bool)
assert_type(metre.dimensions, tuple[float, float, float, float, float, float, float])
assert_type(metre.components, dict[str, float])
assert_type(metre.get_exponent("m"), float | None)
assert_type(metre.is_compatible(centimetre), bool)
assert_type(metre * centimetre / metre**2, Unit)

values = np.array([1.0, 2.0], dtype=np.float64)

scalar = UncertainValue(2.0, 0.1)
array = UncertainValue(values, 0.1)
assert_type(scalar.value, float)
assert_type(scalar.uncertainty, float)
assert_type(scalar.variable_id, str | None)
assert_type(scalar.is_independent(), bool)
assert_type(array.value, NDArray[np.float64])
assert_type(array.uncertainty, NDArray[np.float64])
assert_type(UncertainValue(values, values).value, NDArray[np.float64])
assert_type((scalar + scalar - 1.0).value, float)
assert_type((scalar * array).value, NDArray[np.float64])
assert_type((array / scalar).value, NDArray[np.float64])
assert_type((scalar - array).value, NDArray[np.float64])
assert_type((1.0 / array).value, NDArray[np.float64])
assert_type((2.0 * -scalar**2).value, float)

quantity = Quantity(scalar, metre)
array_quantity = Quantity(array, metre)
assert_type(quantity.value.value, float)
assert_type(quantity.unit, Unit)
assert_type(quantity.to(centimetre).value.value, float)
assert_type(quantity.to_base_unit().value.value, float)
assert_type(array_quantity.to_base_unit().value.value, NDArray[np.float64])
assert_type(array_quantity.to(centimetre).value.value, NDArray[np.float64])
assert_type((quantity + array_quantity).value.value, NDArray[np.float64])
assert_type((array_quantity - quantity).value.value, NDArray[np.float64])
assert_type((quantity * array_quantity).value.value, NDArray[np.float64])
assert_type((quantity / array_quantity).value.value, NDArray[np.float64])
assert_type((2.0 / quantity).value.value, float)
assert_type((abs(-quantity) ** 2).value.value, float)

registry.define_unit("span", [1, 0, 0, 0, 0, 0, 0], 2, aliases=["sp"])
registry.define_unit("origin", (0, 0, 0, 0, 1, 0, 0), 1, offset=0)
assert_type(registry.load_definitions("units.toml"), int)
assert_type(registry.load_definitions_from_string(""), int)
assert_type(IncompatibleUnitError(), IncompatibleUnitError)
assert_type(AffineUnitError(), AffineUnitError)

# Unnecessary-ignore diagnostics make these negative checks fail if stubs loosen.
quantity.to("cm")  # pyright: ignore[reportArgumentType]
Quantity(2.0, metre)  # pyright: ignore[reportArgumentType]
scalar.__add__(metre)  # pyright: ignore[reportArgumentType, reportCallIssue]
metre.is_affine = False  # pyright: ignore[reportAttributeAccessIssue]

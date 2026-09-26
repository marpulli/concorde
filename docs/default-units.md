# Default units

`UnitRegistry()` loads the seven SI roots plus `src/default_units.toml`.
The TOML is embedded at compile time, so loading does not depend on the
working directory or a runtime data-file path. Rebuild the native extension
with `maturin develop` after editing the catalog.

Use `UnitRegistry(load_defaults=False)` for SI roots only, for example before
loading a custom catalog with overlapping aliases. Existing duplicate-alias
and alias/name collision checks still apply.

## Scope and sources

The catalog covers the units found by this read-only inventory of the local
`postgres_isometric` database on port 5433 (the table is named `datapoint`):

```sql
SELECT DISTINCT value->>'unit'
FROM public.datapoint
WHERE value->>'unit' IS NOT NULL
ORDER BY 1;
```

The inventory contained 108 strings across 2,618,832 datapoints. All 108
strings resolve, including empty input as dimensionless and `degree_Celsius`
as an affine temperature unit.

- Standard definitions follow [Pint 0.26.1's default_en.txt](https://github.com/hgrecco/pint/blob/0.26.1/pint/default_en.txt).
- Domain definitions follow `backend/domain/units.py` in
  `~/isometric/python/services/backend`, including CO2e, hydroxide, charge,
  chemical equivalents, USD, TEU and `short_ton_2`.

The catalog entries are roots and necessary derived units, not one entry
per stored expression. Prefixes such as `kilo`, `mega`, and `micro` resolve
lazily against roots and aliases: `kgCO2e` derives from `gCO2e`,
`kilogram_hydroxide` from `gram_hydroxide`, and `kilowatt_hour` from `watt_hour`.
Compounds and powers remain parser expressions. Aliases share one definition.

Notable source conventions are preserved: gallons are US liquid gallons,
acres use US survey feet, `short_ton_2` uses the backend's rounded 907.185 kg,
CO2e has ordinary mass dimensions, and USD is dimensionless. This is a
usage-derived catalog, not the entirety of Pint's unit vocabulary.

## Affine units and differences

`degree_Celsius` (`degC`, `celsius`) and `degree_Fahrenheit` (`degF`,
`fahrenheit`) represent absolute temperatures. Their `delta_` names and
aliases represent differences: `delta_degC`, `delta_degF`, etc.

```python
from concorde import Quantity, UncertainValue, UnitRegistry

reg = UnitRegistry()
c = Quantity(UncertainValue(30, 0.1), reg.parse("degC"))
f = Quantity(UncertainValue(68, 0.2), reg.parse("degF"))
difference = c - f  # approximately 10 delta_degree_Celsius
converted = c.to(reg.parse("K"))  # 303.15 K, uncertainty 0.1 K
```

- Absolute + absolute fails; absolute − absolute returns the left scale's delta.
- Absolute ± delta and delta ± absolute return the absolute operand's unit.
  Delta − absolute is allowed: `5 delta_degC − 20 degC = -15 degC`.
- Multiplication, division, all powers (including 0 and 1), unary `+`, unary
  `-`, and `abs` fail on affine quantities. Delta quantities use normal arithmetic.
- Compounds require explicit deltas: `delta_degC / m` is valid; `degC / m`,
  `degC / degC`, and `degC ** 0` fail, including when parsing unit expressions.
- Delta conversion uses scale only. Direct absolute ↔ delta conversion fails.
- Kelvin remains ordinary and multiplicative. `degC + K` fails, whereas
  `degC − K` subtracts absolute temperatures and returns `delta_degC`;
  `K − degC` returns `K`. Kelvin converts to either absolute or delta units.
- Prefixes are allowed on deltas (e.g. `millidelta_degC`), not affine units.

Custom definitions accept `offset` in coherent base units:
`base_value = value * scale + offset`. An explicit offset, even zero, makes
an affine unit. Registration also creates `delta_<name>` and `delta_<alias>`
with the same scale. The affine/delta pair is registered atomically; names
in that pair cannot subsequently be redefined.

```toml
[[unit]]
name = "custom_temperature"
unit = "K"
scale = 2.0
offset = 10.0
```

The equivalent Python definition is:

```python
reg.define_unit("custom_temperature", [0, 0, 0, 0, 1, 0, 0], 2.0, offset=10.0)
```

Derived definitions must not use affine units in their `unit` expression.
Offsets translate values without adding uncertainty; scale changes apply to
both values and uncertainty and preserve correlations. There is no automatic
conversion setting for forbidden operations. Python raises `AffineUnitError`
(a `ValueError`) for affine-operation failures.

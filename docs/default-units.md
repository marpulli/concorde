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

The inventory contained 108 strings across 2,618,832 datapoints. All 107
non-Celsius strings resolve, including empty input as dimensionless.
`degree_Celsius` remains unsupported: a scale-only definition would silently
produce incorrect absolute-temperature conversions.

- Standard definitions follow [Pint 0.26.1's default_en.txt](https://github.com/hgrecco/pint/blob/0.26.1/pint/default_en.txt).
- Domain definitions follow `backend/domain/units.py` in
  `~/isometric/python/services/backend`, including CO2e, hydroxide, charge,
  chemical equivalents, USD, TEU and `short_ton_2`.

The 29 catalog entries are roots and necessary derived units, not one entry
per stored expression. Prefixes such as `kilo`, `mega`, and `micro` resolve
lazily against roots and aliases: `kgCO2e` derives from `gCO2e`,
`kilogram_hydroxide` from `gram_hydroxide`, and `kilowatt_hour` from `watt_hour`.
Compounds and powers remain parser expressions. Aliases share one definition.

Notable source conventions are preserved: gallons are US liquid gallons,
acres use US survey feet, `short_ton_2` uses the backend's rounded 907.185 kg,
CO2e has ordinary mass dimensions, and USD is dimensionless. This is a
usage-derived catalog, not the entirety of Pint's unit vocabulary.

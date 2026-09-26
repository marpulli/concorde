# SI prefixes

`UnitRegistry.parse` accepts all 24 SI decimal prefixes, from quecto (`q`,
10⁻³⁰) to quetta (`Q`, 10³⁰), on every defined multiplicative unit and its
aliases. No prefixability flag is needed. All current unit definitions are
multiplicative (scale-only).

```python
from concorde import UnitRegistry

registry = UnitRegistry()
assert str(registry.parse("kilometer")) == "km"
assert registry.parse("um") == registry.parse("µm")
assert registry.parse("kilogram") == registry.parse("kg")

registry.define_unit("smoot", [1, 0, 0, 0, 0, 0, 0], 1.7018, aliases=["sm"])
assert str(registry.parse("ksm")) == "ksmoot"
```

Both symbols (`k`) and full prefix names (`kilo`) are accepted, combined with
any registered root name or alias. Matching is case-sensitive: `M` is mega,
while `m` is milli. Micro accepts `µ` (micro sign), `μ` (Greek mu), `u`, and
`micro`; its preferred canonical spelling is `µ`. Deca also accepts `deka`.

Binary prefixes and runtime prefix registration are not supported. The SI
table is data-driven, but is currently fixed for every registry.

## Lookup and collisions

1. An exact registered unit name or alias wins.
2. Otherwise, try prefix spellings longest-first. The remainder must be an
   exact registered unit name or alias. A longer prefix with an unknown
   remainder does not prevent a shorter valid interpretation.
3. Create a unit with the root's dimensions and `prefix.scale * root.scale`.

Generated units prefer the canonical prefix symbol followed by the root's
canonical name. If that spelling would resolve to another explicit unit,
alias, or prefix decomposition, naming tries alternative prefix spellings
in the table's order. If none work with the canonical root, it tries the
root's registered aliases in lexical order, again trying each prefix spelling.
The chosen name always resolves to the original prefix and root. Alternate
input spellings share the same name, equality, and hashing, independently of
parse order or alias registration order.

For example, with an unrelated exact `km`, `kilometer` still means 1000 metres
but serializes as `kilom`. With an explicit root `am`, `deciam` retains that
spelling rather than becoming `dam` (deca + metre). These clashes do not make
valid input fail.

Prefixes do not stack: `kmm`, `mkg`, and `millikilogram` fail, even after `mm`
or `kg` has been parsed. They raise the same unknown-identifier `ValueError` as
any other unrecognized unit. Parsed units share one cache, separate from
explicit definitions, and never become prefix roots. Exact definitions still
take precedence even when their spelling resembles a stacked prefix.

An explicitly named unit is a root, including a TOML definition referencing a
prefixed expression. For example, defining `journey = 2 * km` through TOML's
`unit = "km"` and `scale = 2` fields permits `kjourney`. The resolver does not
infer prefix provenance from the spelling or physical scale of an explicit
definition. Catalogs should not enumerate `km`, `cm`, or `mm` as separate roots.

## Mass and compound expressions

The built-in mass root is `g` (aliases `gram`, `grams`); `kg` is generated as
kilo + gram. Numeric scales remain relative to coherent SI kilograms:
`g` has scale 0.001, `kg` has scale 1, and `mg` has scale 0.000001.

Prefixes work inside ordinary expressions, including powers: `mm^2` has
scale 10⁻⁶ relative to `m^2`. After registering the required catalog units,
expressions such as `mg/L` and `kWh/kg` work without special parsing rules.
Definitions such as `N = kg * m / s^2` retain their existing scale.

Successful unit definitions invalidate the parse cache.
Already returned units remain snapshots; subsequent parses use the updated
registry. Canonical round trips assume the same, unchanged definitions.

## Implementation

`src/prefix.rs` stores the SI table and a lazily initialized, longest-first
spelling index. `src/unit_registry.rs` performs exact lookup, prefix resolution,
unambiguous naming, and caching. Resolution scans the small prefix index and
performs hash-map lookups of possible roots; it does not materialize a prefix ×
unit product. Naming normally uses the canonical root; only when all its
prefixed spellings clash does it collect and sort that root's aliases from the
registry's alias map.

The tokenizer, compound-expression grammar, and conversion arithmetic do not
need prefix-specific rules. Public behavior is tested in
`tests/test_prefixes.py`.

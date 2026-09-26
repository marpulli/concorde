# Concorde backend adoption: remaining work

## Decision frame

This is a candidate backlog, not a set of filed issues. This document does not itself file or update GitHub issues.

Backend requirements below retain the earlier audit; backend source has not been re-audited for this revision.

**Dependency decision:** use Concorde as an explicit local editable dependency for now. Distribution renaming, public/private indexes, wheel publication, release automation, and artifact verification are out of scope. An editable dependency still needs a Rust/PyO3 build step when native code changes; document that in the development setup rather than treating release packaging as an adoption prerequisite.

**Ownership boundary**

- **Concorde owns generic library behavior:** safe parsing, generic unit definitions and defaults, affine conversion, value semantics, NumPy interop, and typing.
- **The Isometric backend owns application policy:** its unit vocabulary and aliases, CO₂e/chemistry/currency definitions, API formatting, Pydantic and JSON persistence adapters, display regularization, statistics, interpolation, outlier handling, bootstrap policy, and HEOS/CoolProp calculations.
- Upstream only behavior useful to other unit-library consumers. Do not move backend algorithms or business definitions into Concorde merely to simplify migration.

**Priorities**

- **P0:** blocks safe runtime behavior, persisted-data compatibility, public-input safety, or the first reliable scalar adapter.
- **P1:** required before removing Pint, but can follow the first scalar adapter.
- **P2:** adoption hygiene that does not block controlled migration.

Candidate IDs are retained for continuity. Completed work is omitted; partial items describe only remaining work. References to existing GitHub issues indicate where to consolidate scope, not a fresh audit of issue open/closed status.

## Adoption constraints

- The backend targets CPython 3.14.6. Exercise the editable extension on the actual development/integration runtime; do not infer support from Concorde's current `requires-python >=3.8` metadata.
- The earlier backend audit found 114 `StandardUnit` strings using 58 identifiers, including `%`, temperatures, `µm`, CO₂e, chemistry, currency, and compounds (`backend/domain/units.py:337-452`). The default registry still lacks the derived/common unit catalog needed by the backend.
- Persistence is an existing JSON contract: scalar fields use `magnitude`, `standard_deviation`, `unit`; list fields use `magnitude`, `standard_deviation`, `units`. Preserve these JSONB payloads and the registered custom fields.
- Backend wrappers own considerably more than unit arithmetic: `ScalarQuantity` includes Pydantic and value semantics; `ListQuantity` includes reductions and domain algorithms; `TimeSeriesQuantity` includes interpolation and axis alignment.

Paths beginning `backend/` or backend `tests/` refer to `/Users/mark.pullin/isometric/python/services/backend`. Concorde paths refer to this repository. Backend line references are from the earlier audit and may drift.

## Concorde quality checks

### R3. Finish source quality checks and an explicit runtime support policy

- **Consolidate with:** [#14](https://github.com/marpulli/concorde/issues/14).
- **Priority:** P0.
- **Remaining scope:** Rust formatting/clippy checks and a documented tested Python range. Resolve existing warnings before denying warnings in CI. Add stub checks when C7 lands.
- **Acceptance criteria:**
  - Rust formatting is checked in PR CI.
  - Python support metadata agrees with the tested runtime policy.
  - Clippy runs with warnings denied after cleanup.
  - C7 runtime/stub drift checks run in the same workflow.
- **Out of scope:** release pipelines, wheel matrices, index publication, and artifact installation checks.

README work ([#13](https://github.com/marpulli/concorde/issues/13)) remains P2. Document editable setup, native rebuild commands, registry behavior, and the public API once those interfaces settle; release packaging is not a prerequisite.

## Concorde API and semantic parity

### C1. Finish parser diagnostics, grammar documentation, and resource limits

- **Related issue:** [#9](https://github.com/marpulli/concorde/issues/9).
- **Priority:** P0.
- **Remaining evidence:** Errors lack precise source positions, and no explicit input/token/depth limits are established.
- **Scope:** Document the supported identifier and expression grammar; include offending input and position in errors; impose input-length, token-count, and nesting limits before unbounded work or recursion.
- **Acceptance criteria:**
  - Unknown identifiers, malformed tokens/exponents, and unmatched parentheses produce useful position-bearing Python errors.
  - Existing valid multiplication/division/power forms and dimensionless empty-input behavior remain intact.
  - Boundary and malformed-input tests demonstrate deterministic errors rather than panic, stack overflow, or excessive work.
- **Non-goals:** unit definitions, fuzzy spelling recovery, or changing empty input back into an error.

### C3. Add a generic default unit catalog

- **Consolidate with:** [#8](https://github.com/marpulli/concorde/issues/8).
- **Priority:** P0.
- **Dependencies:** C4 for affine defaults; coordinate with C1 on grammar documentation.
- **Remaining evidence:** The default registry contains only the roots g, m, s, A, K, mol, and cd; richer definitions are only in fixtures.
- **Scope:** Add generic derived SI/common definitions using the existing TOML loader and alias mechanism. Provide deterministic default loading and an opt-out.
- **Acceptance criteria:**
  - The minimum generic derived/common units in #8 and their aliases resolve, with conversion spot checks.
  - Defaults load from an editable checkout without depending on the caller's working directory.
  - Default loading and opt-out are documented and deterministic.
  - Catalog construction respects the existing duplicate-alias and alias/name collision errors; it does not rely on silent shadowing.
- **Non-goals:** implementing aliases again, wheel data tests, Isometric-only definitions, or enumerating every prefix/unit product.

### C4. Implement affine units with explicit absolute/delta semantics

- **Consolidate with:** [#10](https://github.com/marpulli/concorde/issues/10).
- **Priority:** P0.
- **Remaining evidence:** definitions, TOML, and conversion are scale-only. The backend uses `degC`/`degF` and uncertain temperature subtraction.
- **Scope:** Add affine metadata to Python/TOML definitions; implement K/Celsius/Fahrenheit conversion; decide absolute-versus-delta arithmetic and uncertainty behavior; reject undefined compound/powered affine use.
- **Acceptance criteria:**
  - Temperature value and uncertainty round trips are correct.
  - Subtraction has documented delta semantics, or unsupported add/sub fails explicitly until implemented.
  - Invalid affine powers and compounds fail clearly.
  - Definitions, conversion, and arithmetic tests cover valid and invalid paths.
- **Non-goals:** nonlinear/logarithmic units or Isometric-specific temperature policy.

### C5. Complete Quantity value semantics and numeric ergonomics

- **Consolidate with:** [#6](https://github.com/marpulli/concorde/issues/6).
- **Priority:** P0.
- **Dependencies:** C4 for affine comparisons; C6 for base normalization.
- **Remaining evidence:** Python construction requires `UncertainValue`; scalar/reverse arithmetic, negation, value equality/hash, and ordering are missing from `src/quantity_python.rs`.
- **Scope and acceptance criteria:**
  - Integer/float construction creates zero-uncertainty quantities.
  - `q*n`, `n*q`, `q/n`, and `-q` preserve value, uncertainty, and unit.
  - Independently constructed equivalent scalar quantities compare equal across convertible units; uncertainty participates in equality.
  - Equal scalar quantities have equal hashes across convertible units.
  - Ordering converts compatible units and compares central values, with documented incompatible/non-Quantity behavior.
  - Array equality is specified separately; arrays remain unhashable if scalar hashing is not applicable.
- **Non-goals:** reimplementing power, approximate equality without a separate API, or copying backend wrappers into Concorde.

### C6. Add public compatibility/base APIs and affine round trips

- **Priority:** P0.
- **Dependencies:** C1 for grammar; C4 for affine conversion.
- **Remaining evidence:** Public compatibility and Unit/Quantity base-unit APIs are missing. Future affine forms need round-trip coverage, and the general canonical serialization contract still needs documentation.
- **Scope and acceptance criteria:**
  - Extend `registry.parse(str(unit)) == unit` coverage to supported affine forms.
  - Document the general canonical-name serialization contract and its distinction from diagnostic repr.
  - `unit.is_compatible(other)` is public and tested.
  - Public Unit/Quantity base conversion handles scale and uncertainty correctly, including 1.5 km → 1500 m and generated kg as the coherent mass unit.
- **Non-goals:** Isometric Unicode/UI formatting or replacing diagnostic repr with a persistence contract.

### C7. Add complete, checked type stubs

- **Consolidate with:** [#11](https://github.com/marpulli/concorde/issues/11).
- **Priority:** P1.
- **Dependencies:** C1 and C3-C6 settle the public API; R3 hosts checks.
- **Remaining evidence:** `py.typed` exists but the extension stub does not.
- **Scope and acceptance criteria:**
  - Add `_concorde.pyi` covering all runtime exports, exceptions, properties, methods, and numeric overloads.
  - Include NumPy types, comparisons, registry/default loading, compatibility/base conversion, and affine options as those APIs land.
  - Type check a consumer using the editable dependency without `Any` leakage.
  - Validate runtime/stub agreement in CI.
- **Non-goals:** backend adapter types or wheel contents/publication checks.

## Backend-only migration and integration

### B1. Add the backend registry, custom catalog, and API formatter

- **Priority:** P0.
- **Dependencies:** C1, C3, C4, C6; explicit editable dependency setup.
- **Scope:** Create one backend-owned registry using generic defaults plus Isometric definitions. Preserve `StandardUnit` and persisted Pint spellings; implement `format_unit`/`deformat_unit` using the public Unit API.
- **Acceptance criteria:**
  - All 114 audited `StandardUnit` strings parse and convert to base units; all 58 identifiers resolve through names, aliases, or prefixes.
  - %, temperatures, CO₂e, chemistry, `_100g`, USD, and TEU have explicit backend definitions.
  - Existing persisted scalar/list unit strings deserialize without migration.
  - API spelling and formatting contracts, including CO₂, middle-dot, exponents, and inverse formatting, remain intact.
  - Initialization is centralized; application callers cannot arbitrarily redefine the shared registry.
- **Non-goals:** upstreaming domain definitions or using diagnostic repr in APIs.

### B2. Reimplement ScalarQuantity on Concorde while preserving contracts

- **Priority:** P0.
- **Dependencies:** B1, C5-C7.
- **Scope:** Preserve the backend wrapper while replacing Pint internals. Translate errors; retain raw numeric/Decimal construction, uncertainty, rounding, Pydantic, and database serialization. Remove `from_pint`/`_pint_quantity` and migrate callers rather than keeping misleading shims.
- **Acceptance criteria:**
  - Existing scalar construction, arithmetic, conversion/base, compatibility, equality/hash/order, and rounding tests pass.
  - JSON/Pydantic keys and old-row deserialization remain unchanged.
  - REST/GraphQL values, standard deviations, and formatted units retain their contracts.
  - The scalar implementation and migrated callers no longer require Pint types.
- **Non-goals:** renaming database custom types, changing JSON keys, or exposing raw Concorde objects through APIs.

### B3. Migrate ListQuantity without moving statistics upstream

- **Priority:** P1.
- **Dependencies:** B2.
- **Scope:** Back value/uncertainty arrays and arithmetic with Concorde. Preserve backend length/exception rules, reductions, indexing/filtering, comparison, and JSON serialization.
- **Acceptance criteria:**
  - Existing list behavior and conversion/base tests pass.
  - Invalid shapes produce expected backend errors, never Rust panics.
  - Persisted list JSON retains the `units` key and round-trips modulo normal numeric normalization.
  - Winsorization, bootstrap, median/outlier, discounting, and HEOS behavior remain backend-owned and tested.
  - No list path constructs or imports Pint quantities.

### B4. Migrate TimeSeriesQuantity while retaining interpolation policy

- **Priority:** P1.
- **Dependencies:** B2 and B3.
- **Scope:** Replace the two Pint quantities with backend adapters; retain independent/dependent APIs, axis alignment, interpolation/extrapolation, arithmetic, conversions, and Pydantic validation.
- **Acceptance criteria:**
  - Aligned/misaligned and differing-length arithmetic preserves axes and values.
  - Axis conversion/base conversion preserves results.
  - Linear interpolation, nearest-neighbor extrapolation, and six-decimal axis coalescing remain unchanged.
  - Invalid shapes or incompatible units raise backend errors.
  - No direct Pint types or operations remain.

### B5. Cut over remaining callsites and remove Pint/uncertainties

- **Priority:** P1.
- **Dependencies:** B1-B4.
- **Remaining evidence:** The earlier audit found direct Pint use in conversions, AI conversion tools, sensor creation, tracer compatibility, REST parsing, and expression-parser types. Backend unit code and tests also use `uncertainties`.
- **Scope:** Route these callers through backend-owned adapters; replace Pint exceptions, unit containers, registry internals, and `uncertainties` construction/introspection. Remove direct dependencies after all callers migrate. Centralize direct Concorde access through import policy.
- **Acceptance criteria:**
  - Backend setup explicitly points to the intended local Concorde checkout as an editable dependency; a bare index install of the unrelated `concorde` project is not used.
  - Document native build/rebuild steps and exercise parse/convert/persistence against the freshly built extension on CPython 3.14.
  - No backend/test runtime or type-only imports reference Pint or `uncertainties`; remove direct dependencies and obsolete lock entries, accounting for any genuine transitive requirements.
  - Conversion regularization, AI tools, sensor/tracer validation, REST parsing, and expression behavior retain their results/errors.
  - Import policy prohibits direct Concorde use outside the backend adapter boundary.
- **Non-goals:** wheel publication, production artifact delivery, domain logic upstreaming, or silent dual-engine fallbacks. Production deployment distribution can be planned separately when needed.

## Ordered rollout

1. Configure the editable Concorde checkout and document native rebuild/test commands.
2. Add source safety checks: R3 and C1.
3. Finish generic parity: C3-C6, then C7 against the resulting public API. Keep README work P2.
4. Build the backend boundary: B1, then B2 as the first parse/convert/persist end-to-end slice.
5. Move composite wrappers: B3, then B4, without changing backend policy.
6. Complete B5 and remove the old engines. No release publication milestone is required for this editable-dependency phase.

## Global non-goals

- This document does not create/update GitHub issues or change backend code, dependencies, database rows, or schemas.
- No existing quantity JSON migration unless evidence proves compatible parsing impossible; prefer compatibility in B1/B2.
- No silent Pint/Concorde disagreement through compatibility fallbacks.
- No upstream migration of domain vocabulary, UI formatting, statistics, interpolation, HEOS/CoolProp, or regularization policy.
- No claim of Python-version support beyond the tested policy.
- No packaging/release backlog in this phase. Editable integration is not itself a production deployment design.

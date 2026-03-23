"""Benchmark comparing concorde vs pint+uncertainties for common unit operations."""

import timeit
import numpy as np

import concorde
import pint
from uncertainties import ufloat
from uncertainties import unumpy

N_SCALAR = 100_000
N_ARRAY = 1_000
ARRAY_SIZE = 10_000


def fmt(seconds, n):
    per_op = seconds / n
    if per_op < 1e-6:
        return f"{per_op * 1e9:.1f} ns/op"
    elif per_op < 1e-3:
        return f"{per_op * 1e6:.1f} µs/op"
    else:
        return f"{per_op * 1e3:.1f} ms/op"


def bench(name, concorde_fn, pint_fn, n):
    ct = timeit.timeit(concorde_fn, number=n)
    pt = timeit.timeit(pint_fn, number=n)
    ratio = pt / ct
    print(f"  {name:<45} concorde: {fmt(ct, n):>12}  pint: {fmt(pt, n):>12}  speedup: {ratio:.1f}x")


def main():
    # --- Setup: concorde ---
    ureg_c = concorde.UnitRegistry()
    m_c = ureg_c.parse("m")
    kg_c = ureg_c.parse("kg")
    s_c = ureg_c.parse("s")

    val_c = concorde.UncertainValue(5.0, 0.1)
    val2_c = concorde.UncertainValue(3.0, 0.2)
    q1_c = concorde.Quantity(val_c, m_c)
    q2_c = concorde.Quantity(val2_c, m_c)
    q3_c = concorde.Quantity(val2_c, kg_c)

    arr_vals = np.random.rand(ARRAY_SIZE)
    arr_uncs = np.random.rand(ARRAY_SIZE) * 0.1
    arr_c = concorde.UncertainValue(arr_vals, arr_uncs)
    arr2_c = concorde.UncertainValue(arr_vals * 2, arr_uncs * 0.5)
    qa_c = concorde.Quantity(arr_c, m_c)
    qa2_c = concorde.Quantity(arr2_c, m_c)
    qa3_c = concorde.Quantity(arr2_c, kg_c)

    # --- Setup: pint + uncertainties ---
    ureg_p = pint.UnitRegistry()
    q1_p = ufloat(5.0, 0.1) * ureg_p.meter
    q2_p = ufloat(3.0, 0.2) * ureg_p.meter
    q3_p = ufloat(3.0, 0.2) * ureg_p.kilogram

    qa_p = unumpy.uarray(arr_vals, arr_uncs) * ureg_p.meter
    qa2_p = unumpy.uarray(arr_vals * 2, arr_uncs * 0.5) * ureg_p.meter
    qa3_p = unumpy.uarray(arr_vals * 2, arr_uncs * 0.5) * ureg_p.kilogram

    # =========================================================================
    # Scalar benchmarks
    # =========================================================================
    print(f"Scalar benchmarks ({N_SCALAR} iterations)")
    print("=" * 100)

    bench(
        "Quantity creation (with uncertainty)",
        lambda: concorde.Quantity(concorde.UncertainValue(5.0, 0.1), m_c),
        lambda: ufloat(5.0, 0.1) * ureg_p.meter,
        N_SCALAR,
    )

    bench(
        "Unit parsing (simple: 'm')",
        lambda: ureg_c.parse("m"),
        lambda: ureg_p.parse_units("m"),
        N_SCALAR,
    )

    bench(
        "Unit parsing (compound: 'kg m / s^2')",
        lambda: ureg_c.parse("kg m / s^2"),
        lambda: ureg_p.parse_units("kg * m / s ** 2"),
        N_SCALAR,
    )

    bench(
        "Quantity multiply (different units)",
        lambda: q1_c * q3_c,
        lambda: q1_p * q3_p,
        N_SCALAR,
    )

    # Note: concorde Quantity __add__/__sub__ are currently broken at the
    # Python binding level, so we compare UncertainValue add vs pint Quantity add.
    bench(
        "Scalar add (UncertainValue vs pint Q)",
        lambda: val_c + val2_c,
        lambda: q1_p + q2_p,
        N_SCALAR,
    )

    bench(
        "Unit multiply",
        lambda: m_c * kg_c,
        lambda: ureg_p.meter * ureg_p.kilogram,
        N_SCALAR,
    )

    bench(
        "Unit divide",
        lambda: m_c / s_c,
        lambda: ureg_p.meter / ureg_p.second,
        N_SCALAR,
    )

    bench(
        "Unit power",
        lambda: m_c ** 3,
        lambda: ureg_p.meter ** 3,
        N_SCALAR,
    )

    # =========================================================================
    # Array benchmarks
    # =========================================================================
    print()
    print(f"Array benchmarks ({N_ARRAY} iterations, array size {ARRAY_SIZE})")
    print("=" * 100)

    bench(
        "Array quantity creation (with uncertainty)",
        lambda: concorde.Quantity(concorde.UncertainValue(arr_vals, arr_uncs), m_c),
        lambda: unumpy.uarray(arr_vals, arr_uncs) * ureg_p.meter,
        N_ARRAY,
    )

    bench(
        "Array quantity multiply (diff units)",
        lambda: qa_c * qa3_c,
        lambda: qa_p * qa3_p,
        N_ARRAY,
    )

    bench(
        "Array quantity add (same units)",
        lambda: arr_c + arr2_c,  # UncertainValue add (Quantity add is broken)
        lambda: qa_p + qa2_p,
        N_ARRAY,
    )


if __name__ == "__main__":
    main()

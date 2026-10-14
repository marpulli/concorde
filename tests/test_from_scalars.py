import numpy as np
import pytest
from concorde import UncertainValue


def test_from_scalars_retains_repeated_and_original_array_sources():
    x = UncertainValue(np.array([1., 2.]), .1)
    values = UncertainValue.from_scalars([x[0], x[1], x[0]])
    assert (values.sum()-2*x[0]-x[1]).uncertainty == 0.
    assert (values[0]-values[2]).uncertainty == 0.
    assert (UncertainValue.from_json(values.to_json()).sum()-values.sum()).uncertainty == 0.


def test_empty_and_invalid_scalar_assembly():
    values = UncertainValue.from_scalars([])
    assert values.value.shape == (0,)
    assert values.uncertainty.shape == (0,)
    with pytest.raises(TypeError):
        UncertainValue.from_scalars([values])

import numpy as np
import pytest
from concorde import UncertainValue


def test_weight_before_squaring_avoids_intermediate_overflow_and_underflow():
    for scale, sigma in [(1e200, 1e-200), (1e-200, 1e200)]:
        assert (UncertainValue(1., sigma)*scale).uncertainty == pytest.approx(1.)
        np.testing.assert_allclose((UncertainValue(np.ones(2), sigma)*scale).uncertainty, 1.)


def test_scaled_uncertainty_matches_scalar_multiplication():
    assert (UncertainValue(500000., 50.)*.001).uncertainty == .05

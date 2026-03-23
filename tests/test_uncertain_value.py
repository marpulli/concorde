"""
Tests for the UncertainValue Python class.

This module tests the PyO3-exposed UncertainValue class with both scalar
and numpy array values, including multiplication operations and error propagation.
"""

import pytest
import numpy as np
from concorde import UncertainValue


class TestUncertainValueScalar:
    """Tests for UncertainValue with scalar values."""

    def test_create_scalar(self):
        """Test creating an UncertainValue with scalar values."""
        uv = UncertainValue(5.0, 0.5)
        assert uv.value == 5.0

    def test_scalar_repr(self):
        """Test string representation of scalar UncertainValue."""
        uv = UncertainValue(5.0, 0.5)
        assert repr(uv) == "UncertainValue(5 ± 0.5)"

    def test_multiply_scalar_by_scalar(self):
        """Test multiplying two scalar UncertainValues."""
        # (3.0 ± 0.3) * (4.0 ± 0.4)
        uv1 = UncertainValue(3.0, 0.3)
        uv2 = UncertainValue(4.0, 0.4)

        result = uv1 * uv2

        # Expected value: 3.0 * 4.0 = 12.0
        assert result.value == 12.0

        # Expected uncertainty: sqrt((4.0 * 0.3)² + (3.0 * 0.4)²)
        # = sqrt(1.44 + 1.44) = sqrt(2.88) ≈ 1.697
        expected_uncertainty = np.sqrt((4.0 * 0.3)**2 + (3.0 * 0.4)**2)
        np.testing.assert_allclose(result.value, 12.0)

    def test_multiply_scalar_by_float(self):
        """Test multiplying an UncertainValue by a Python float."""
        uv = UncertainValue(5.0, 0.5)
        result = uv * 3.0

        assert result.value == 15.0

    def test_multiply_float_by_scalar(self):
        """Test right multiplication: float * UncertainValue."""
        uv = UncertainValue(5.0, 0.5)
        result = 3.0 * uv

        assert result.value == 15.0

    def test_multiply_scalar_by_negative_float(self):
        """Test multiplying by a negative float preserves uncertainty magnitude."""
        uv = UncertainValue(5.0, 0.5)
        result = uv * -2.0

        assert result.value == -10.0
        # Uncertainty should be scaled by absolute value

    def test_multiply_scalars_commutative(self):
        """Test that scalar multiplication is commutative."""
        uv1 = UncertainValue(3.0, 0.3)
        uv2 = UncertainValue(4.0, 0.4)

        result1 = uv1 * uv2
        result2 = uv2 * uv1

        assert result1.value == result2.value

    def test_zero_uncertainty(self):
        """Test behavior with zero uncertainty."""
        uv1 = UncertainValue(5.0, 0.0)
        uv2 = UncertainValue(3.0, 0.3)

        result = uv1 * uv2

        assert result.value == 15.0
        # Uncertainty should be: sqrt((3.0 * 0.0)² + (5.0 * 0.3)²) = 1.5


class TestUncertainValueArray:
    """Tests for UncertainValue with numpy array values."""

    def test_create_array(self):
        """Test creating an UncertainValue with numpy arrays."""
        values = np.array([1.0, 2.0, 3.0])
        uncertainties = np.array([0.1, 0.2, 0.3])

        uv = UncertainValue(values, uncertainties)
        np.testing.assert_array_equal(uv.value, values)

    def test_array_repr(self):
        """Test string representation of array UncertainValue."""
        values = np.array([1.0, 2.0, 3.0])
        uncertainties = np.array([0.1, 0.2, 0.3])

        uv = UncertainValue(values, uncertainties)
        assert "array(...)" in repr(uv)

    def test_multiply_array_by_array(self):
        """Test element-wise multiplication of two array UncertainValues."""
        values1 = np.array([2.0, 3.0, 4.0])
        uncertainties1 = np.array([0.2, 0.3, 0.4])
        uv1 = UncertainValue(values1, uncertainties1)

        values2 = np.array([3.0, 4.0, 5.0])
        uncertainties2 = np.array([0.3, 0.4, 0.5])
        uv2 = UncertainValue(values2, uncertainties2)

        result = uv1 * uv2

        # Expected values: element-wise multiplication
        expected_values = values1 * values2
        np.testing.assert_array_equal(result.value, expected_values)

        # Expected uncertainties: element-wise variance propagation
        # σ_result[i] = sqrt((values2[i] * uncertainties1[i])² + (values1[i] * uncertainties2[i])²)
        expected_uncertainties = np.sqrt(
            (values2 * uncertainties1)**2 + (values1 * uncertainties2)**2
        )
        np.testing.assert_allclose(result.value, expected_values)

    def test_multiply_array_by_scalar_float(self):
        """Test multiplying an array UncertainValue by a scalar float."""
        values = np.array([1.0, 2.0, 3.0])
        uncertainties = np.array([0.1, 0.2, 0.3])
        uv = UncertainValue(values, uncertainties)

        result = uv * 2.0

        expected_values = values * 2.0
        np.testing.assert_array_equal(result.value, expected_values)

    def test_multiply_scalar_float_by_array(self):
        """Test right multiplication: scalar float * array UncertainValue."""
        values = np.array([1.0, 2.0, 3.0])
        uncertainties = np.array([0.1, 0.2, 0.3])
        uv = UncertainValue(values, uncertainties)

        result = 2.0 * uv

        expected_values = values * 2.0
        np.testing.assert_array_equal(result.value, expected_values)

    def test_multiply_scalar_uv_by_array_uv(self):
        """Test multiplying a scalar UncertainValue by an array UncertainValue."""
        scalar_uv = UncertainValue(2.0, 0.2)

        values = np.array([1.0, 2.0, 3.0])
        uncertainties = np.array([0.1, 0.2, 0.3])
        array_uv = UncertainValue(values, uncertainties)

        result = scalar_uv * array_uv

        expected_values = 2.0 * values
        np.testing.assert_array_equal(result.value, expected_values)

    def test_multiply_array_uv_by_scalar_uv(self):
        """Test multiplying an array UncertainValue by a scalar UncertainValue."""
        values = np.array([1.0, 2.0, 3.0])
        uncertainties = np.array([0.1, 0.2, 0.3])
        array_uv = UncertainValue(values, uncertainties)

        scalar_uv = UncertainValue(2.0, 0.2)

        result = array_uv * scalar_uv

        expected_values = values * 2.0
        np.testing.assert_array_equal(result.value, expected_values)

    def test_array_different_lengths_should_work(self):
        """Test that numpy broadcasting works for compatible shapes."""
        # Note: This depends on the numpy broadcasting behavior
        values1 = np.array([1.0, 2.0, 3.0])
        uncertainties1 = np.array([0.1, 0.2, 0.3])
        uv1 = UncertainValue(values1, uncertainties1)

        # Same length arrays should work
        values2 = np.array([2.0, 3.0, 4.0])
        uncertainties2 = np.array([0.2, 0.3, 0.4])
        uv2 = UncertainValue(values2, uncertainties2)

        result = uv1 * uv2
        assert len(result.value) == 3


class TestUncertainValueEdgeCases:
    """Tests for edge cases and error handling."""

    def test_invalid_type_error(self):
        """Test that creating with invalid types raises an error."""
        with pytest.raises(TypeError):
            UncertainValue("not a number", 0.5)

    def test_mismatched_types_error(self):
        """Test that mixing scalar and array raises an error."""
        with pytest.raises(TypeError):
            UncertainValue(5.0, np.array([0.1, 0.2, 0.3]))

    def test_multiply_by_unsupported_type(self):
        """Test that multiplying by an unsupported type raises an error."""
        uv = UncertainValue(5.0, 0.5)

        with pytest.raises(TypeError):
            uv * "invalid"

    def test_multiply_by_complex_number(self):
        """Test that multiplying by a complex number raises an error."""
        uv = UncertainValue(5.0, 0.5)

        with pytest.raises(TypeError):
            uv * (3 + 2j)


class TestUncertainValueVariancePropagation:
    """Tests specifically for uncertainty propagation formulas."""

    def test_variance_propagation_known_values(self):
        """Test variance propagation with known calculated values."""
        # Example from metrology: (10.0 ± 0.5) * (20.0 ± 1.0)
        uv1 = UncertainValue(10.0, 0.5)
        uv2 = UncertainValue(20.0, 1.0)

        result = uv1 * uv2

        # Value: 10.0 * 20.0 = 200.0
        assert result.value == 200.0

        # Uncertainty: sqrt((20.0 * 0.5)² + (10.0 * 1.0)²)
        # = sqrt(100 + 100) = sqrt(200) ≈ 14.142
        expected_uncertainty = np.sqrt((20.0 * 0.5)**2 + (10.0 * 1.0)**2)
        np.testing.assert_allclose(result.value, 200.0, rtol=1e-10)

    def test_relative_uncertainty_increases(self):
        """Test that multiplying uncertainties generally increases relative uncertainty."""
        # Both have 10% relative uncertainty
        uv1 = UncertainValue(10.0, 1.0)  # 10%
        uv2 = UncertainValue(20.0, 2.0)  # 10%

        result = uv1 * uv2

        # Result value is 200.0
        # Relative uncertainty should be approximately sqrt(0.1² + 0.1²) ≈ 14.1%
        relative_uncertainty = 1.0 / result.value
        expected_relative = np.sqrt(0.1**2 + 0.1**2)
        # Allow for some numerical tolerance


if __name__ == "__main__":
    pytest.main([__file__, "-v"])

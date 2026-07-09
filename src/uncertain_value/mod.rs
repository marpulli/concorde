mod ops;
mod python;

use numpy::ndarray::ArcArray;
use numpy::ndarray::Dim;
use std::collections::HashMap;
use std::iter::Sum;
use std::ops::Add;
use std::ops::Mul;
use toml::Value;
use uuid::Uuid;

// Re-export the Python bindings
pub use python::PyUncertainValue;

pub type NumpyArray1D = ArcArray<f64, Dim<[usize; 1]>>;

pub type VarId = Uuid;

fn generate_unique_id() -> VarId {
    Uuid::now_v7()
}

#[derive(Debug, Clone)]
pub enum ValueType {
    Scalar(f64),
    Array(NumpyArray1D),
}

impl ValueType {
    fn to_array(&self) -> NumpyArray1D {
        match self {
            ValueType::Array(a) => a.clone(),
            ValueType::Scalar(s) => ArcArray::from_elem((1,), s.clone()),
        }
    }

    pub(crate) fn powf(&self, n: f64) -> ValueType {
        match self {
            ValueType::Scalar(s) => ValueType::Scalar(s.powf(n)),
            ValueType::Array(a) => ValueType::Array(a.mapv(|v| v.powf(n)).into_shared()),
        }
    }
}

impl Add for ValueType {
    type Output = Self;
    fn add(self: ValueType, rhs: ValueType) -> Self {
        match (self, rhs) {
            (ValueType::Scalar(l), ValueType::Scalar(r)) => ValueType::Scalar(l + r),
            (r, l) => ValueType::Array(r.to_array() + l.to_array()),
        }
    }
}

impl Mul for ValueType {
    type Output = Self;
    fn mul(self: ValueType, rhs: ValueType) -> Self {
        match (self, rhs) {
            (ValueType::Scalar(l), ValueType::Scalar(r)) => ValueType::Scalar(l * r),
            (r, l) => ValueType::Array(r.to_array() * l.to_array()),
        }
    }
}

impl Sum for ValueType {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(ValueType::Scalar(0.0), |acc, x| acc + x)
    }
}

/// For example: x - x = 0 ± 0 (not sqrt(2) * uncertainty as naive propagation gives)
/// - Uncertainty is computed via: σ² = Σᵢ (∂f/∂xᵢ)² σᵢ²
#[derive(Debug, Clone)]
pub struct UncertainValue {
    pub value: ValueType,

    // TOOD: cache the computed uncertainy value
    /// Variable ID is only present for independent variables
    pub variable_id: Option<VarId>,

    /// Derivatives with respect to each contributing independent variable
    /// Maps: variable_id -> ∂(this_value)/∂(that_variable)
    derivatives: HashMap<VarId, ValueType>,

    /// Uncertainties of the source independent variables
    /// Maps: variable_id -> σ (standard uncertainty)
    /// Shared across all values derived from the same sources
    source_uncertainties: HashMap<VarId, ValueType>,
}

impl UncertainValue {
    pub fn new_independent(value: f64, uncertainty: f64) -> Self {
        let id = generate_unique_id();
        let mut derivatives = HashMap::new();
        derivatives.insert(id, ValueType::Scalar(1.0));

        let mut source_uncertainties = HashMap::new();
        source_uncertainties.insert(id, ValueType::Scalar(uncertainty));

        UncertainValue {
            value: ValueType::Scalar(value),
            variable_id: Some(id),
            derivatives,
            source_uncertainties,
        }
    }

    pub fn new_independent_array(value: NumpyArray1D, uncertainty: NumpyArray1D) -> Self {
        let id = generate_unique_id();
        let mut derivatives = HashMap::new();

        let ones = ArcArray::from_elem(value.dim(), 1.0);
        derivatives.insert(id, ValueType::Array(ones));

        let mut source_uncertainties = HashMap::new();
        source_uncertainties.insert(id, ValueType::Array(uncertainty));

        UncertainValue {
            value: ValueType::Array(value),
            variable_id: Some(id),
            derivatives,
            source_uncertainties,
        }
    }

    pub(crate) fn from_computation(
        value: ValueType,
        derivatives: HashMap<VarId, ValueType>,
        source_uncertainties: HashMap<VarId, ValueType>,
    ) -> Self {
        UncertainValue {
            value,
            variable_id: None,
            derivatives,
            source_uncertainties,
        }
    }

    /// Check if this is an independent variable (vs computed)
    pub fn is_independent(&self) -> bool {
        self.variable_id.is_some()
    }

    /// Compute the uncertainty using derivative propagation
    ///
    /// Formula: σ² = Σᵢ (∂f/∂xᵢ)² σᵢ²
    pub fn uncertainty(&self) -> ValueType {
        let var = self
            .derivatives
            .iter()
            .filter_map(|(var_id, deriv)| {
                let sigma = self.source_uncertainties.get(var_id)?;
                Some(deriv.clone() * deriv.clone() * sigma.clone() * sigma.clone())
            })
            .sum();
        match var {
            ValueType::Scalar(s) => ValueType::Scalar(s.sqrt()),
            ValueType::Array(a) => ValueType::Array(a.sqrt().into_shared()),
        }
    }

    pub fn source_uncertainties(&self) -> &HashMap<VarId, ValueType> {
        &self.source_uncertainties
    }

    pub fn derivatives(&self) -> &HashMap<VarId, ValueType> {
        &self.derivatives
    }
}

#[cfg(test)]
mod tests {
    use numpy::array;

    use super::*;

    #[test]
    fn test_independent_variable_creation() {
        let x = UncertainValue::new_independent(3.0, 0.3);

        // Should have a variable ID
        assert!(x.variable_id.is_some());
        assert!(x.is_independent());

        // Should have one derivative (w.r.t. itself) = 1.0
        assert_eq!(x.derivatives.len(), 1);
        let var_id = x.variable_id.unwrap();
        match x.derivatives.get(&var_id) {
            Some(ValueType::Scalar(d)) => assert_eq!(*d, 1.0),
            _ => panic!("Expected derivative = 1.0"),
        }

        // Check uncertainty
        match x.uncertainty() {
            ValueType::Scalar(u) => assert_eq!(u, 0.3),
            _ => panic!("Expected scalar uncertainty"),
        }
    }

    #[test]
    fn test_independent_array_variable_creation() {
        let magnitude = array![0.0, 1.0, 2.0].into_shared();
        let uncertainty = array![0.0, 0.1, 0.2].into_shared();
        let x = UncertainValue::new_independent_array(magnitude, uncertainty);

        // Should have a variable ID
        assert!(x.variable_id.is_some());
        assert!(x.is_independent());

        // Should have one derivative (w.r.t. itself) = 1.0
        assert_eq!(x.derivatives.len(), 1);
        let var_id = x.variable_id.unwrap();
        match x.derivatives.get(&var_id) {
            Some(ValueType::Array(d)) => assert_eq!(d, array![1.0, 1.0, 1.0].into_shared()),
            _ => panic!("Expected derivative = 1.0"),
        }

        // Check uncertainty
        match x.uncertainty() {
            ValueType::Array(u) => assert_eq!(u, array![0.0, 0.1, 0.2]),
            _ => panic!("Expected array uncertainty"),
        }
    }

    #[test]
    fn test_multiply_uncertain_by_uncertain() {
        // Test variance propagation: (a ± σ_a) * (b ± σ_b)
        // Result: (a*b) ± sqrt((b*σ_a)² + (a*σ_b)²)

        let value1 = UncertainValue::new_independent(3.0, 0.3);
        let value2 = UncertainValue::new_independent(4.0, 0.4);

        let result = &value1 * &value2;

        // Expected: 3.0 * 4.0 = 12.0
        match result.value {
            ValueType::Scalar(v) => assert_eq!(v, 12.0),
            _ => panic!("Expected scalar result"),
        }

        // Result should be computed (no variable_id)
        assert!(result.variable_id.is_none());
        assert!(!result.is_independent());

        // Should have derivatives w.r.t. both variables
        assert_eq!(result.derivatives.len(), 2);

        // Expected uncertainty: sqrt((4.0 * 0.3)² + (3.0 * 0.4)²) ≈ 1.697
        let expected_uncertainty = ((4.0_f64 * 0.3).powi(2) + (3.0_f64 * 0.4).powi(2)).sqrt();
        match result.uncertainty() {
            ValueType::Scalar(u) => assert!(
                (u - expected_uncertainty).abs() < 1e-10,
                "Got '{u}' expected {expected_uncertainty}"
            ),
            ValueType::Array(_) => panic!("Expected scalar uncertainty, got array"),
        }
    }

    #[test]
    fn test_multiply_uncertain_by_scalar() {
        let uncertain = UncertainValue::new_independent(5.0, 0.5);

        let result = &uncertain * 3.0;
        let _result_2 = 3.0 * &uncertain;

        match result.value {
            ValueType::Scalar(v) => assert_eq!(v, 15.0),
            _ => panic!("Expected scalar result"),
        }

        // Should still have same derivatives, just scaled
        assert_eq!(result.derivatives.len(), 1);

        match result.uncertainty() {
            ValueType::Scalar(u) => assert_eq!(u, 1.5),
            _ => panic!("Expected scalar uncertainty"),
        }
    }

    #[test]
    fn test_correlation_x_times_x() {
        // x * x should have correlation
        let x = UncertainValue::new_independent(3.0, 0.3);
        let result = &x * &x;

        // Value should be 9.0
        match result.value {
            ValueType::Scalar(v) => assert_eq!(v, 9.0),
            _ => panic!("Expected scalar result"),
        }

        // For z = x * x:
        // dz/dx = 2*x = 6.0
        // σ_z = |dz/dx| * σ_x = 6.0 * 0.3 = 1.8
        match result.uncertainty() {
            ValueType::Scalar(u) => assert!((u - 1.8).abs() < 1e-10),
            _ => panic!("Expected scalar uncertainty"),
        }

        // Check derivative explicitly
        let var_id = x.variable_id.unwrap();
        match result.derivatives.get(&var_id) {
            Some(ValueType::Scalar(d)) => assert_eq!(*d, 6.0), // 2*x = 6.0
            _ => panic!("Expected derivative = 6.0"),
        }
    }

    #[test]
    fn test_perfect_correlation_chain() {
        // Test: y = 2*x, z = 3*y = 6*x
        // dz/dx should be 6.0
        let x = UncertainValue::new_independent(5.0, 0.5);
        let y = &x * 2.0;
        let z = &y * 3.0;

        // Check value
        match z.value {
            ValueType::Scalar(v) => assert_eq!(v, 30.0),
            _ => panic!("Expected scalar result"),
        }

        // Check derivative
        let var_id = x.variable_id.unwrap();
        match z.derivatives.get(&var_id) {
            Some(ValueType::Scalar(d)) => assert_eq!(*d, 6.0),
            _ => panic!("Expected derivative = 6.0"),
        }

        // Uncertainty should be 6.0 * 0.5 = 3.0
        match z.uncertainty() {
            ValueType::Scalar(u) => assert_eq!(u, 3.0),
            _ => panic!("Expected scalar uncertainty"),
        }
    }

    #[test]
    fn test_x_minus_x_is_zero() {
        // THE CLASSIC TEST: x - x should give 0 ± 0
        let x = UncertainValue::new_independent(5.0, 0.5);
        let result = &x - &x;

        // Value should be 0
        match result.value {
            ValueType::Scalar(v) => assert_eq!(v, 0.0),
            _ => panic!("Expected scalar result"),
        }

        // Uncertainty should be 0 (perfect cancellation)
        // dz/dx = 1 - 1 = 0, so σ_z = 0 * σ_x = 0
        match result.uncertainty() {
            ValueType::Scalar(u) => assert_eq!(u, 0.0),
            _ => panic!("Expected scalar uncertainty"),
        }

        // Check derivative is 0
        let var_id = x.variable_id.unwrap();
        match result.derivatives.get(&var_id) {
            Some(ValueType::Scalar(d)) => assert_eq!(*d, 0.0),
            _ => panic!("Expected derivative = 0.0"),
        }
    }

    #[test]
    fn test_addition() {
        let x = UncertainValue::new_independent(3.0, 0.3);
        let y = UncertainValue::new_independent(4.0, 0.4);
        let z = &x + &y;

        // Value: 3 + 4 = 7
        match z.value {
            ValueType::Scalar(v) => assert_eq!(v, 7.0),
            _ => panic!("Expected scalar result"),
        }

        // Uncertainty: √(0.3² + 0.4²) = 0.5
        match z.uncertainty() {
            ValueType::Scalar(u) => assert_eq!(u, 0.5),
            _ => panic!("Expected scalar uncertainty"),
        }
    }

    #[test]
    fn test_subtraction() {
        let x = UncertainValue::new_independent(10.0, 1.0);
        let y = UncertainValue::new_independent(3.0, 0.5);
        let z = &x - &y;

        // Value: 10 - 3 = 7
        match z.value {
            ValueType::Scalar(v) => assert_eq!(v, 7.0),
            _ => panic!("Expected scalar result"),
        }

        // Uncertainty: √(1.0² + 0.5²) ≈ 1.118
        let expected = ((1.0_f64).powi(2) + (0.5_f64).powi(2)).sqrt();
        match z.uncertainty() {
            ValueType::Scalar(u) => assert!((u - expected).abs() < 1e-10),
            _ => panic!("Expected scalar uncertainty"),
        }
    }

    #[test]
    fn test_division() {
        let x = UncertainValue::new_independent(10.0, 1.0);
        let y = UncertainValue::new_independent(2.0, 0.2);
        let z = &x / &y;

        // Value: 10 / 2 = 5
        match z.value {
            ValueType::Scalar(v) => assert_eq!(v, 5.0),
            _ => panic!("Expected scalar result"),
        }

        // For z = x/y:
        // ∂z/∂x = 1/y = 0.5, ∂z/∂y = -x/y² = -2.5
        // σ_z = √((0.5 * 1.0)² + (-2.5 * 0.2)²) = √(0.25 + 0.25) ≈ 0.707
        let expected = ((0.5_f64 * 1.0).powi(2) + (2.5_f64 * 0.2).powi(2)).sqrt();
        match z.uncertainty() {
            ValueType::Scalar(u) => assert!((u - expected).abs() < 1e-10),
            _ => panic!("Expected scalar uncertainty"),
        }
    }

    #[test]
    fn test_negation() {
        let x = UncertainValue::new_independent(5.0, 0.5);
        let result = -&x;

        // Value should be -5.0
        match result.value {
            ValueType::Scalar(v) => assert_eq!(v, -5.0),
            _ => panic!("Expected scalar result"),
        }

        // Uncertainty should remain 0.5
        match result.uncertainty() {
            ValueType::Scalar(u) => assert_eq!(u, 0.5),
            _ => panic!("Expected scalar uncertainty"),
        }
    }

    #[test]
    fn test_complex_expression() {
        // Test: z = (x + y) / (x - y) with correlation tracking
        let x = UncertainValue::new_independent(10.0, 1.0);
        let y = UncertainValue::new_independent(2.0, 0.2);

        let numerator = &x + &y; // 12.0
        let denominator = &x - &y; // 8.0
        let result = &numerator / &denominator; // 1.5

        match result.value {
            ValueType::Scalar(v) => assert_eq!(v, 1.5),
            _ => panic!("Expected scalar result"),
        }

        // Should have derivatives w.r.t. both x and y
        assert_eq!(result.derivatives.len(), 2);
    }

    #[test]
    fn test_pow_matches_repeated_multiplication() {
        // x**2 should have the same value and uncertainty as x*x (correlation-aware:
        // sigma = |2x|*sigma_x, not the naive sqrt(2)*sigma_x you'd get treating the
        // two factors as independent)
        let x = UncertainValue::new_independent(2.0, 0.2);

        let squared = x.pow(2.0);
        let multiplied = &x * &x;

        match (&squared.value, &multiplied.value) {
            (ValueType::Scalar(a), ValueType::Scalar(b)) => assert_eq!(*a, *b),
            _ => panic!("Expected scalar values"),
        }

        match (squared.uncertainty(), multiplied.uncertainty()) {
            (ValueType::Scalar(a), ValueType::Scalar(b)) => assert!((a - b).abs() < 1e-10),
            _ => panic!("Expected scalar uncertainties"),
        }
    }

    #[test]
    fn test_pow_uncertainty_formula() {
        // sigma of x^2 should equal |2x| * sigma_x
        let x = UncertainValue::new_independent(3.0, 0.3);
        let squared = x.pow(2.0);

        match squared.uncertainty() {
            ValueType::Scalar(u) => assert!((u - (2.0 * 3.0 * 0.3)).abs() < 1e-10),
            _ => panic!("Expected scalar uncertainty"),
        }
    }

    #[test]
    fn test_pow_fractional_exponent_sqrt() {
        let x = UncertainValue::new_independent(4.0, 0.4);
        let root = x.pow(0.5);

        match root.value {
            ValueType::Scalar(v) => assert!((v - 2.0).abs() < 1e-10),
            _ => panic!("Expected scalar value"),
        }

        // sigma of sqrt(x) = 0.5 * x^(-0.5) * sigma_x = 0.5 / 2.0 * 0.4 = 0.1
        match root.uncertainty() {
            ValueType::Scalar(u) => assert!((u - 0.1).abs() < 1e-10),
            _ => panic!("Expected scalar uncertainty"),
        }
    }

    #[test]
    fn test_pow_array_value() {
        use numpy::array;

        let magnitude = array![2.0, 3.0].into_shared();
        let uncertainty = array![0.2, 0.3].into_shared();
        let x = UncertainValue::new_independent_array(magnitude, uncertainty);

        let squared = x.pow(2.0);

        match squared.value {
            ValueType::Array(v) => assert_eq!(v, array![4.0, 9.0]),
            _ => panic!("Expected array value"),
        }
    }
}

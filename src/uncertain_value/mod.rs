mod ops;
mod python;

use numpy::ndarray::ArcArray;
use numpy::ndarray::Dim;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

// Re-export the Python bindings
pub use python::PyUncertainValue;

// Type alias for 1D numpy arrays
pub type NumpyArray = ArcArray<f64, Dim<[usize; 1]>>;

// Unique identifier for independent random variables
pub type VarId = usize;

// Global counter for generating unique variable IDs
static NEXT_VAR_ID: AtomicUsize = AtomicUsize::new(1);

fn generate_unique_id() -> VarId {
    NEXT_VAR_ID.fetch_add(1, Ordering::SeqCst)
}

// Enum to represent either scalar or array values
#[derive(Debug, Clone)]
pub enum ValueType {
    Scalar(f64),
    Array(NumpyArray),
}

/// UncertainValue with automatic correlation tracking via derivative propagation
///
/// This implementation tracks derivatives with respect to all independent random
/// variables that contribute to this value, enabling proper correlation handling.
/// For example: x - x = 0 ± 0 (not sqrt(2) * uncertainty as naive propagation gives)
///
/// # Design:
/// - Independent variables have a `variable_id` (Some(id)) and derivatives = {id: 1.0}
/// - Computed values have `variable_id = None` and derivatives track all contributors
/// - Uncertainty is computed via: σ² = Σᵢ (∂f/∂xᵢ)² σᵢ²
#[derive(Debug, Clone)]
pub struct UncertainValue {
    /// The central value (scalar or array)
    pub value: ValueType,

    /// Variable ID - present only for independent variables
    /// Independent: Some(id), Computed: None
    pub variable_id: Option<VarId>,

    /// Derivatives with respect to each contributing independent variable
    /// Maps: variable_id -> ∂(this_value)/∂(that_variable)
    /// For independent variable with id=k: {k: 1.0}
    /// For computed values: {contributing_ids: computed_derivatives}
    pub derivatives: HashMap<VarId, ValueType>,

    /// Uncertainties of the source independent variables
    /// Maps: variable_id -> σ (standard uncertainty)
    /// Shared across all values derived from the same sources
    pub source_uncertainties: HashMap<VarId, f64>,
}

impl UncertainValue {
    /// Create a new independent random variable with a unique ID
    ///
    /// # Arguments
    /// * `value` - The central value
    /// * `uncertainty` - The standard uncertainty (σ)
    ///
    /// # Returns
    /// An independent UncertainValue with derivative = 1.0 w.r.t. itself
    pub fn new_independent(value: f64, uncertainty: f64) -> Self {
        let id = generate_unique_id();
        let mut derivatives = HashMap::new();
        derivatives.insert(id, ValueType::Scalar(1.0));

        let mut source_uncertainties = HashMap::new();
        source_uncertainties.insert(id, uncertainty);

        UncertainValue {
            value: ValueType::Scalar(value),
            variable_id: Some(id),
            derivatives,
            source_uncertainties,
        }
    }

    /// Create a new independent random variable with array values
    ///
    /// # Arguments
    /// * `value` - Array of central values
    /// * `uncertainty` - Array of standard uncertainties
    pub fn new_independent_array(value: NumpyArray, uncertainty: NumpyArray) -> Self {
        let id = generate_unique_id();
        let mut derivatives = HashMap::new();

        // For array independent variables, derivative is an array of ones
        let ones = ArcArray::from_elem(value.dim(), 1.0);
        derivatives.insert(id, ValueType::Array(ones));

        // For arrays, we store the uncertainty array as a single "source"
        // The actual per-element uncertainties are in the array itself
        // We'll need to handle this carefully in uncertainty computation
        let mut source_uncertainties = HashMap::new();
        source_uncertainties.insert(id, 1.0); // Placeholder, actual uncertainty is in the array

        UncertainValue {
            value: ValueType::Array(value),
            variable_id: Some(id),
            derivatives,
            source_uncertainties,
        }
    }

    /// Create a computed value (internal use)
    ///
    /// # Arguments
    /// * `value` - The computed value
    /// * `derivatives` - Derivatives w.r.t. contributing variables
    /// * `source_uncertainties` - Uncertainties of source variables
    pub(crate) fn from_computation(
        value: ValueType,
        derivatives: HashMap<VarId, ValueType>,
        source_uncertainties: HashMap<VarId, f64>,
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
        match &self.value {
            ValueType::Scalar(_) => {
                let variance: f64 = self.derivatives.iter()
                    .filter_map(|(var_id, deriv)| {
                        let sigma = self.source_uncertainties.get(var_id)?;
                        match deriv {
                            ValueType::Scalar(d) => Some(d * d * sigma * sigma),
                            _ => None,
                        }
                    })
                    .sum();
                ValueType::Scalar(variance.sqrt())
            }
            ValueType::Array(_) => {
                // For arrays, we need element-wise computation
                // This is more complex and will be implemented as needed
                todo!("Array uncertainty computation not yet implemented")
            }
        }
    }

    /// Legacy constructor for backward compatibility
    /// Creates an independent variable
    #[deprecated(note = "Use new_independent instead")]
    pub fn new_scalar(value: f64, uncertainty: f64) -> Self {
        Self::new_independent(value, uncertainty)
    }

    /// Legacy constructor for backward compatibility
    /// Creates an independent variable array
    #[deprecated(note = "Use new_independent_array instead")]
    pub fn new_array(value: NumpyArray, uncertainty: NumpyArray) -> Self {
        Self::new_independent_array(value, uncertainty)
    }
}

#[cfg(test)]
mod tests {
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

        // Expected uncertainty: sqrt((4.0 * 0.3)² + (3.0 * 0.4)²)
        // = sqrt(1.44 + 1.44) = sqrt(2.88) ≈ 1.697
        let expected_uncertainty = ((4.0_f64 * 0.3).powi(2) + (3.0_f64 * 0.4).powi(2)).sqrt();
        match result.uncertainty() {
            ValueType::Scalar(u) => assert!((u - expected_uncertainty).abs() < 1e-10),
            _ => panic!("Expected scalar uncertainty"),
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

        let numerator = &x + &y;    // 12.0
        let denominator = &x - &y;  // 8.0
        let result = &numerator / &denominator;  // 1.5

        match result.value {
            ValueType::Scalar(v) => assert_eq!(v, 1.5),
            _ => panic!("Expected scalar result"),
        }

        // Should have derivatives w.r.t. both x and y
        assert_eq!(result.derivatives.len(), 2);
    }
}

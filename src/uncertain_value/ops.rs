use super::{UncertainValue, ValueType, VarId};
use std::collections::HashMap;
use std::ops::{Add, Div, Mul, Neg, Sub};

// ============================================================================
// Generic Binary Operation Framework
// ============================================================================

/// Generic function to apply binary operations with derivative propagation
/// using the chain rule.
///
/// For operation z = f(a, b), this computes:
/// - value: z = f(a, b)
/// - derivatives: ∂z/∂x = (∂f/∂a)(∂a/∂x) + (∂f/∂b)(∂b/∂x)
///
/// # Type Parameters
/// All three function parameters follow the same signature pattern:
/// `Fn(&ValueType, &ValueType) -> ValueType`
///
/// - `F`: Function to compute the result value z = f(a, b)
/// - `DA`: Function to compute ∂f/∂a (partial derivative w.r.t. first argument)
/// - `DB`: Function to compute ∂f/∂b (partial derivative w.r.t. second argument)
///

fn binary_op<F, DA, DB>(
    a: &UncertainValue,
    b: &UncertainValue,
    value_fn: F,
    deriv_a_fn: DA,
    deriv_b_fn: DB,
) -> UncertainValue
where
    F: Fn(&ValueType, &ValueType) -> ValueType,
    DA: Fn(&ValueType, &ValueType) -> ValueType,
    DB: Fn(&ValueType, &ValueType) -> ValueType,
{
    // Compute result value
    let value = value_fn(&a.value, &b.value);

    // Merge source uncertainties from both operands
    let mut source_uncertainties = a.source_uncertainties.clone();
    source_uncertainties.extend(
        b.source_uncertainties()
            .iter()
            .map(|(k, v)| (*k, v.clone())),
    );

    // Compute derivatives using chain rule: ∂z/∂x = (∂f/∂a)(∂a/∂x) + (∂f/∂b)(∂b/∂x)
    let mut new_derivatives: HashMap<VarId, ValueType> = HashMap::new();

    // Get partial derivatives ∂f/∂a and ∂f/∂b
    let df_da = deriv_a_fn(&a.value, &b.value);
    let df_db = deriv_b_fn(&a.value, &b.value);

    // Collect all variable IDs from both operands
    let all_vars: std::collections::HashSet<_> =
        a.derivatives.keys().chain(b.derivatives.keys()).collect();

    for var_id in all_vars {
        let da_dx = a.derivatives.get(var_id);
        let db_dx = b.derivatives.get(var_id);

        let derivative = match (da_dx, db_dx) {
            (Some(da), Some(db)) => {
                // Both depend on this variable: ∂z/∂x = (∂f/∂a)(∂a/∂x) + (∂f/∂b)(∂b/∂x)
                add_value_types(&mul_value_types(&df_da, da), &mul_value_types(&df_db, db))
            }
            (Some(da), None) => {
                // Only 'a' depends on this variable: ∂z/∂x = (∂f/∂a)(∂a/∂x)
                mul_value_types(&df_da, da)
            }
            (None, Some(db)) => {
                // Only 'b' depends on this variable: ∂z/∂x = (∂f/∂b)(∂b/∂x)
                mul_value_types(&df_db, db)
            }
            (None, None) => continue, // Should never happen
        };

        new_derivatives.insert(*var_id, derivative);
    }

    UncertainValue::from_computation(value, new_derivatives, source_uncertainties)
}

// ============================================================================
// Helper Functions for ValueType Operations
// ============================================================================

/// Helper: Multiply two ValueTypes (scalar * scalar, array * array, etc.)
fn mul_value_types(a: &ValueType, b: &ValueType) -> ValueType {
    match (a, b) {
        (ValueType::Scalar(x), ValueType::Scalar(y)) => ValueType::Scalar(x * y),
        (l, r) => ValueType::Array(l.to_array() * r.to_array()),
    }
}

/// Helper: Add two ValueTypes
fn add_value_types(a: &ValueType, b: &ValueType) -> ValueType {
    match (a, b) {
        (ValueType::Scalar(x), ValueType::Scalar(y)) => ValueType::Scalar(x + y),
        (l, r) => ValueType::Array(l.to_array() + r.to_array()),
    }
}

/// Helper: Subtract two ValueTypes
fn sub_value_types(a: &ValueType, b: &ValueType) -> ValueType {
    match (a, b) {
        (ValueType::Scalar(x), ValueType::Scalar(y)) => ValueType::Scalar(x - y),
        (l, r) => ValueType::Array(l.to_array() - r.to_array()),
    }
}

/// Helper: Divide two ValueTypes
fn div_value_types(a: &ValueType, b: &ValueType) -> ValueType {
    match (a, b) {
        (ValueType::Scalar(x), ValueType::Scalar(y)) => ValueType::Scalar(x / y),
        (l, r) => ValueType::Array(l.to_array() / r.to_array()),
    }
}

// ============================================================================
// Addition: z = a + b
// Derivatives: ∂z/∂a = 1, ∂z/∂b = 1
// ============================================================================

impl Add<&UncertainValue> for &UncertainValue {
    type Output = UncertainValue;

    fn add(self, other: &UncertainValue) -> Self::Output {
        binary_op(
            self,
            other,
            |a, b| add_value_types(a, b),   // z = a + b
            |a, _b| ValueType::Scalar(1.0), // ∂z/∂a = 1
            |_a, b| ValueType::Scalar(1.0), // ∂z/∂b = 1
        )
    }
}

// ============================================================================
// Subtraction: z = a - b
// Derivatives: ∂z/∂a = 1, ∂z/∂b = -1
// ============================================================================

impl Sub<&UncertainValue> for &UncertainValue {
    type Output = UncertainValue;

    fn sub(self, other: &UncertainValue) -> Self::Output {
        binary_op(
            self,
            other,
            |a, b| sub_value_types(a, b),    // z = a - b
            |a, _b| ValueType::Scalar(1.0),  // ∂z/∂a = 1
            |_a, b| ValueType::Scalar(-1.0), // ∂z/∂b = -1
        )
    }
}

// ============================================================================
// Multiplication: z = a * b
// Derivatives: ∂z/∂a = b, ∂z/∂b = a
// ============================================================================

impl Mul<&UncertainValue> for &UncertainValue {
    type Output = UncertainValue;

    fn mul(self, other: &UncertainValue) -> Self::Output {
        match self.uncertainty() {
            ValueType::Scalar(s) => println!("Self: {s}"),
            _ => (),
        }

        match other.uncertainty() {
            ValueType::Scalar(s) => println!("Other: {s}"),
            _ => (),
        }

        let res = binary_op(
            self,
            other,
            |a, b| mul_value_types(a, b), // z = a * b
            |_a, b| b.clone(),            // ∂z/∂a = b
            |a, _b| a.clone(),            // ∂z/∂b = a
        );
        match res.uncertainty() {
            ValueType::Scalar(s) => println!("Result: {s}"),
            _ => (),
        }
        for (k, v) in res.derivatives.iter() {
            match v {
                ValueType::Scalar(s) => println!("{k}: {s}"),
                _ => (),
            }
        }
        res
    }
}

// ============================================================================
// Division: z = a / b
// Derivatives: ∂z/∂a = 1/b, ∂z/∂b = -a/b²
// ============================================================================

impl Div<&UncertainValue> for &UncertainValue {
    type Output = UncertainValue;

    fn div(self, other: &UncertainValue) -> Self::Output {
        binary_op(
            self,
            other,
            |a, b| div_value_types(a, b), // z = a / b
            |_a, b| {
                div_value_types(
                    // ∂z/∂a = 1/b
                    &ValueType::Scalar(1.0),
                    b,
                )
            },
            |a, b| {
                // ∂z/∂b = -a/b²
                let b_squared = mul_value_types(b, b);
                let a_over_b2 = div_value_types(a, &b_squared);
                mul_value_types(&a_over_b2, &ValueType::Scalar(-1.0))
            },
        )
    }
}

// ============================================================================
// Scalar Multiplication: z = a * scalar
// Derivatives: ∂z/∂x = scalar * ∂a/∂x
// ============================================================================

impl Mul<f64> for &UncertainValue {
    type Output = UncertainValue;

    fn mul(self, scalar: f64) -> Self::Output {
        let value = mul_value_types(&self.value, &ValueType::Scalar(scalar));

        // For multiplication by scalar: d(a*c)/dx = c * da/dx
        let new_derivatives: HashMap<VarId, ValueType> = self
            .derivatives
            .iter()
            .map(|(var_id, deriv)| (*var_id, mul_value_types(deriv, &ValueType::Scalar(scalar))))
            .collect();

        UncertainValue::from_computation(value, new_derivatives, self.source_uncertainties.clone())
    }
}

// Implement Mul for &UncertainValue * &f64
impl Mul<&f64> for &UncertainValue {
    type Output = UncertainValue;

    fn mul(self, scalar: &f64) -> Self::Output {
        self * (*scalar)
    }
}

// Implement Mul for f64 * &UncertainValue
impl Mul<&UncertainValue> for f64 {
    type Output = UncertainValue;

    fn mul(self, uncertain: &UncertainValue) -> Self::Output {
        uncertain * self
    }
}

// ============================================================================
// Scalar addition
// ============================================================================
impl Add<&f64> for &UncertainValue {
    type Output = UncertainValue;

    fn add(self, scalar: &f64) -> Self::Output {
        self + &UncertainValue::new_independent(*scalar, 0.0)
    }
}

impl Add<&UncertainValue> for &f64 {
    type Output = UncertainValue;

    fn add(self, uncertain_value: &UncertainValue) -> Self::Output {
        uncertain_value + self
    }
}

// ============================================================================
// Negation: z = -a
// Derivative: ∂z/∂x = -∂a/∂x
// ============================================================================

impl Neg for &UncertainValue {
    type Output = UncertainValue;

    fn neg(self) -> Self::Output {
        self * -1.0
    }
}

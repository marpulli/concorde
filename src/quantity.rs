use crate::uncertain_value::UncertainValue;
use crate::unit::Unit;
use std::ops::Mul;

// Marker trait for types that can be used in Quantity
// This is a simple marker - the actual multiplication constraints
// are specified in the impl blocks that need them
pub trait QuantityValue {}

// Implement for f64
impl QuantityValue for f64 {}

// Implement for UncertainValue
impl QuantityValue for UncertainValue {}

pub struct Quantity<T>
where
    T: QuantityValue,
{
    value: T,
    unit: Unit,
}

impl<T> Quantity<T>
where
    T: QuantityValue,
{
    pub fn new(value: T, unit: Unit) -> Self {
        Quantity { value, unit }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn unit(&self) -> &Unit {
        &self.unit
    }
}

// Can multiply two quantities together
impl<T, U, Output> Mul<&Quantity<U>> for &Quantity<T>
where
    T: QuantityValue,
    U: QuantityValue,
    for<'a, 'b> &'a T: Mul<&'b U, Output = Output>,
    Output: QuantityValue,
{
    type Output = Quantity<Output>;

    fn mul(self, other: &Quantity<U>) -> Self::Output {
        Quantity {
            value: &self.value * &other.value,
            unit: &self.unit * &other.unit,
        }
    }
}

// Can multiply by a scalar
impl<T> Mul<f64> for &Quantity<T>
where
    T: QuantityValue,
    for<'a> &'a T: Mul<f64, Output = T>,
{
    type Output = Quantity<T>;

    fn mul(self, other: f64) -> Self::Output {
        Quantity {
            value: &self.value * other,
            unit: self.unit.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multiply_uncertain_by_float() {
        // Create a simple unit for testing
        use crate::unit::DefinedUnit;
        use std::collections::HashMap;
        use std::sync::Arc;

        let kg_unit = Arc::new(DefinedUnit::new(
            "kg".to_string(),
            [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            1.0,
            vec![],
        ));

        let m_unit = Arc::new(DefinedUnit::new(
            "m".to_string(),
            [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            1.0,
            vec![],
        ));

        let kg = Unit::new(HashMap::from([(kg_unit, 1.0)]));
        let m = Unit::new(HashMap::from([(m_unit, 1.0)]));

        // Create an uncertain quantity: 5.0 ± 0.5 kg
        #[allow(deprecated)]
        let uncertain_value = UncertainValue::new_scalar(5.0, 0.5);
        let uncertain_qty = Quantity::new(uncertain_value, kg);

        // Create a float quantity: 2.0 m
        let float_qty = Quantity::new(2.0, m);

        // Multiply: (5.0 ± 0.5 kg) * (2.0 m) = (10.0 ± 1.0 kg*m)
        let result = &uncertain_qty * &float_qty;

        use crate::uncertain_value::ValueType;
        match &result.value.value {
            ValueType::Scalar(v) => assert_eq!(*v, 10.0),
            _ => panic!("Expected scalar result"),
        }

        match result.value.uncertainty() {
            ValueType::Scalar(u) => assert_eq!(u, 1.0),
            _ => panic!("Expected scalar uncertainty"),
        }
    }
}

use crate::uncertain_value::UncertainValue;
use crate::unit::{IncompatibleUnitsError, Unit};
use std::ops::{Add, Div, Mul, Sub};

// Marker trait for types that can be used in Quantity
// This is a simple marker - the actual multiplication constraints
// are specified in the impl blocks that need them
pub trait QuantityValue {}

// Implement for f64
impl QuantityValue for f64 {}

// Implement for UncertainValue
impl QuantityValue for UncertainValue {}

/// Types that support raising to a scalar power.
pub trait Powf {
    fn powf(&self, n: f64) -> Self;
}

impl Powf for f64 {
    fn powf(&self, n: f64) -> f64 {
        f64::powf(*self, n)
    }
}

impl Powf for UncertainValue {
    fn powf(&self, n: f64) -> UncertainValue {
        self.pow(n)
    }
}

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

impl<T> Quantity<T>
where
    T: QuantityValue,
    for<'a> &'a T: Mul<f64, Output = T>,
{
    /// Convert this quantity to an equivalent value expressed in `target` units.
    /// Returns an error if `target` is not dimensionally compatible.
    pub fn to(&self, target: &Unit) -> Result<Quantity<T>, IncompatibleUnitsError> {
        let factor = target.conversion_factor(&self.unit)?;
        Ok(Quantity {
            value: &self.value * factor,
            unit: target.clone(),
        })
    }
}

impl<T> Quantity<T>
where
    T: QuantityValue + Powf,
{
    /// Raise both the value and the unit to a scalar power.
    pub fn pow(&self, n: f64) -> Quantity<T> {
        Quantity {
            value: self.value.powf(n),
            unit: self.unit.pow(n),
        }
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

// Can add two quantities with compatible units
impl<T> Add<&Quantity<T>> for &Quantity<T>
where
    T: QuantityValue,
    for<'a, 'b> &'a T: Add<&'b T, Output = T>,
    for<'a> &'a T: Mul<f64, Output = T>,
{
    type Output = Result<Quantity<T>, IncompatibleUnitsError>;

    fn add(self, other: &Quantity<T>) -> Self::Output {
        let factor = self.unit.conversion_factor(&other.unit)?;

        let converted = &other.value * factor;
        Ok(Quantity {
            value: &self.value + &converted,
            unit: self.unit.clone(),
        })
    }
}

// Can subtract two quantities with compatible units
impl<T> Sub<&Quantity<T>> for &Quantity<T>
where
    T: QuantityValue,
    for<'a, 'b> &'a T: Sub<&'b T, Output = T>,
    for<'a> &'a T: Mul<f64, Output = T>,
{
    type Output = Result<Quantity<T>, IncompatibleUnitsError>;

    fn sub(self, other: &Quantity<T>) -> Self::Output {
        let factor = self.unit.conversion_factor(&other.unit)?;

        let converted = &other.value * factor;
        Ok(Quantity {
            value: &self.value - &converted,
            unit: self.unit.clone(),
        })
    }
}

// Can divide two quantities
impl<T, U, Output> Div<&Quantity<U>> for &Quantity<T>
where
    T: QuantityValue,
    U: QuantityValue,
    for<'a, 'b> &'a T: Div<&'b U, Output = Output>,
    Output: QuantityValue,
{
    type Output = Quantity<Output>;

    fn div(self, other: &Quantity<U>) -> Self::Output {
        Quantity {
            value: &self.value / &other.value,
            unit: &self.unit / &other.unit,
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
    fn test_add_quantities_same_unit() {
        use crate::unit::DefinedUnit;
        use std::collections::HashMap;
        use std::sync::Arc;

        let kg_unit = Arc::new(DefinedUnit::new(
            "kg".to_string(),
            [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            1.0,
            vec![],
        ));

        let kg = Unit::new(HashMap::from([(kg_unit.clone(), 1.0)]));

        let a = Quantity::new(UncertainValue::new_independent(5.0, 0.3), kg.clone());
        let b = Quantity::new(UncertainValue::new_independent(3.0, 0.4), kg);

        let result = (&a + &b).unwrap();

        use crate::uncertain_value::ValueType;
        match &result.value().value {
            ValueType::Scalar(v) => assert_eq!(*v, 8.0),
            _ => panic!("Expected scalar result"),
        }

        // Uncertainty: sqrt(0.3^2 + 0.4^2) = 0.5
        match result.value().uncertainty() {
            ValueType::Scalar(u) => assert!((u - 0.5).abs() < 1e-10),
            _ => panic!("Expected scalar uncertainty"),
        }
    }

    #[test]
    fn test_add_quantities_incompatible_units() {
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

        let a = Quantity::new(UncertainValue::new_independent(5.0, 0.3), kg);
        let b = Quantity::new(UncertainValue::new_independent(3.0, 0.4), m);

        let result = &a + &b;
        assert!(result.is_err());
    }

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
        let uncertain_value = UncertainValue::new_independent(5.0, 0.5);
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

    fn km_and_m_units() -> (Unit, Unit) {
        use crate::unit::DefinedUnit;
        use std::collections::HashMap;
        use std::sync::Arc;

        let m_unit = Arc::new(DefinedUnit::new(
            "m".to_string(),
            [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            1.0,
            vec![],
        ));
        let km_unit = Arc::new(DefinedUnit::new(
            "km".to_string(),
            [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            1000.0,
            vec![],
        ));

        let m = Unit::new(HashMap::from([(m_unit, 1.0)]));
        let km = Unit::new(HashMap::from([(km_unit, 1.0)]));
        (km, m)
    }

    #[test]
    fn test_to_converts_scalar_value_and_uncertainty() {
        use crate::uncertain_value::ValueType;

        let (km, m) = km_and_m_units();
        let d = Quantity::new(UncertainValue::new_independent(1.5, 0.1), km);

        let converted = d.to(&m).unwrap();

        match &converted.value().value {
            ValueType::Scalar(v) => assert_eq!(*v, 1500.0),
            _ => panic!("Expected scalar value"),
        }
        match converted.value().uncertainty() {
            ValueType::Scalar(u) => assert!((u - 100.0).abs() < 1e-10),
            _ => panic!("Expected scalar uncertainty"),
        }
        assert_eq!(converted.unit().to_string(), "m");
    }

    #[test]
    fn test_to_round_trip() {
        use crate::uncertain_value::ValueType;

        let (km, m) = km_and_m_units();
        let d = Quantity::new(UncertainValue::new_independent(1.5, 0.1), km.clone());

        let round_tripped = d.to(&m).unwrap().to(&km).unwrap();

        match &round_tripped.value().value {
            ValueType::Scalar(v) => assert!((v - 1.5).abs() < 1e-10),
            _ => panic!("Expected scalar value"),
        }
    }

    #[test]
    fn test_to_incompatible_units_errors() {
        use crate::unit::DefinedUnit;
        use std::collections::HashMap;
        use std::sync::Arc;

        let (km, _) = km_and_m_units();
        let s_unit = Arc::new(DefinedUnit::new(
            "s".to_string(),
            [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
            1.0,
            vec![],
        ));
        let s = Unit::new(HashMap::from([(s_unit, 1.0)]));

        let d = Quantity::new(UncertainValue::new_independent(1.5, 0.1), km);
        assert!(d.to(&s).is_err());
    }

    #[test]
    fn test_quantity_pow() {
        use crate::uncertain_value::ValueType;
        use crate::unit::DefinedUnit;
        use std::collections::HashMap;
        use std::sync::Arc;

        let m_unit = Arc::new(DefinedUnit::new(
            "m".to_string(),
            [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            1.0,
            vec![],
        ));
        let m = Unit::new(HashMap::from([(m_unit, 1.0)]));

        let length = Quantity::new(UncertainValue::new_independent(3.0, 0.3), m);
        let area = length.pow(2.0);

        match &area.value().value {
            ValueType::Scalar(v) => assert_eq!(*v, 9.0),
            _ => panic!("Expected scalar value"),
        }
        assert_eq!(area.unit().get_exponent(&"m".to_string()), Some(2.0));
    }
}

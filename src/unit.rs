use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::{ops::Div, ops::Mul};

#[derive(Debug, Clone)]
pub enum UnitKind {
    Linear,
    Delta,
    Affine {
        offset: f64,
        delta: Arc<DefinedUnit>,
    },
}

#[derive(Debug, Clone)]
pub enum UnitError {
    Incompatible(IncompatibleUnitsError),
    Affine {
        operation: &'static str,
        unit: String,
    },
}

impl fmt::Display for UnitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Incompatible(e) => e.fmt(f),
            Self::Affine { operation, unit } => write!(
                f,
                "cannot perform {operation} with affine unit {unit}; use an explicit delta unit for differences"
            ),
        }
    }
}

impl std::error::Error for UnitError {}

/// Numeric transform: target = source * scale + offset.
#[derive(Debug, Clone, Copy)]
pub struct Conversion {
    pub scale: f64,
    pub offset: f64,
}

impl Conversion {
    pub const IDENTITY: Self = Self {
        scale: 1.0,
        offset: 0.0,
    };

    pub fn apply<T>(&self, value: &T) -> T
    where
        for<'a> &'a T: Mul<f64, Output = T>,
        for<'a, 'b> &'a T: std::ops::Add<&'b f64, Output = T>,
    {
        let scaled = value * self.scale;
        if self.offset == 0.0 {
            scaled
        } else {
            &scaled + &self.offset
        }
    }
}

/// Convert both operands, perform the requested numeric operation, then attach `result`.
/// The result unit can differ from the units used to align the operands (absolute - absolute).
pub struct BinaryOperation {
    pub lhs: Conversion,
    pub rhs: Conversion,
    pub result: Unit,
}

#[derive(Debug, Clone)]
pub struct Unit {
    // TODO: consider using rational numbers for exponent and strong type for the unit
    pub(crate) components: std::collections::HashMap<Arc<DefinedUnit>, f64>,
}

// Dimensions are: Length, Mass, Time, ELectric Current, temperature, amount of substance, luminous intensity

#[derive(Debug, Clone)]
pub struct DefinedUnit {
    pub name: String,
    pub dimensions: [f64; 7],
    pub scale: f64,
    pub kind: UnitKind,
    aliases: Vec<String>,
}

// Implement PartialEq and Eq based on name only
// This is valid because name uniqueness is guaranteed elsewhere
impl PartialEq for DefinedUnit {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for DefinedUnit {}

// Implement Hash based on name only to match the equality implementation
impl std::hash::Hash for DefinedUnit {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl DefinedUnit {
    pub fn new(
        name: String,
        dimensions: [f64; 7],
        scale: f64,
        aliases: Vec<String>,
    ) -> DefinedUnit {
        DefinedUnit {
            name: name,
            dimensions: dimensions,
            scale: scale,
            kind: UnitKind::Linear,
            aliases: aliases,
        }
    }

    /// Offsets are in coherent base units: base = value * scale + offset.
    pub fn with_offset(mut self, offset: f64) -> Self {
        let delta = DefinedUnit {
            name: format!("delta_{}", self.name),
            dimensions: self.dimensions,
            scale: self.scale,
            kind: UnitKind::Delta,
            aliases: self.aliases.iter().map(|a| format!("delta_{a}")).collect(),
        };
        self.kind = UnitKind::Affine {
            offset,
            delta: Arc::new(delta),
        };
        self
    }

    pub fn aliases(&self) -> &Vec<String> {
        &self.aliases
    }
}

impl fmt::Display for DefinedUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.components.is_empty() {
            return write!(f, "dimensionless");
        }

        let mut parts: Vec<_> = self
            .components
            .iter()
            .map(|(unit, &exp)| {
                if exp == 1.0 {
                    unit.name.clone()
                } else {
                    format!("{}^{}", unit.name, exp)
                }
            })
            .collect();
        parts.sort(); // deterministic output
        write!(f, "{}", parts.join(" * "))
    }
}

impl Unit {
    /// Internal constructor. User expressions must use checked unit operations.
    pub fn new(components: HashMap<Arc<DefinedUnit>, f64>) -> Unit {
        Unit {
            components: components
                .into_iter()
                .filter(|(_, power)| *power != 0.0)
                .collect(),
        }
    }

    pub fn pow(&self, power: f64) -> Result<Unit, UnitError> {
        self.check_multiplicative("exponentiation")?;
        Ok(Unit::new(
            self.components
                .iter()
                .map(|(k, v)| (k.clone(), v * power))
                .collect(),
        ))
    }

    pub fn get_exponent(&self, unit_name: &String) -> Option<f64> {
        self.components
            .iter()
            .find(|(defined_unit, _)| &defined_unit.name == unit_name)
            .map(|(_, &exponent)| exponent)
    }

    /// Compute the overall dimensions of this compound unit.
    /// Each component's dimensions are scaled by its exponent and summed.
    pub fn dimensions(&self) -> [f64; 7] {
        let mut dims = [0.0; 7];
        for (defined_unit, &exponent) in &self.components {
            for (i, &d) in defined_unit.dimensions.iter().enumerate() {
                dims[i] += d * exponent;
            }
        }
        dims
    }

    /// Compute the overall scale factor of this compound unit.
    /// Each component's scale is raised to its exponent and multiplied together.
    pub fn scale(&self) -> f64 {
        self.components
            .iter()
            .map(|(defined_unit, &exponent)| defined_unit.scale.powf(exponent))
            .product()
    }

    /// Dimensional compatibility alone does not imply an operation is allowed.
    pub fn is_compatible(&self, other: &Unit) -> bool {
        let d1 = self.dimensions();
        let d2 = other.dimensions();
        d1.iter().zip(d2.iter()).all(|(a, b)| (a - b).abs() < 1e-12)
    }

    pub fn is_affine(&self) -> bool {
        self.components
            .keys()
            .any(|u| matches!(u.kind, UnitKind::Affine { .. }))
    }

    pub fn is_delta(&self) -> bool {
        self.components
            .keys()
            .any(|u| matches!(u.kind, UnitKind::Delta))
    }

    fn offset(&self) -> f64 {
        self.components
            .keys()
            .find_map(|u| match u.kind {
                UnitKind::Affine { offset, .. } => Some(offset),
                _ => None,
            })
            .unwrap_or(0.0)
    }

    fn delta(&self) -> Unit {
        match self.components.keys().find_map(|u| match &u.kind {
            UnitKind::Affine { delta, .. } => Some(delta.clone()),
            _ => None,
        }) {
            Some(delta) => Unit::new(HashMap::from([(delta, 1.0)])),
            None => self.clone(),
        }
    }

    /// Shared guard for products, powers and origin-dependent unary operations.
    pub fn check_multiplicative(&self, operation: &'static str) -> Result<(), UnitError> {
        if self.is_affine() {
            return Err(UnitError::Affine {
                operation,
                unit: self.to_string(),
            });
        }
        Ok(())
    }

    fn check_dimensions(&self, other: &Unit) -> Result<(), UnitError> {
        if !self.is_compatible(other) {
            return Err(UnitError::Incompatible(IncompatibleUnitsError {
                lhs: self.to_string(),
                rhs: other.to_string(),
            }));
        }
        Ok(())
    }

    pub fn conversion_to(&self, target: &Unit) -> Result<Conversion, UnitError> {
        self.check_dimensions(target)?;
        if (self.is_affine() && target.is_delta()) || (self.is_delta() && target.is_affine()) {
            return Err(UnitError::Affine {
                operation: "absolute/delta conversion",
                unit: format!("{self} -> {target}"),
            });
        }
        Ok(Conversion {
            scale: self.scale() / target.scale(),
            offset: (self.offset() - target.offset()) / target.scale(),
        })
    }

    pub fn addition(&self, rhs: &Unit) -> Result<BinaryOperation, UnitError> {
        self.additive_operation(rhs, false)
    }

    pub fn subtraction(&self, rhs: &Unit) -> Result<BinaryOperation, UnitError> {
        self.additive_operation(rhs, true)
    }

    fn additive_operation(&self, rhs: &Unit, subtract: bool) -> Result<BinaryOperation, UnitError> {
        self.check_dimensions(rhs)?;
        // A delta is translated into the absolute operand's scale, never its origin.
        if self.is_affine() && rhs.is_delta() {
            return Ok(BinaryOperation {
                lhs: Conversion::IDENTITY,
                rhs: Conversion {
                    scale: rhs.scale() / self.scale(),
                    offset: 0.0,
                },
                result: self.clone(),
            });
        }
        if self.is_delta() && rhs.is_affine() {
            return Ok(BinaryOperation {
                lhs: Conversion {
                    scale: self.scale() / rhs.scale(),
                    offset: 0.0,
                },
                rhs: Conversion::IDENTITY,
                result: rhs.clone(),
            });
        }
        if !subtract && (self.is_affine() || rhs.is_affine()) {
            return Err(UnitError::Affine {
                operation: "addition of absolutes",
                unit: format!("{self}, {rhs}"),
            });
        }
        Ok(BinaryOperation {
            lhs: Conversion::IDENTITY,
            rhs: rhs.conversion_to(self)?,
            result: if subtract { self.delta() } else { self.clone() },
        })
    }
}
impl Mul for Unit {
    type Output = Result<Unit, UnitError>;
    fn mul(self, other: Self) -> Self::Output {
        &self * &other
    }
}
impl Mul for &Unit {
    type Output = Result<Unit, UnitError>;
    fn mul(self, other: &Unit) -> Self::Output {
        self.check_multiplicative("multiplication")?;
        other.check_multiplicative("multiplication")?;
        // The new unit components is a sum of the units of "self" and "other"
        // This code ensures that if both units have the same components, they are summed and
        // any components that sum to zero are removed
        let mut component_map = std::collections::HashMap::new();

        // Add all components from both units
        for (component, value) in &self.components {
            *component_map.entry(component.clone()).or_insert(0.0) += value;
        }
        for (component, value) in &other.components {
            *component_map.entry(component.clone()).or_insert(0.0) += value;
        }

        Ok(Unit::new(component_map))
    }
}
impl Div for Unit {
    type Output = Result<Unit, UnitError>;

    fn div(self, other: Self) -> Self::Output {
        &self / &other
    }
}

impl Div for &Unit {
    type Output = Result<Unit, UnitError>;

    fn div(self, other: &Unit) -> Self::Output {
        self.check_multiplicative("division")?;
        other.check_multiplicative("division")?;
        let mut component_map = std::collections::HashMap::new();

        for (component, value) in &self.components {
            *component_map.entry(component.clone()).or_insert(0.0) += value;
        }
        for (component, value) in &other.components {
            *component_map.entry(component.clone()).or_insert(0.0) -= value;
        }

        component_map.retain(|_, v| v.abs() > 1e-12);

        Ok(Unit::new(component_map))
    }
}

/// Error type for unit compatibility failures in addition/subtraction.
#[derive(Debug, Clone)]
pub struct IncompatibleUnitsError {
    pub lhs: String,
    pub rhs: String,
}

impl fmt::Display for IncompatibleUnitsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "incompatible units: {} and {}", self.lhs, self.rhs)
    }
}

impl std::error::Error for IncompatibleUnitsError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affine_plans_are_owned_by_units() {
        let registry = crate::unit_registry::UnitRegistry::new_with_defaults();
        let unit = |name: &str| registry.parse_string(name.into()).unwrap();
        let c = unit("degC");
        let f = unit("degF");
        let dc = unit("delta_degC");
        let df = unit("delta_degF");
        let k = unit("K");

        let plan = c.subtraction(&f).unwrap();
        assert_eq!(plan.result.to_string(), dc.to_string());
        let lhs = plan.lhs.apply::<f64>(&30.0);
        let rhs = plan.rhs.apply::<f64>(&68.0);
        assert!((lhs - rhs - 10.0).abs() < 1e-12);

        let plan = df.subtraction(&c).unwrap();
        assert_eq!(plan.result.to_string(), c.to_string());
        assert!((plan.lhs.apply::<f64>(&9.0) - plan.rhs.apply::<f64>(&20.0) + 15.0).abs() < 1e-12);
        assert!(c.addition(&f).is_err());
        assert!(c.addition(&k).is_err());
        assert!(c.conversion_to(&dc).is_err());
        assert!(dc.conversion_to(&c).is_err());
        assert!((c.conversion_to(&k).unwrap().apply::<f64>(&20.0) - 293.15).abs() < 1e-12);
        assert_eq!(dc.conversion_to(&k).unwrap().offset, 0.0);
    }

    #[test]
    fn affine_operations_fail_before_simplification() {
        let registry = crate::unit_registry::UnitRegistry::new_with_defaults();
        let c = registry.parse_string("degC".into()).unwrap();
        assert!((&c / &c).is_err());
        assert!((&c * &c).is_err());
        for power in [0.0, 1.0, -1.0, 0.5, 2.0] {
            assert!(c.pow(power).is_err());
        }
        for operation in ["unary plus", "negation", "absolute value"] {
            assert!(c.check_multiplicative(operation).is_err());
        }
    }

    // Helper functions to create commonly used DefinedUnit instances
    fn create_kg_unit() -> Arc<DefinedUnit> {
        Arc::new(DefinedUnit {
            name: "kg".to_string(),
            dimensions: [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0], // Mass dimension
            kind: UnitKind::Linear,
            scale: 1.0,
            aliases: vec![],
        })
    }

    fn create_s_unit() -> Arc<DefinedUnit> {
        Arc::new(DefinedUnit {
            name: "s".to_string(),
            dimensions: [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0], // Time dimension
            kind: UnitKind::Linear,
            scale: 1.0,
            aliases: vec![],
        })
    }

    #[test]
    fn test_mul_two_kg_unit() {
        let kg = create_kg_unit();

        let unit_1 = Unit::new(HashMap::from([(kg.clone(), 1.0)]));
        let unit_2 = Unit::new(HashMap::from([(kg.clone(), 1.0)]));

        let unit_3 = (unit_1 * unit_2).unwrap();
        let kg_exponent = unit_3.components.get(&*kg);
        assert_eq!(kg_exponent, Some(&2.0));
    }

    #[test]
    fn test_div_two_kg_unit() {
        let kg = create_kg_unit();

        let unit_1 = Unit::new(HashMap::from([(kg.clone(), 1.0)]));
        let unit_2 = Unit::new(HashMap::from([(kg.clone(), 1.0)]));

        let unit_3 = (unit_1 / unit_2).unwrap();
        assert!(unit_3.components.is_empty());
    }

    #[test]
    fn test_mul_two_different_unit() {
        let kg = create_kg_unit();
        let s = create_s_unit();

        let unit_1 = Unit::new(HashMap::from([(kg.clone(), 1.0)]));
        let unit_2 = Unit::new(HashMap::from([(s.clone(), 1.0)]));

        let unit_3 = (unit_1 * unit_2).unwrap();
        assert_eq!(unit_3.components.get(&*kg), Some(&1.0));
        assert_eq!(unit_3.components.get(&*s), Some(&1.0));
    }

    #[test]
    fn test_div_two_different_unit() {
        let kg = create_kg_unit();
        let s = create_s_unit();

        let unit_1 = Unit::new(HashMap::from([(kg.clone(), 1.0)]));
        let unit_2 = Unit::new(HashMap::from([(s.clone(), 1.0)]));

        let unit_3 = (unit_1 / unit_2).unwrap();
        assert_eq!(unit_3.components.get(&*kg), Some(&1.0));
        assert_eq!(unit_3.components.get(&*s), Some(&-1.0));
    }
}

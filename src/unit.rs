use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::{ops::Div, ops::Mul};

#[derive(Debug, Clone)]
pub struct Unit {
    // TODO: consider using rational numbers for exponent and strong type for the unit
    pub components: std::collections::HashMap<Arc<DefinedUnit>, f64>,
}

// Dimensions are: Length, Mass, Time, ELectric Current, temperature, amount of substance, luminous intensity

#[derive(Debug, Clone)]
pub struct DefinedUnit {
    pub name: String,
    pub dimensions: [f64; 7],
    // TODO: this implies only "linear" units are used.
    // We probably want a more generic affine unit support, or even non-linear unit conversion scales
    pub scale: f64,
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
        // TODO: this implies only "linear" units are used.
        // We probably want a more generic affine unit support, or even non-linear unit conversion scales
        scale: f64,
        aliases: Vec<String>,
    ) -> DefinedUnit {
        DefinedUnit {
            name: name,
            dimensions: dimensions,
            scale: scale,
            aliases: aliases,
        }
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
    pub fn new(units: HashMap<Arc<DefinedUnit>, f64>) -> Unit {
        if units.is_empty() {
            return Unit {
                components: HashMap::new(),
            };
        }

        let mut result_components = HashMap::new();
        for (defined_unit, exponent) in units {
            result_components.insert(defined_unit, exponent);
        }
        // filter zero
        result_components.retain(|_, &mut value| value != 0.0);

        Unit {
            components: result_components,
        }
    }

    pub fn pow(&self, power: f64) -> Unit {
        let components = self
            .components
            .iter()
            .map(|(k, v)| (k.clone(), v * power))
            .collect();
        return Unit {
            components: components,
        };
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

    /// Check if two units have the same dimensions (are compatible for addition).
    pub fn is_compatible(&self, other: &Unit) -> bool {
        let d1 = self.dimensions();
        let d2 = other.dimensions();
        d1.iter().zip(d2.iter()).all(|(a, b)| (a - b).abs() < 1e-12)
    }

    /// Compute the conversion factor to convert a value in `other` units to `self` units.
    /// Returns an error if units are not dimensionally compatible.
    /// Usage: value_in_self = value_in_other * factor
    pub fn conversion_factor(&self, other: &Unit) -> Result<f64, IncompatibleUnitsError> {
        if !self.is_compatible(other) {
            return Err(IncompatibleUnitsError {
                lhs: self.to_string(),
                rhs: other.to_string(),
            });
        }
        Ok(other.scale() / self.scale())
    }
}
impl Mul for Unit {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
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

        // filter out zeros
        component_map.retain(|_, &mut v| v != 0.0);

        return Unit {
            components: component_map,
        };
    }
}
impl Mul for &Unit {
    type Output = Unit;
    fn mul(self, other: &Unit) -> Unit {
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

        // filter out zeros
        component_map.retain(|_, &mut v| v != 0.0);

        return Unit {
            components: component_map,
        };
    }
}
impl Div for Unit {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        // The new unit components is a sum of the units of "self" and "other"
        // This code ensures that if both units have the same components, they are summed and
        // any components that sum to zero are removed
        let mut component_map = std::collections::HashMap::new();

        // Add all components from both units
        for (component, value) in &self.components {
            *component_map.entry(component.clone()).or_insert(0.0) += value;
        }
        for (component, value) in &other.components {
            *component_map.entry(component.clone()).or_insert(0.0) -= value;
        }

        // filter out zeros
        component_map.retain(|_, &mut v| v != 0.0);

        return Unit {
            components: component_map,
        };
    }
}

impl Div for &Unit {
    type Output = Unit;

    fn div(self, other: &Unit) -> Unit {
        let mut component_map = std::collections::HashMap::new();

        for (component, value) in &self.components {
            *component_map.entry(component.clone()).or_insert(0.0) += value;
        }
        for (component, value) in &other.components {
            *component_map.entry(component.clone()).or_insert(0.0) -= value;
        }

        component_map.retain(|_, v| v.abs() > 1e-12);

        Unit {
            components: component_map,
        }
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
        write!(
            f,
            "cannot add/subtract incompatible units: {} and {}",
            self.lhs, self.rhs
        )
    }
}

impl std::error::Error for IncompatibleUnitsError {}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper functions to create commonly used DefinedUnit instances
    fn create_kg_unit() -> Arc<DefinedUnit> {
        Arc::new(DefinedUnit {
            name: "kg".to_string(),
            dimensions: [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0], // Mass dimension
            scale: 1.0,
            aliases: vec![],
        })
    }

    fn create_s_unit() -> Arc<DefinedUnit> {
        Arc::new(DefinedUnit {
            name: "s".to_string(),
            dimensions: [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0], // Time dimension
            scale: 1.0,
            aliases: vec![],
        })
    }

    #[test]
    fn test_mul_two_kg_unit() {
        let kg = create_kg_unit();

        let unit_1 = Unit::new(HashMap::from([(kg.clone(), 1.0)]));
        let unit_2 = Unit::new(HashMap::from([(kg.clone(), 1.0)]));

        let unit_3 = unit_1 * unit_2;
        let kg_exponent = unit_3.components.get(&*kg);
        assert_eq!(kg_exponent, Some(&2.0));
    }

    #[test]
    fn test_div_two_kg_unit() {
        let kg = create_kg_unit();

        let unit_1 = Unit::new(HashMap::from([(kg.clone(), 1.0)]));
        let unit_2 = Unit::new(HashMap::from([(kg.clone(), 1.0)]));

        let unit_3 = unit_1 / unit_2;
        assert!(unit_3.components.is_empty());
    }

    #[test]
    fn test_mul_two_different_unit() {
        let kg = create_kg_unit();
        let s = create_s_unit();

        let unit_1 = Unit::new(HashMap::from([(kg.clone(), 1.0)]));
        let unit_2 = Unit::new(HashMap::from([(s.clone(), 1.0)]));

        let unit_3 = unit_1 * unit_2;
        assert_eq!(unit_3.components.get(&*kg), Some(&1.0));
        assert_eq!(unit_3.components.get(&*s), Some(&1.0));
    }

    #[test]
    fn test_div_two_different_unit() {
        let kg = create_kg_unit();
        let s = create_s_unit();

        let unit_1 = Unit::new(HashMap::from([(kg.clone(), 1.0)]));
        let unit_2 = Unit::new(HashMap::from([(s.clone(), 1.0)]));

        let unit_3 = unit_1 / unit_2;
        assert_eq!(unit_3.components.get(&*kg), Some(&1.0));
        assert_eq!(unit_3.components.get(&*s), Some(&-1.0));
    }
}

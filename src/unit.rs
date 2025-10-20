use std::collections::HashMap;
use std::sync::Arc;
use std::{ops::Div, ops::Mul};
pub struct Unit {
    // TODO: consider using rational numbers for exponent and strong type for the unit
    components: std::collections::HashMap<Arc<DefinedUnit>, f64>,
}

// Dimensions are: Length, Mass, Time, ELectric Current, temperature, amount of substance, luminous intensity

#[derive(Debug, Clone)]
pub struct DefinedUnit {
    pub name: String,
    dimensions: [f64; 7],
    // TODO: this implies only "linear" units are used.
    // We probably want a more generic affine unit support, or even non-linear unit conversion scales
    scale: f64,
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

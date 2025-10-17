use ordered_float::OrderedFloat;
use std::{collections::HashMap, fmt};

pub struct Unit {
    // TODO: consider using rational numbers for exponent and strong type for the unit
    // The string here represents a "defined unit" that is part of the registry
    components: std::collections::HashMap<String, f64>,
}

// Dimensions are: Length, Mass, Time, ELectric Current, temperature, amount of substance, luminous intensity

pub struct DefinedUnit {
    name: String,
    dimensions: [f64; 7],
    // TODO: this implies only "linear" units are used.
    // We probably want a more generic affine unit support, or even non-linear unit conversion scales
    scale: f64,
    aliases: Vec<String>,
}

impl Unit {
    pub fn new(units: HashMap<String, f64>) -> Unit {
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

    pub fn mul(&self, other: &Self) -> Self {
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

    pub fn div(&self, other: &Self) -> Self {
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

    #[test]
    fn test_mul_two_kg_unit() {
        // You'd need to modify DefinedUnit to use OrderedFloat<f64> for scale
        let kg = DefinedUnit {
            name: "kg".to_string(),
            dimensions: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            scale: 1.0,
            aliases: vec![],
        };

        let unit_1 = Unit::new(HashMap::from([(kg.name.clone(), 1.0)]));
        let unit_2 = Unit::new(HashMap::from([(kg.name.clone(), 1.0)]));

        let unit_3 = unit_1.mul(&unit_2);
        let kg_exponent = unit_3.components.get(&"kg".to_string());
        assert!(kg_exponent == Some(&2.0))
    }

    #[test]
    fn test_div_two_kg_unit() {
        // You'd need to modify DefinedUnit to use OrderedFloat<f64> for scale
        let kg = DefinedUnit {
            name: "kg".to_string(),
            dimensions: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            scale: 1.0,
            aliases: vec![],
        };

        let unit_1 = Unit::new(HashMap::from([(kg.name.clone(), 1.0)]));
        let unit_2 = Unit::new(HashMap::from([(kg.name.clone(), 1.0)]));

        let unit_3 = unit_1.div(&unit_2);
        let kg_exponent = unit_3.components.get(&"kg".to_string());
        assert!(kg_exponent.iter().len() == 0)
    }
}

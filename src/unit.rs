use std::fmt;

pub struct Unit {
    // TODO: Consider whether we need to support custom dimensions
    dimension_exponents: [f64; 7],

    scale_factor: f64,

    // TODO: consider using rational numbers for exponent and strong type for the unit
    // The string here represents a "defined unit" that is part of the registry
    // Also consider using a map here. We expect the vectors to be short so I don't expect much
    // speed improvement and the ordering is important, so maybe not worth it.
    components: Vec<(String, f64)>,
}

#[derive(Debug)]
pub struct IncompatibleUnitError {
    pub expected: String,
    pub found: String,
}

impl fmt::Display for IncompatibleUnitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Incompatible units: expected {}, found {}",
            self.expected, self.found
        )
    }
}

impl std::error::Error for IncompatibleUnitError {}

impl Unit {
    pub fn has_same_dimensions(&self, other: &Self) -> bool {
        return other.dimension_exponents == self.dimension_exponents;
    }

    pub fn mul(&self, other: &Self) -> Self {
        let new_dimension =
            std::array::from_fn(|i| self.dimension_exponents[i] + other.dimension_exponents[i]);

        let mut new_components = Vec::with_capacity(self.components.len() + other.components.len());
        for (component, value) in &self.components {
            let other_value = other
                .components
                .iter()
                .find(|(k, _)| k == component)
                .map(|(_, v)| v)
                .unwrap_or(&0.0);
            let sum = other_value + value;
            if other_value + value != 0.0 {
                new_components.push((component.clone(), other_value + value))
            }
        }

        for (component, value) in &other.components {
            if (self
                .components
                .iter()
                .find(|(k, _)| k == component)
                .is_none())
            {
                new_components.push((component.clone(), *value));
            }
        }

        return Unit {
            dimension_exponents: new_dimension,
            scale_factor: self.scale_factor * other.scale_factor,
            components: new_components,
        };
    }
}

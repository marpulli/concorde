use crate::parser::{self, ParserError};
use crate::unit::{DefinedUnit, Unit};
use serde::Deserialize;
use std::sync::Mutex;
use std::{collections::HashMap, sync::Arc};

pub struct UnitRegistry {
    defined_units: HashMap<String, Arc<DefinedUnit>>,
    aliases: HashMap<String, String>,
    /// Parse cache: maps input string → parsed Unit (interior-mutable for &self API)
    parse_cache: Mutex<HashMap<String, Unit>>,
}

impl UnitRegistry {
    pub fn new(defined_units: Vec<DefinedUnit>) -> UnitRegistry {
        UnitRegistry {
            defined_units: defined_units
                .into_iter()
                .map(|f| (f.name.clone(), Arc::new(f)))
                .collect(),
            aliases: HashMap::new(),
            parse_cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn parse_string(&self, string: String) -> Result<Unit, ParserError> {
        // Fast-path: check cache first
        {
            let cache = self.parse_cache.lock().unwrap();
            if let Some(unit) = cache.get(&string) {
                return Ok(unit.clone());
            }
        }
        let unit = parser::parse(self, &string)?;
        self.parse_cache
            .lock()
            .unwrap()
            .insert(string, unit.clone());
        Ok(unit)
    }

    pub fn new_with_si() -> UnitRegistry {
        let si_units = vec![
            DefinedUnit::new(
                "kg".to_string(),
                [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                1.0,
                vec![],
            ),
            DefinedUnit::new(
                "m".to_string(),
                [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                1.0,
                vec![],
            ),
            DefinedUnit::new(
                "s".to_string(),
                [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
                1.0,
                vec![],
            ),
            DefinedUnit::new(
                "A".to_string(),
                [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
                1.0,
                vec![],
            ),
            DefinedUnit::new(
                "K".to_string(),
                [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                1.0,
                vec![],
            ),
            DefinedUnit::new(
                "mol".to_string(),
                [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
                1.0,
                vec![],
            ),
            DefinedUnit::new(
                "cd".to_string(),
                [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
                1.0,
                vec![],
            ),
        ];
        UnitRegistry::new(si_units)
    }

    pub fn define_unit(&mut self, unit: DefinedUnit) {
        self.defined_units.insert(unit.name.clone(), Arc::new(unit));
        // Invalidate parse cache since new units may affect existing parses
        self.parse_cache.lock().unwrap().clear();
    }

    pub fn get(&self, name: &String) -> Option<Arc<DefinedUnit>> {
        self.defined_units.get(name).cloned()
    }
}

#[cfg(test)]
mod test {
    use std::ops::Not;

    use super::*;

    fn create_unit_registry() -> UnitRegistry {
        let kg_unit = DefinedUnit::new(
            "kg".to_string(),
            [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0], // Mass dimension
            1.0,
            vec![],
        );
        let s = DefinedUnit::new(
            "s".to_string(),
            [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
            1.0,
            vec![],
        );
        let m = DefinedUnit::new(
            "m".to_string(),
            [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            1.0,
            vec![],
        );
        return UnitRegistry::new(Vec::from_iter([kg_unit, s, m]));
    }

    #[test]
    fn test_parse_integer_exponent() {
        let unit_registry = create_unit_registry();
        let unit = unit_registry.parse_string("kg**2".to_string());

        assert!(unit.is_ok());
        let unit = unit.unwrap();
        // Check that the unit has the correct exponent for kg
        let kg_def = unit_registry.get(&"kg".to_string()).unwrap();
        assert_eq!(unit.components.get(&kg_def), Some(&2.0));
    }

    #[test]
    fn test_parse_decimal_exponent() {
        let unit_registry = create_unit_registry();
        let unit = unit_registry.parse_string("kg**2.5".to_string());

        assert!(unit.is_ok());
        let unit = unit.unwrap();
        // Check that the unit has the correct exponent for kg
        let kg_def = unit_registry.get(&"kg".to_string()).unwrap();
        assert_eq!(unit.components.get(&kg_def), Some(&2.5));
    }

    #[test]
    fn test_parse_fraction_exponent() {
        let unit_registry = create_unit_registry();
        let unit = unit_registry.parse_string("kg**(2/5)".to_string());

        assert!(unit.is_ok());
        let unit = unit.unwrap();
        // Check that the unit has the correct exponent for kg
        let kg_def = unit_registry.get(&"kg".to_string()).unwrap();
        assert_eq!(unit.components.get(&kg_def), Some(&0.4));
    }

    #[test]
    fn test_parse_unit_multiplication_with_asterisk() {
        let unit_registry = create_unit_registry();
        let unit = unit_registry.parse_string("kg * s".to_string());

        assert!(unit.is_ok());
        let unit = unit.unwrap();
        // Check that the unit has the correct exponent for kg
        let kg_def = unit_registry.get(&"kg".to_string()).unwrap();
        let s_def = unit_registry.get(&"s".to_string()).unwrap();
        assert_eq!(unit.components.get(&kg_def), Some(&1.0));
        assert_eq!(unit.components.get(&s_def), Some(&1.0));
        assert_eq!(unit.components.len(), 2)
    }

    #[test]
    fn test_parse_unit_division() {
        let unit_registry = create_unit_registry();
        let unit = unit_registry.parse_string("kg / s".to_string());

        assert!(unit.is_ok());
        let unit = unit.unwrap();
        // Check that the unit has the correct exponent for kg
        let kg_def = unit_registry.get(&"kg".to_string()).unwrap();
        let s_def = unit_registry.get(&"s".to_string()).unwrap();
        assert_eq!(unit.components.get(&kg_def), Some(&1.0));
        assert_eq!(unit.components.get(&s_def), Some(&-1.0));
        assert_eq!(unit.components.len(), 2)
    }

    #[test]
    fn test_multiplication_and_division_precedence() {
        let unit_registry = create_unit_registry();
        let unit = unit_registry.parse_string("kg / s * s".to_string());

        assert!(unit.is_ok());
        let unit = unit.unwrap();
        // Check that the unit has the correct exponent for kg
        let kg_def = unit_registry.get(&"kg".to_string()).unwrap();
        let s_def = unit_registry.get(&"s".to_string()).unwrap();
        assert_eq!(unit.components.get(&kg_def), Some(&1.0));
        assert!(unit.components.contains_key(&s_def).not());
        assert_eq!(unit.components.len(), 1)
    }

    #[test]
    fn test_brackets() {
        let unit_registry = create_unit_registry();
        let unit = unit_registry.parse_string("kg / (s * s)".to_string());

        assert!(unit.is_ok());
        let unit = unit.unwrap();
        // Check that the unit has the correct exponent for kg
        let kg_def = unit_registry.get(&"kg".to_string()).unwrap();
        let s_def = unit_registry.get(&"s".to_string()).unwrap();
        assert_eq!(unit.components.get(&kg_def), Some(&1.0));
        assert_eq!(unit.components.get(&s_def), Some(&-2.0));
    }

    #[test]
    fn test_complex_precedence_mixed_operators() {
        let registry = create_unit_registry();
        let unit = registry.parse_string("kg * m^2 / s^2".to_string()).unwrap();
        assert_eq!(unit.get_exponent(&"kg".to_string()), Some(1.0));
        assert_eq!(unit.get_exponent(&"m".to_string()), Some(2.0));
        assert_eq!(unit.get_exponent(&"s".to_string()), Some(-2.0));
    }

    #[test]
    fn test_chained_exponentiation_with_fractions() {
        let registry = create_unit_registry();
        let unit = registry
            .parse_string("m^(3/2) * kg^(1/3) / s^(2/3)".to_string())
            .unwrap();
        assert_eq!(unit.get_exponent(&"m".to_string()), Some(1.5));
        assert_eq!(unit.get_exponent(&"kg".to_string()), Some(1.0 / 3.0));
        assert_eq!(unit.get_exponent(&"s".to_string()), Some(-2.0 / 3.0));
    }

    #[test]
    fn test_same_unit_multiple_times() {
        // m^(3/2) * kg^(1/3) / s^(2/3)
        let registry = create_unit_registry();
        let unit = registry.parse_string("kg * kg * kg".to_string()).unwrap();
        assert_eq!(unit.get_exponent(&"kg".to_string()), Some(3.0));
    }

    #[test]
    fn test_same_unit_cancellation() {
        let registry = create_unit_registry();
        let unit = registry.parse_string("kg / kg".to_string()).unwrap();
        assert_eq!(unit.get_exponent(&"kg".to_string()), None);
    }

    #[test]
    fn test_implicit_multiplication() {
        let registry = create_unit_registry();
        let unit = registry.parse_string("kg s".to_string()).unwrap();
        assert_eq!(unit.get_exponent(&"kg".to_string()), Some(1.0));
        assert_eq!(unit.get_exponent(&"s".to_string()), Some(1.0));
    }
}

use crate::{unit::DefinedUnit, unit_registry::UnitRegistry};
use serde::Deserialize;

#[derive(Deserialize)]
struct UnitDefinitionFile {
    #[serde(rename = "unit", default)]
    units: Vec<UnitDefinitionEntry>,
}

#[derive(Deserialize)]
struct UnitDefinitionEntry {
    name: String,
    dimensions: Option<[f64; 7]>,
    unit: Option<String>,
    scale: Option<f64>,
    #[serde(default)]
    aliases: Vec<String>,
}

/// Load unit definitions from a TOML file on disk into the given registry.
/// Returns the number of unit entries loaded, or an error message.
pub fn load_from_file(registry: &mut UnitRegistry, path: &str) -> Result<usize, String> {
    let contents =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read file '{path}': {e}"))?;
    load_from_string(registry, &contents)
}

/// Load unit definitions from a TOML string into the given registry.
/// Returns the number of unit entries loaded, or an error message.
pub fn load_from_string(registry: &mut UnitRegistry, toml_contents: &str) -> Result<usize, String> {
    let unit_definitions: UnitDefinitionFile =
        toml::from_str(toml_contents).map_err(|e| format!("Failed to parse TOML: {e}"))?;

    let count = unit_definitions.units.len();
    for entry in unit_definitions.units {
        apply_entry(registry, entry)?;
    }
    Ok(count)
}

fn apply_entry(registry: &mut UnitRegistry, entry: UnitDefinitionEntry) -> Result<(), String> {
    match (&entry.dimensions, &entry.unit) {
        (Some(_), Some(_)) => {
            return Err(format!(
                "Unit '{}': specify dimensions or unit, not both",
                entry.name
            ));
        }
        (None, None) => {
            return Err(format!(
                "Unit '{}': must specify either dimensions or unit",
                entry.name
            ));
        }
        _ => {}
    }

    let scale = entry.scale.unwrap_or(1.0);

    let (dimensions, resolved_scale) = if let Some(dims) = entry.dimensions {
        (dims, scale)
    } else {
        let expression = entry.unit.unwrap();
        let resolved_unit = registry.parse_string(expression.clone()).map_err(|e| {
            format!(
                "Failed to resolve unit '{expression}' for '{}': {e}",
                entry.name
            )
        })?;
        (resolved_unit.dimensions(), scale * resolved_unit.scale())
    };

    registry.define_unit(DefinedUnit::new(
        entry.name,
        dimensions,
        resolved_scale,
        entry.aliases,
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit_registry::UnitRegistry;

    #[test]
    fn test_load_base_unit() {
        let mut registry = UnitRegistry::new_with_si();
        let toml = r#"
[[unit]]
name = "furlong"
dimensions = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
scale = 201.168
"#;
        let count = load_from_string(&mut registry, toml).unwrap();
        assert_eq!(count, 1);
        let unit = registry.get(&"furlong".to_string()).unwrap();
        assert_eq!(unit.scale, 201.168);
    }

    #[test]
    fn test_load_derived_unit() {
        let mut registry = UnitRegistry::new_with_si();
        let toml = r#"
[[unit]]
name = "N"
unit = "kg * m / s^2"
"#;
        let count = load_from_string(&mut registry, toml).unwrap();
        assert_eq!(count, 1);
        let unit = registry.get(&"N".to_string()).unwrap();
        assert_eq!(unit.dimensions, [1.0, 1.0, -2.0, 0.0, 0.0, 0.0, 0.0]);
        assert_eq!(unit.scale, 1.0);
    }

    #[test]
    fn test_default_scale() {
        let mut registry = UnitRegistry::new_with_si();
        let toml = r#"
[[unit]]
name = "thing"
dimensions = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
"#;
        load_from_string(&mut registry, toml).unwrap();
        let unit = registry.get(&"thing".to_string()).unwrap();
        assert_eq!(unit.scale, 1.0);
    }

    #[test]
    fn test_both_dimensions_and_unit_errors() {
        let mut registry = UnitRegistry::new_with_si();
        let toml = r#"
[[unit]]
name = "bad"
dimensions = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
unit = "m"
"#;
        let err = load_from_string(&mut registry, toml).unwrap_err();
        assert!(err.contains("not both"), "error was: {err}");
    }

    #[test]
    fn test_neither_dimensions_nor_unit_errors() {
        let mut registry = UnitRegistry::new_with_si();
        let toml = r#"
[[unit]]
name = "bad"
"#;
        let err = load_from_string(&mut registry, toml).unwrap_err();
        assert!(err.contains("must specify"), "error was: {err}");
    }

    #[test]
    fn test_unknown_base_errors() {
        let mut registry = UnitRegistry::new_with_si();
        let toml = r#"
[[unit]]
name = "bad"
unit = "nonexistent"
"#;
        let err = load_from_string(&mut registry, toml).unwrap_err();
        assert!(err.contains("Failed to resolve unit"), "error was: {err}");
    }

    #[test]
    fn test_invalid_toml_errors() {
        let mut registry = UnitRegistry::new_with_si();
        let err = load_from_string(&mut registry, "this is {{ not valid toml").unwrap_err();
        assert!(err.contains("Failed to parse TOML"), "error was: {err}");
    }

    #[test]
    fn test_file_not_found_errors() {
        let mut registry = UnitRegistry::new_with_si();
        let err = load_from_file(&mut registry, "/nonexistent/path.toml").unwrap_err();
        assert!(err.contains("Failed to read file"), "error was: {err}");
    }
}

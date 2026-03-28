use crate::{
    unit::{DefinedUnit, Unit},
    unit_registry::UnitRegistry,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct UnitDefinitionFile {
    #[serde(rename = "unit")]
    pub units: Vec<UnitDefinitionEntry>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum UnitDefinitionEntry {
    Derived(DerivedUnitDefinitionEntry),
    FromDimension(UnitDefinitionFromDimension),
}

#[derive(Deserialize)]
pub struct UnitDefinitionFromDimension {
    pub name: String,
    pub dimensions: [f64; 7],
    pub scale: f64,
}

#[derive(Deserialize)]
pub struct DerivedUnitDefinitionEntry {
    pub name: String,
    pub derived: String,
    pub scale: f64,
}

pub fn parse(toml_contents: &str) -> Result<UnitRegistry, String> {
    let unit_definitions: UnitDefinitionFile =
        toml::from_str(toml_contents).map_err(|e| format!("Failed to parse toml: {e}"))?;

    let mut registry = UnitRegistry::new_with_si();
    for definition in unit_definitions.units {
        match definition {
            UnitDefinitionEntry::FromDimension(value) => registry.define_unit(DefinedUnit::new(
                value.name,
                value.dimensions,
                value.scale,
                Vec::new(),
            )),
            UnitDefinitionEntry::Derived(value) => {
                let maybe_new_unit = registry.parse_string(value.derived);

                match maybe_new_unit {
                    Ok(new_unit) => registry.define_unit(DefinedUnit::new(
                        value.name,
                        new_unit.dimensions(),
                        value.scale * new_unit.scale(),
                        Vec::new(),
                    )),
                    _ => println!("Couldn't parse unit"),
                }
            }
        }
    }
    Ok(registry)
}

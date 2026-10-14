//! Quantities embed the uncertain-value wire payload and a unit description.
//! The caller supplies the registry: loading never installs or changes units.
use super::Quantity;
use crate::uncertain_value::{UncertainValue, UncertainValuePayload};
use crate::unit::{Unit, UnitKind};
use crate::unit_registry::UnitRegistry;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Payload {
    version: u32,
    value: UncertainValuePayload,
    unit: UnitDescription,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct UnitDescription {
    expression: String,
    components: Vec<Component>,
}

/// Compare definitions, not just names or aggregate scale/dimensions. Recording
/// components also avoids floating-point differences from compound-unit folds.
#[derive(Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Component {
    name: String,
    exponent: f64,
    dimensions: [f64; 7],
    scale: f64,
    kind: Kind,
}

#[derive(Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Kind {
    Linear,
    Delta,
    Affine { offset: f64 },
}

impl UnitDescription {
    fn from_unit(unit: &Unit) -> Result<Self, String> {
        let mut components = Vec::new();
        for (definition, &exponent) in &unit.components {
            if !exponent.is_finite()
                || !definition.scale.is_finite()
                || definition.dimensions.iter().any(|d| !d.is_finite())
            {
                return Err("JSON serialization requires finite unit definitions".into());
            }
            let kind = match definition.kind {
                UnitKind::Linear => Kind::Linear,
                UnitKind::Delta => Kind::Delta,
                UnitKind::Affine { offset, .. } => {
                    if !offset.is_finite() {
                        return Err("JSON serialization requires finite unit definitions".into());
                    }
                    Kind::Affine { offset }
                }
            };
            components.push(Component {
                name: definition.name.clone(),
                exponent,
                dimensions: definition.dimensions,
                scale: definition.scale,
                kind,
            });
        }
        components.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(Self {
            expression: unit.to_string(),
            components,
        })
    }

    fn resolve(mut self, registry: &UnitRegistry) -> Result<Unit, String> {
        let unit = registry
            .parse_string(self.expression)
            .map_err(|e| e.to_string())?;
        self.components.sort_by(|a, b| a.name.cmp(&b.name));
        if self.components != Self::from_unit(&unit)?.components {
            return Err(
                "Unit definitions in the supplied registry do not match the serialized quantity"
                    .into(),
            );
        }
        Ok(unit)
    }
}

impl Quantity<UncertainValue> {
    pub(crate) fn to_json(&self) -> Result<String, String> {
        let payload = Payload {
            version: 1,
            value: self.value().to_payload()?,
            unit: UnitDescription::from_unit(self.unit())?,
        };
        serde_json::to_string(&payload).map_err(|e| e.to_string())
    }

    pub(crate) fn from_json(json: &str, registry: &UnitRegistry) -> Result<Self, String> {
        let payload: Payload = serde_json::from_str(json).map_err(|e| e.to_string())?;
        if payload.version != 1 {
            return Err(format!(
                "Unsupported quantity serialization version: {}",
                payload.version
            ));
        }
        let unit = payload.unit.resolve(registry)?;
        let value = UncertainValue::from_payload(payload.value)?;
        Ok(Self::new(value, unit))
    }
}

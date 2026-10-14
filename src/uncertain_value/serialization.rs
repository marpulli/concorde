//! Versioned, self-contained wire data. Source coordinates belong to the original
//! source, never the current output position. No process-global identity cache.
use super::{UncertainValue, ValueType, VarId};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Payload {
    version: u32,
    value: f64,
    sources: Vec<Source>,
    derivatives: Vec<Derivative>,
    variable_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    id: String,
    index: Option<usize>,
    uncertainty: f64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Derivative {
    source: usize,
    coefficient: f64,
}

fn finite(value: f64) -> Result<(), String> {
    if value.is_finite() {
        Ok(())
    } else {
        Err("JSON serialization requires finite numbers".into())
    }
}

impl UncertainValue {
    pub(crate) fn to_payload(&self) -> Result<Payload, String> {
        let ValueType::Scalar(value) = self.value else {
            return Err("Array serialization is not supported yet".into());
        };
        finite(value)?;
        let mut ids: Vec<_> = self.derivatives.keys().copied().collect();
        ids.sort_by_key(|id| (id.root, id.element));
        let mut sources = Vec::new();
        let mut derivatives = Vec::new();
        for id in ids {
            let ValueType::Scalar(coefficient) = self.derivatives[&id] else {
                return Err("Expected scalar derivative".into());
            };
            let ValueType::Scalar(uncertainty) = self.source_uncertainties[&id] else {
                return Err("Expected scalar source".into());
            };
            finite(coefficient)?;
            finite(uncertainty)?;
            if uncertainty < 0.0 {
                return Err("Source uncertainty must be nonnegative".into());
            }
            derivatives.push(Derivative {
                source: sources.len(),
                coefficient,
            });
            sources.push(Source {
                id: id.root.to_string(),
                index: id.element,
                uncertainty,
            });
        }
        Ok(Payload {
            version: 1,
            value,
            sources,
            derivatives,
            variable_id: self.variable_id.map(|id| id.root.to_string()),
        })
    }

    pub(crate) fn from_payload(payload: Payload) -> Result<Self, String> {
        if payload.version != 1 {
            return Err(format!(
                "Unsupported serialization version: {}",
                payload.version
            ));
        }
        finite(payload.value)?;
        let mut ids = Vec::new();
        let mut uncertainties = HashMap::new();
        for source in payload.sources {
            let id = VarId {
                root: Uuid::parse_str(&source.id).map_err(|e| e.to_string())?,
                element: source.index,
            };
            finite(source.uncertainty)?;
            if source.uncertainty < 0.0 {
                return Err("Source uncertainty must be nonnegative".into());
            }
            if uncertainties
                .insert(id, ValueType::Scalar(source.uncertainty))
                .is_some()
            {
                return Err("Duplicate source identity".into());
            }
            ids.push(id);
        }
        let mut used = HashSet::new();
        let mut derivatives = HashMap::new();
        for derivative in payload.derivatives {
            finite(derivative.coefficient)?;
            let id = *ids
                .get(derivative.source)
                .ok_or("Invalid source reference")?;
            if !used.insert(id) {
                return Err("Duplicate derivative source".into());
            }
            derivatives.insert(id, ValueType::Scalar(derivative.coefficient));
        }
        if used.len() != ids.len() {
            return Err("Unused source definition".into());
        }
        let mut value =
            Self::from_computation(ValueType::Scalar(payload.value), derivatives, uncertainties);
        if let Some(root) = payload.variable_id {
            let id = VarId {
                root: Uuid::parse_str(&root).map_err(|e| e.to_string())?,
                element: None,
            };
            if ids != [id] || !matches!(value.derivatives.get(&id), Some(ValueType::Scalar(1.0))) {
                return Err("Invalid independent variable identity".into());
            }
            value.variable_id = Some(id);
        }
        value.ensure_consistent_sources(&value)?;
        Ok(value)
    }

    pub(crate) fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(&self.to_payload()?).map_err(|e| e.to_string())
    }

    pub(crate) fn from_json(json: &str) -> Result<Self, String> {
        Self::from_payload(serde_json::from_str(json).map_err(|e| e.to_string())?)
    }
}

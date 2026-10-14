//! Versioned self-contained wire data. Source coordinates belong to the original
//! source, never the current output position. Arrays share source definitions;
//! derivative rows are sparse, not a dense output-by-source matrix.
//! Source identities are trusted to refer to immutable source definitions.
use super::{UncertainValue, ValueType, VarId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Payload {
    version: u32,
    value: Numbers,
    sources: Vec<Source>,
    derivatives: Derivatives,
    variable_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum Numbers {
    Scalar(f64),
    Array(Vec<f64>),
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum Derivatives {
    Scalar(Vec<Derivative>),
    Array(Vec<Vec<Derivative>>),
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    id: String,
    uncertainty: Numbers,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    indices: Option<Vec<usize>>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Derivative {
    source: usize,
    coefficient: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    coordinate: Option<usize>,
}

fn finite(value: f64) -> Result<(), String> {
    if value.is_finite() {
        Ok(())
    } else {
        Err("JSON serialization requires finite numbers".into())
    }
}

fn sigma(value: f64) -> Result<(), String> {
    finite(value)?;
    if value < 0.0 {
        Err("Source uncertainty must be nonnegative".into())
    } else {
        Ok(())
    }
}

impl UncertainValue {
    pub(crate) fn to_payload(&self) -> Result<Payload, String> {
        let (value, rows) = match &self.value {
            ValueType::Scalar(value) => (Numbers::Scalar(*value), vec![self.clone()]),
            ValueType::Array(values) => (
                Numbers::Array(values.to_vec()),
                (0..values.len())
                    .map(|i| self.element(i).expect("Valid array element"))
                    .collect(),
            ),
        };
        for row in &rows {
            finite(row.value.scalar())?;
        }
        let mut sources = Vec::new();
        let mut references = HashMap::new();
        let mut grouped: BTreeMap<(Uuid, bool), BTreeMap<Option<usize>, f64>> = BTreeMap::new();
        for row in &rows {
            for id in row.derivatives.keys() {
                let uncertainty = row.source_uncertainties[id].scalar();
                sigma(uncertainty)?;
                grouped
                    .entry((id.root, id.element.is_some()))
                    .or_default()
                    .insert(id.element, uncertainty);
            }
        }
        for ((root, _), entries) in grouped {
            let source = sources.len();
            if let Some(&uncertainty) = entries.get(&None) {
                references.insert(
                    VarId {
                        root,
                        element: None,
                    },
                    (source, None),
                );
                sources.push(Source {
                    id: root.to_string(),
                    uncertainty: Numbers::Scalar(uncertainty),
                    indices: None,
                });
            } else {
                let mut indices = Vec::new();
                let mut uncertainties = Vec::new();
                for (index, uncertainty) in entries {
                    let index = index.expect("Array source coordinate");
                    references.insert(
                        VarId {
                            root,
                            element: Some(index),
                        },
                        (source, Some(index)),
                    );
                    indices.push(index);
                    uncertainties.push(uncertainty);
                }
                sources.push(Source {
                    id: root.to_string(),
                    uncertainty: Numbers::Array(uncertainties),
                    indices: Some(indices),
                });
            }
        }
        let mut encoded = Vec::new();
        for row in rows {
            let mut derivatives = Vec::new();
            for (id, coefficient) in row.derivatives {
                let coefficient = coefficient.scalar();
                finite(coefficient)?;
                let (source, coordinate) = references[&id];
                derivatives.push(Derivative {
                    source,
                    coordinate,
                    coefficient,
                });
            }
            derivatives.sort_by_key(|d| (d.source, d.coordinate));
            encoded.push(derivatives);
        }
        let derivatives = match &value {
            Numbers::Scalar(_) => Derivatives::Scalar(encoded.pop().expect("Scalar row")),
            Numbers::Array(_) => Derivatives::Array(encoded),
        };
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
        let mut tables = Vec::new();
        let mut uncertainties = HashMap::new();
        for source in payload.sources {
            let root = Uuid::parse_str(&source.id).map_err(|e| e.to_string())?;
            let entries = match (source.uncertainty, source.indices) {
                (Numbers::Scalar(uncertainty), None) => vec![(None, None, uncertainty)],
                (Numbers::Array(uncertainties), Some(indices))
                    if uncertainties.len() == indices.len() && !indices.is_empty() =>
                {
                    indices
                        .into_iter()
                        .zip(uncertainties)
                        .map(|(i, s)| (Some(i), Some(i), s))
                        .collect()
                }
                _ => return Err("Invalid source uncertainty shape".into()),
            };
            let mut table = HashMap::new();
            for (reference, element, uncertainty) in entries {
                sigma(uncertainty)?;
                let id = VarId { root, element };
                if uncertainties
                    .insert(id, ValueType::Scalar(uncertainty))
                    .is_some()
                {
                    return Err("Duplicate source identity".into());
                }
                table.insert(reference, id);
            }
            tables.push(table);
        }
        let is_array = matches!(payload.value, Numbers::Array(_));
        let values = match payload.value {
            Numbers::Scalar(value) => vec![value],
            Numbers::Array(values) => values,
        };
        let derivatives = match payload.derivatives {
            Derivatives::Scalar(row) if !is_array => vec![row],
            // [] is ambiguous under untagged decoding; only an empty array may
            // interpret the empty scalar-row spelling as zero derivative rows.
            Derivatives::Scalar(row) if values.is_empty() && row.is_empty() => vec![],
            Derivatives::Array(rows) if is_array => rows,
            _ => return Err("Derivative shape does not match values".into()),
        };
        if values.len() != derivatives.len() {
            return Err("Derivative shape does not match values".into());
        }
        let mut used = HashSet::new();
        let mut rows = Vec::new();
        for (value, row) in values.into_iter().zip(derivatives) {
            finite(value)?;
            let mut coefficients = HashMap::new();
            let mut source_uncertainties = HashMap::new();
            for derivative in row {
                finite(derivative.coefficient)?;
                let id = *tables
                    .get(derivative.source)
                    .and_then(|table| table.get(&derivative.coordinate))
                    .ok_or("Invalid source reference")?;
                if coefficients
                    .insert(id, ValueType::Scalar(derivative.coefficient))
                    .is_some()
                {
                    return Err("Duplicate derivative source".into());
                }
                source_uncertainties.insert(id, uncertainties[&id].clone());
                used.insert(id);
            }
            rows.push(Self::from_computation(
                ValueType::Scalar(value),
                coefficients,
                source_uncertainties,
            ));
        }
        if used.len() != uncertainties.len() {
            return Err("Unused source definition".into());
        }
        let variable_id = payload
            .variable_id
            .map(|s| Uuid::parse_str(&s).map_err(|e| e.to_string()))
            .transpose()?;
        if let Some(root) = variable_id {
            for (i, row) in rows.iter().enumerate() {
                let id = VarId {
                    root,
                    element: if is_array { Some(i) } else { None },
                };
                if row.derivatives.len() != 1
                    || !matches!(row.derivatives.get(&id), Some(ValueType::Scalar(1.0)))
                {
                    return Err("Invalid independent variable identity".into());
                }
            }
        }
        let mut value = if is_array {
            Self::from_elements(rows)
        } else {
            rows.pop().expect("Scalar row")
        };
        value.variable_id = variable_id.map(|root| VarId {
            root,
            element: None,
        });
        Ok(value)
    }

    pub(crate) fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(&self.to_payload()?).map_err(|e| e.to_string())
    }

    pub(crate) fn from_json(json: &str) -> Result<Self, String> {
        Self::from_payload(serde_json::from_str(json).map_err(|e| e.to_string())?)
    }
}

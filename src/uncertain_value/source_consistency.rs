//! Reject conflicting source definitions when independently restored values meet.
use super::{NumpyArray1D, UncertainValue, ValueType, VarId};
use std::collections::HashMap;
use uuid::Uuid;

enum Definition {
    Scalar(f64),
    Array {
        whole: Option<NumpyArray1D>,
        selected: HashMap<usize, f64>,
    },
}

fn conflict() -> String {
    "Conflicting definitions for the same uncertainty source".into()
}

fn record(
    definitions: &mut HashMap<Uuid, Definition>,
    id: VarId,
    sigma: &ValueType,
) -> Result<(), String> {
    if let (None, ValueType::Scalar(sigma)) = (id.element, sigma) {
        match definitions
            .entry(id.root)
            .or_insert(Definition::Scalar(*sigma))
        {
            Definition::Scalar(existing) if existing == sigma => return Ok(()),
            _ => return Err(conflict()),
        }
    }
    let definition = definitions
        .entry(id.root)
        .or_insert_with(|| Definition::Array {
            whole: None,
            selected: HashMap::new(),
        });
    let Definition::Array { whole, selected } = definition else {
        return Err(conflict());
    };
    match sigma {
        ValueType::Array(array) => {
            if let Some(existing) = whole {
                let shared = existing.as_ptr() == array.as_ptr()
                    && existing.shape() == array.shape()
                    && existing.strides() == array.strides();
                if !shared && existing != array {
                    return Err(conflict());
                }
            }
            for (&i, &sigma) in selected.iter() {
                if array.get(i) != Some(&sigma) {
                    return Err(conflict());
                }
            }
            *whole = Some(array.clone());
        }
        ValueType::Scalar(sigma) => {
            let i = id.element.expect("Indexed scalar source");
            if let Some(array) = whole {
                if array.get(i) != Some(sigma) {
                    return Err(conflict());
                }
            }
            if let Some(existing) = selected.insert(i, *sigma) {
                if existing != *sigma {
                    return Err(conflict());
                }
            }
        }
    }
    Ok(())
}

impl UncertainValue {
    pub(crate) fn ensure_consistent_sources(&self, other: &Self) -> Result<(), String> {
        fn collect(
            value: &UncertainValue,
            definitions: &mut HashMap<Uuid, Definition>,
        ) -> Result<(), String> {
            if let Some(elements) = &value.elements {
                for element in elements.iter() {
                    collect(element, definitions)?;
                }
            } else {
                for (&id, sigma) in &value.source_uncertainties {
                    record(definitions, id, sigma)?;
                }
            }
            Ok(())
        }
        let mut definitions = HashMap::new();
        collect(self, &mut definitions)?;
        collect(other, &mut definitions)
    }
}

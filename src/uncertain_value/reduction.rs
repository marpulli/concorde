use super::{UncertainValue, ValueType, VarId};
use std::collections::HashMap;

impl UncertainValue {
    /// Chain a scalar result's per-output derivatives back to the original sources.
    /// Compact arrays are traversed directly, without allocating a scalar/map per
    /// input or repeatedly cloning an ever-growing accumulator.
    pub(super) fn reduce_with_gradient(&self, value: f64, gradient: impl Fn(usize) -> f64) -> Self {
        let ValueType::Array(values) = &self.value else {
            unreachable!("Array reduction invariant");
        };
        let mut derivatives = HashMap::<VarId, f64>::new();
        let mut uncertainties = HashMap::new();
        let mut accumulate = |source, coefficient: f64, sigma| {
            if coefficient != 0.0 {
                *derivatives.entry(source).or_default() += coefficient;
                uncertainties
                    .entry(source)
                    .or_insert(ValueType::Scalar(sigma));
            }
        };
        if let Some(elements) = &self.elements {
            for (i, element) in elements.iter().enumerate() {
                let weight = gradient(i);
                for (&source, derivative) in &element.derivatives {
                    accumulate(
                        source,
                        weight * derivative.scalar(),
                        element.source_uncertainties[&source].scalar(),
                    );
                }
            }
        } else {
            for (&source, derivative) in &self.derivatives {
                match &self.source_uncertainties[&source] {
                    ValueType::Array(sigmas) => {
                        for i in 0..values.len() {
                            let coordinate = if sigmas.len() == 1 { 0 } else { i };
                            accumulate(
                                VarId {
                                    root: source.root,
                                    element: Some(coordinate),
                                },
                                gradient(i) * derivative.at(i),
                                sigmas[coordinate],
                            );
                        }
                    }
                    ValueType::Scalar(sigma) => {
                        let coefficient = (0..values.len())
                            .map(|i| gradient(i) * derivative.at(i))
                            .sum();
                        accumulate(source, coefficient, *sigma);
                    }
                }
            }
        }
        Self::from_computation(
            ValueType::Scalar(value),
            derivatives
                .into_iter()
                .map(|(id, d)| (id, ValueType::Scalar(d)))
                .collect(),
            uncertainties,
        )
    }

    /// Arithmetic mean; undefined for an empty collection.
    pub(crate) fn mean(&self) -> Result<Self, &'static str> {
        match &self.value {
            ValueType::Scalar(_) => Ok(self.clone()),
            ValueType::Array(values) => {
                if values.is_empty() {
                    return Err("Mean requires at least one element");
                }
                let weight = 1.0 / values.len() as f64;
                let value = values.iter().map(|v| v * weight).sum();
                Ok(self.reduce_with_gradient(value, |_| weight))
            }
        }
    }

    /// Sum all outputs; the empty sum is the exact scalar zero.
    pub(crate) fn sum(&self) -> Self {
        match &self.value {
            ValueType::Scalar(_) => self.clone(),
            ValueType::Array(values) => self.reduce_with_gradient(values.sum(), |_| 1.0),
        }
    }
}

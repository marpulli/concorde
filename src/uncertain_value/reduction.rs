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

    /// Standard deviation of nominal outputs, with first-order uncertainty of
    /// that statistic (not the per-element measurement uncertainties).
    pub(crate) fn std(&self, ddof: f64) -> Result<Self, &'static str> {
        let length = match &self.value {
            ValueType::Scalar(_) => 1,
            ValueType::Array(values) => values.len(),
        };
        if length == 0 || !ddof.is_finite() || length as f64 <= ddof {
            return Err(
                "Standard deviation requires nonempty input and finite ddof < element count",
            );
        }
        let exact_zero =
            || Self::from_computation(ValueType::Scalar(0.0), HashMap::new(), HashMap::new());
        let ValueType::Array(values) = &self.value else {
            return if self.value.scalar().is_finite() {
                Ok(exact_zero())
            } else {
                Err("Standard deviation requires finite values")
            };
        };
        if values.iter().any(|v| !v.is_finite()) {
            return Err("Standard deviation requires finite values");
        }

        // Remove a common offset before scaling, to retain small spreads around
        // large nominal values. If subtraction overflows, normalize first.
        let mut normalized = values.iter().map(|v| v - values[0]).collect::<Vec<_>>();
        let mut scale = normalized.iter().fold(0.0_f64, |s, d| s.max(d.abs()));
        if !scale.is_finite() {
            scale = values.iter().fold(0.0_f64, |s, v| s.max(v.abs()));
            normalized = values.iter().map(|v| v / scale).collect();
        } else if scale != 0.0 {
            normalized.iter_mut().for_each(|d| *d /= scale);
        }
        if scale == 0.0 {
            // The norm is nondifferentiable at zero spread unless every input's
            // first-order perturbation is identical. Never report a falsely
            // exact zero for equal, independently uncertain measurements.
            let first = self.element(0).unwrap();
            for i in 1..length {
                let difference = &self.element(i).unwrap() - &first;
                if difference.derivatives.iter().any(|(id, d)| {
                    d.scalar() != 0.0 && difference.source_uncertainties[id].scalar() != 0.0
                }) {
                    return Err("Uncertainty of standard deviation is undefined at zero spread");
                }
            }
            return Ok(exact_zero());
        }
        let mean = normalized.iter().sum::<f64>() / length as f64;
        let deviations = normalized.iter().map(|v| v - mean).collect::<Vec<_>>();
        let denominator = length as f64 - ddof;
        let normalized_std = (deviations.iter().map(|d| d * d).sum::<f64>() / denominator).sqrt();
        let mut gradient = deviations
            .iter()
            .map(|d| d / (denominator * normalized_std))
            .collect::<Vec<_>>();
        // Enforce translation invariance exactly for a shared additive source.
        gradient[length - 1] = -gradient[..length - 1].iter().sum::<f64>();
        Ok(self.reduce_with_gradient(scale * normalized_std, |i| gradient[i]))
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

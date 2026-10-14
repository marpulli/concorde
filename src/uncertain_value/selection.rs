use super::{UncertainValue, ValueType, VarId};
use numpy::ndarray::ArcArray;
use std::borrow::Cow;
use std::collections::HashMap;

impl ValueType {
    pub(super) fn scalar(&self) -> f64 {
        match self {
            Self::Scalar(value) => *value,
            Self::Array(_) => unreachable!("Internal scalar row invariant"),
        }
    }

    /// Select from a broadcast value, retaining scalar inputs unchanged.
    pub(super) fn at(&self, index: usize) -> f64 {
        match self {
            Self::Scalar(value) => *value,
            Self::Array(values) => values[if values.len() == 1 { 0 } else { index }],
        }
    }
}

impl UncertainValue {
    pub(super) fn from_elements(elements: Vec<Self>) -> Self {
        let values = elements
            .iter()
            .map(|e| e.value.scalar())
            .collect::<numpy::ndarray::Array1<_>>();
        Self {
            value: ValueType::Array(values.into_shared()),
            elements: Some(elements.into()),
            variable_id: None,
            derivatives: HashMap::new(),
            source_uncertainties: HashMap::new(),
        }
    }

    pub(crate) fn select(&self, indices: &[usize]) -> Option<Self> {
        if matches!(self.value, ValueType::Scalar(_)) {
            return None;
        }
        indices
            .iter()
            .map(|&i| self.element(i))
            .collect::<Option<Vec<_>>>()
            .map(Self::from_elements)
    }

    /// Replace validated positions without changing any existing source definition.
    /// Construction is transactional: callers swap in the result only on success.
    pub(crate) fn replaced(&self, indices: &[usize], replacement: &Self) -> Result<Self, &'static str> {
        let ValueType::Array(values) = &self.value else {
            return Err("Scalar values cannot be assigned through a mask");
        };
        if let ValueType::Array(rhs) = &replacement.value {
            if rhs.len() != 1 && rhs.len() != indices.len() {
                return Err("Replacement length must be one or the number of selected elements");
            }
        }
        let mut elements = (0..values.len()).map(|i| self.element(i).unwrap()).collect::<Vec<_>>();
        for (j, &i) in indices.iter().enumerate() {
            let slot = elements.get_mut(i).ok_or("Assignment index out of range")?;
            *slot = replacement.broadcast_element(j);
        }
        Ok(Self::from_elements(elements))
    }

    /// Select a broadcast operand for elementwise arithmetic.
    pub(super) fn broadcast_element(&self, index: usize) -> Self {
        match &self.value {
            ValueType::Scalar(_) => self.clone(),
            ValueType::Array(values) => self
                .element(if values.len() == 1 { 0 } else { index })
                .expect("Validated array shape"),
        }
    }

    /// Integer indexing preserves source identity, including the source coordinate.
    /// Different elements of an independent source array are independent; scalar
    /// sources broadcast across an array remain shared by all selected elements.
    pub(crate) fn element(&self, index: usize) -> Option<Self> {
        let ValueType::Array(values) = &self.value else {
            return None;
        };
        let value = *values.get(index)?;
        if let Some(elements) = &self.elements {
            return elements.get(index).cloned();
        }
        let mut derivatives = HashMap::new();
        let mut uncertainties = HashMap::new();
        for (source, derivative) in &self.derivatives {
            let uncertainty = &self.source_uncertainties[source];
            let selected_source = match uncertainty {
                ValueType::Array(values) => VarId {
                    root: source.root,
                    element: Some(if values.len() == 1 { 0 } else { index }),
                },
                ValueType::Scalar(_) => *source,
            };
            // A source may already have an explicit element contribution from
            // earlier scalar/array arithmetic. Combine rather than overwrite it.
            *derivatives.entry(selected_source).or_insert(0.0) += derivative.at(index);
            uncertainties.insert(selected_source, ValueType::Scalar(uncertainty.at(index)));
        }
        Some(Self::from_computation(
            ValueType::Scalar(value),
            derivatives
                .into_iter()
                .map(|(id, d)| (id, ValueType::Scalar(d)))
                .collect(),
            uncertainties,
        ))
    }

    /// Align array sources with explicitly selected elements before arithmetic.
    /// For `x - x[0]`, split element zero out of x's vector source so its
    /// derivative cancels only at position zero. The remaining vector stays
    /// compact: indexing does not expand every source into a dense Jacobian.
    pub(super) fn align_sources(&self, other: &Self) -> Cow<'_, Self> {
        let mut aligned = Cow::Borrowed(self);
        for selected in other.derivatives.keys().chain(self.derivatives.keys()) {
            let Some(element) = selected.element else {
                continue;
            };
            let root = VarId {
                root: selected.root,
                element: None,
            };
            let Some(ValueType::Array(uncertainties)) = self.source_uncertainties.get(&root) else {
                continue;
            };
            let Some(derivative) = aligned.derivatives.get(&root) else {
                continue;
            };
            let mask =
                ArcArray::from_shape_fn(
                    uncertainties.dim(),
                    |i| {
                        if i == element {
                            1.0
                        } else {
                            0.0
                        }
                    },
                );
            let contribution = derivative.clone() * ValueType::Array(mask.clone());
            let remainder =
                derivative.clone() * ValueType::Array(mask.mapv(|v| 1.0 - v).into_shared());
            let aligned = aligned.to_mut();
            aligned.derivatives.insert(root, remainder);
            aligned
                .derivatives
                .entry(*selected)
                .and_modify(|d| *d = d.clone() + contribution.clone())
                .or_insert(contribution);
            aligned
                .source_uncertainties
                .insert(*selected, ValueType::Scalar(uncertainties[element]));
        }
        aligned
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use numpy::array;

    fn source() -> UncertainValue {
        UncertainValue::new_independent_array(
            array![2.0, 4.0].into_shared(),
            array![0.1, 0.2].into_shared(),
        )
    }

    fn scalar_uncertainty(value: &UncertainValue) -> f64 {
        match value.uncertainty() {
            ValueType::Scalar(value) => value,
            _ => panic!("Expected a scalar"),
        }
    }

    #[test]
    fn selection_tracks_source_coordinates() {
        let x = source();
        let first = x.element(0).unwrap();
        let again = x.element(0).unwrap();
        let second = x.element(1).unwrap();
        assert_eq!(scalar_uncertainty(&(&first - &again)), 0.0);
        assert!((scalar_uncertainty(&(&first - &second)) - 0.1_f64.hypot(0.2)).abs() < 1e-12);
        assert!(x.element(2).is_none());
        assert!(first.element(0).is_none());
    }

    #[test]
    fn selection_broadcasts_back_to_parent() {
        let x = source();
        let first = x.element(0).unwrap();
        let difference = &x - &first;
        assert_eq!(scalar_uncertainty(&difference.element(0).unwrap()), 0.0);
        assert!(
            (scalar_uncertainty(&difference.element(1).unwrap()) - 0.1_f64.hypot(0.2)).abs()
                < 1e-12
        );
    }

    #[test]
    fn ordinary_arithmetic_does_not_clone_for_alignment() {
        let x = source();
        let y = source();
        assert!(matches!(x.align_sources(&y), Cow::Borrowed(_)));
    }
}

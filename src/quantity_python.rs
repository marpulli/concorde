use crate::quantity::Quantity;
use crate::uncertain_value::{PyUncertainValue, UncertainValue};
use crate::unit_python::PyUnit;
use pyo3::prelude::*;

#[pyclass(name = "Quantity")]
pub struct PyQuantity {
    inner: Quantity<UncertainValue>,
}

#[pymethods]
impl PyQuantity {
    #[new]
    fn new(value: &PyUncertainValue, unit: &PyUnit) -> Self {
        PyQuantity {
            inner: Quantity::new(value.inner.clone(), unit.inner.clone()),
        }
    }

    #[getter]
    fn value(&self) -> PyUncertainValue {
        PyUncertainValue {
            inner: self.inner.value().clone(),
        }
    }

    #[getter]
    fn unit(&self) -> PyUnit {
        PyUnit {
            inner: self.inner.unit().clone(),
        }
    }

    fn __mul__(&self, other: &PyQuantity) -> PyQuantity {
        PyQuantity {
            inner: &self.inner * &other.inner,
        }
    }

    fn __add__(&self, other: &PyQuantity) -> PyResult<PyQuantity> {
        (&self.inner + &other.inner)
            .map(|inner| PyQuantity { inner })
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __sub__(&self, other: &PyQuantity) -> PyResult<PyQuantity> {
        (&self.inner - &other.inner)
            .map(|inner| PyQuantity { inner })
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        use crate::uncertain_value::ValueType;
        let value_str = match &self.inner.value().value {
            ValueType::Scalar(v) => {
                let unc = self.inner.value().uncertainty();
                match unc {
                    ValueType::Scalar(u) => format!("{} ± {}", v, u),
                    _ => format!("{}", v),
                }
            }
            ValueType::Array(_) => "array(...)".to_string(),
        };
        format!("Quantity({}, {})", value_str, self.inner.unit())
    }
}

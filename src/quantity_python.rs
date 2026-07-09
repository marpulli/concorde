use crate::quantity::Quantity;
use crate::uncertain_value::{PyUncertainValue, UncertainValue};
use crate::unit_python::PyUnit;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;

#[pyclass(name = "Quantity")]
pub struct PyQuantity {
    inner: Quantity<UncertainValue>,
}

#[pymethods]
impl PyQuantity {
    #[new]
    fn new(value: &Bound<'_, PyAny>, unit: &PyUnit) -> PyResult<Self> {
        let value = if let Ok(uv) = value.extract::<PyRef<PyUncertainValue>>() {
            uv.inner.clone()
        } else if let Ok(scalar) = value.extract::<f64>() {
            UncertainValue::new_independent(scalar, 0.0)
        } else {
            return Err(PyTypeError::new_err(
                "value must be an UncertainValue or a float",
            ));
        };

        Ok(PyQuantity {
            inner: Quantity::new(value, unit.inner.clone()),
        })
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

    fn __mul__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyQuantity> {
        if let Ok(other) = other.extract::<PyRef<PyQuantity>>() {
            Ok(PyQuantity {
                inner: &self.inner * &other.inner,
            })
        } else if let Ok(scalar) = other.extract::<f64>() {
            Ok(PyQuantity {
                inner: &self.inner * scalar,
            })
        } else {
            Err(PyTypeError::new_err(
                "unsupported operand type for *: expected Quantity or float",
            ))
        }
    }

    fn __rmul__(&self, scalar: f64) -> PyQuantity {
        PyQuantity {
            inner: &self.inner * scalar,
        }
    }

    fn __add__(&self, other: &PyQuantity) -> PyResult<PyQuantity> {
        (&self.inner + &other.inner)
            .map(|inner| PyQuantity { inner })
            .map_err(|e| crate::IncompatibleUnitError::new_err(e.to_string()))
    }

    fn __sub__(&self, other: &PyQuantity) -> PyResult<PyQuantity> {
        (&self.inner - &other.inner)
            .map(|inner| PyQuantity { inner })
            .map_err(|e| crate::IncompatibleUnitError::new_err(e.to_string()))
    }

    fn __truediv__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyQuantity> {
        if let Ok(other) = other.extract::<PyRef<PyQuantity>>() {
            Ok(PyQuantity {
                inner: &self.inner / &other.inner,
            })
        } else if let Ok(scalar) = other.extract::<f64>() {
            Ok(PyQuantity {
                inner: &self.inner * (1.0 / scalar),
            })
        } else {
            Err(PyTypeError::new_err(
                "unsupported operand type for /: expected Quantity or float",
            ))
        }
    }

    fn to(&self, unit: &PyUnit) -> PyResult<PyQuantity> {
        self.inner
            .to(&unit.inner)
            .map(|inner| PyQuantity { inner })
            .map_err(|e| crate::IncompatibleUnitError::new_err(e.to_string()))
    }

    fn __neg__(&self) -> PyQuantity {
        PyQuantity {
            inner: -&self.inner,
        }
    }

    fn __eq__(&self, other: &PyQuantity) -> bool {
        self.inner == other.inner
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

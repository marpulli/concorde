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

    fn __mul__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyQuantity> {
        let inner = if let Ok(quantity) = other.extract::<PyRef<'_, PyQuantity>>() {
            (&self.inner * &quantity.inner)?
        } else {
            (&self.inner * other.extract::<f64>()?)?
        };
        Ok(PyQuantity { inner })
    }

    fn __rmul__(&self, other: f64) -> PyResult<PyQuantity> {
        Ok(PyQuantity {
            inner: (&self.inner * other)?,
        })
    }

    fn __add__(&self, other: &PyQuantity) -> PyResult<PyQuantity> {
        Ok(PyQuantity {
            inner: (&self.inner + &other.inner)?,
        })
    }

    fn __sub__(&self, other: &PyQuantity) -> PyResult<PyQuantity> {
        Ok(PyQuantity {
            inner: (&self.inner - &other.inner)?,
        })
    }

    fn __truediv__(&self, other: &Bound<'_, PyAny>) -> PyResult<PyQuantity> {
        let inner = if let Ok(quantity) = other.extract::<PyRef<'_, PyQuantity>>() {
            (&self.inner / &quantity.inner)?
        } else {
            self.inner.divide_scalar(other.extract::<f64>()?)?
        };
        Ok(PyQuantity { inner })
    }

    fn __rtruediv__(&self, other: f64) -> PyResult<PyQuantity> {
        let lhs = Quantity::new(
            UncertainValue::new_independent(other, 0.0),
            crate::unit::Unit::new(std::collections::HashMap::new()),
        );
        Ok(PyQuantity {
            inner: (&lhs / &self.inner)?,
        })
    }

    fn __pos__(&self) -> PyResult<PyQuantity> {
        Ok(PyQuantity {
            inner: self.inner.positive()?,
        })
    }

    fn __neg__(&self) -> PyResult<PyQuantity> {
        Ok(PyQuantity {
            inner: self.inner.negative()?,
        })
    }

    fn __abs__(&self) -> PyResult<PyQuantity> {
        Ok(PyQuantity {
            inner: self.inner.abs()?,
        })
    }

    fn to(&self, unit: &PyUnit) -> PyResult<PyQuantity> {
        Ok(PyQuantity {
            inner: self.inner.to(&unit.inner)?,
        })
    }

    fn __pow__(&self, n: f64, modulo: Option<&Bound<'_, PyAny>>) -> PyResult<PyQuantity> {
        if modulo.is_some() {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "pow() with modulo is not supported for Quantity",
            ));
        }
        Ok(PyQuantity {
            inner: self.inner.pow(n)?,
        })
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
            ValueType::Array(v) => {
                let unc = self.inner.value().uncertainty();
                match unc {
                    ValueType::Array(u) => format!("{} ± {}, shape=({},)", v, u, v.len()),
                    ValueType::Scalar(u) => format!("{} ± {}, shape=({},)", v, u, v.len()),
                }
            }
        };
        format!("Quantity({}, {})", value_str, self.inner.unit())
    }
}

use crate::quantity::Quantity;
use crate::uncertain_value::{PyUncertainValue, UncertainValue};
use crate::unit_python::{PyUnit, PyUnitRegistry};
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

    /// Convert to coherent SI units, preserving uncertainty and correlations.
    /// Absolute temperatures include their offset; temperature differences do not.
    fn to_base_unit(&self) -> PyResult<PyQuantity> {
        Ok(PyQuantity {
            inner: self.inner.to_base_unit()?,
        })
    }

    /// Select an array element or slice, retaining the unit and uncertainty sources.
    fn __getitem__(&self, index: &Bound<'_, PyAny>) -> PyResult<PyQuantity> {
        Ok(PyQuantity {
            inner: Quantity::new(
                crate::uncertain_value::index_value(self.inner.value(), index)?,
                self.inner.unit().clone(),
            ),
        })
    }

    /// Assign quantities through a boolean mask, converting to the target unit.
    fn __setitem__(slf: &Bound<'_, Self>, mask: &Bound<'_, PyAny>, replacement: &Bound<'_, Self>) -> PyResult<()> {
        let result = {
            let current = slf.try_borrow()?;
            let replacement = replacement.try_borrow()?;
            let converted = replacement.inner.to(current.inner.unit())?;
            Quantity::new(
                crate::uncertain_value::assign_value(current.inner.value(), mask, converted.value())?,
                current.inner.unit().clone(),
            )
        };
        slf.try_borrow_mut()?.inner = result;
        Ok(())
    }

    /// Standard deviation with N-ddof; absolute affine inputs yield delta units.
    /// Shares UncertainValue.std's validation and first-order uncertainty semantics.
    #[pyo3(signature = (ddof=0.0))]
    fn std(&self, ddof: f64) -> PyResult<Self> {
        let unit = self.inner.unit().subtraction(self.inner.unit())?.result;
        let value = self.inner.value().std(ddof).map_err(pyo3::exceptions::PyValueError::new_err)?;
        Ok(Self { inner: Quantity::new(value, unit) })
    }

    /// Median by nominal value, retaining the unit and selected source identities.
    /// Ties retain input order, including for absolute affine quantities.
    fn median(&self) -> PyResult<Self> {
        Ok(Self { inner: Quantity::new(self.inner.value().median().map_err(pyo3::exceptions::PyValueError::new_err)?, self.inner.unit().clone()) })
    }

    /// Arithmetic mean, retaining the unit (including absolute affine units).
    fn mean(&self) -> PyResult<Self> {
        Ok(Self { inner: Quantity::new(self.inner.value().mean().map_err(pyo3::exceptions::PyValueError::new_err)?, self.inner.unit().clone()) })
    }

    /// Natural logarithm after converting to dimensionless units.
    fn log(&self) -> PyResult<Self> {
        let unit = crate::unit::Unit::new(Default::default());
        let converted = self.inner.to(&unit)?;
        let value = converted.value().log().map_err(pyo3::exceptions::PyValueError::new_err)?;
        Ok(Self { inner: Quantity::new(value, unit) })
    }

    /// Sum all elements in this unit. Absolute affine units cannot be summed.
    fn sum(&self) -> PyResult<Self> {
        self.inner.unit().check_multiplicative("sum")?;
        Ok(Self { inner: Quantity::new(self.inner.value().sum(), self.inner.unit().clone()) })
    }

    /// Serialize this quantity's uncertainty sources and unit definitions.
    fn to_json(&self) -> PyResult<String> {
        self.inner.to_json().map_err(pyo3::exceptions::PyValueError::new_err)
    }

    /// Restore using an explicit registry; missing or changed definitions fail.
    #[staticmethod]
    fn from_json(json: &str, registry: &PyUnitRegistry) -> PyResult<Self> {
        Ok(Self {
            inner: Quantity::from_json(json, &registry.inner).map_err(pyo3::exceptions::PyValueError::new_err)?,
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

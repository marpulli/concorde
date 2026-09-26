use super::{NumpyArray1D, UncertainValue, ValueType};
use numpy::{PyArray1, PyArrayMethods, PyUntypedArray, PyUntypedArrayMethods};
use pyo3::{IntoPyObjectExt, prelude::*};

/// Python wrapper for UncertainValue
///
/// This struct provides Python bindings for the Rust UncertainValue type,
/// allowing it to be used from Python with numpy arrays or scalar values.
#[pyclass(name = "UncertainValue")]
pub struct PyUncertainValue {
    pub(crate) inner: UncertainValue,
}

impl PyUncertainValue {
    fn check_binary_shapes(&self, other: &Self) -> PyResult<()> {
        if let (ValueType::Array(lhs), ValueType::Array(rhs)) =
            (&self.inner.value, &other.inner.value)
        {
            // Arrays are one-dimensional; a length of one can broadcast.
            if lhs.len() != rhs.len() && lhs.len() != 1 && rhs.len() != 1 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Incompatible array lengths: {} and {}",
                    lhs.len(),
                    rhs.len(),
                )));
            }
        }
        Ok(())
    }
}

#[pymethods]
impl PyUncertainValue {
    /// Create a new UncertainValue from Python
    ///
    /// Args:
    ///     value: Either a float or a one-dimensional float64 numpy array
    ///     uncertainty: A scalar or, for array values, a float64 array of the same shape
    ///
    /// Returns:
    ///     UncertainValue: A new uncertain value instance
    ///
    /// Raises:
    ///     TypeError: If value or uncertainty has an unsupported type
    ///     ValueError: If value and uncertainty arrays have different shapes
    #[new]
    fn new(value: &Bound<'_, PyAny>, uncertainty: &Bound<'_, PyAny>) -> PyResult<Self> {
        // If both value and uncertainty are scalars, create a scalar UncertainValue
        if !value.is_instance_of::<PyUntypedArray>()
            && !uncertainty.is_instance_of::<PyUntypedArray>()
        {
            if let (Ok(val), Ok(unc)) = (value.extract::<f64>(), uncertainty.extract::<f64>()) {
                return Ok(Self {
                    inner: UncertainValue::new_independent(val, unc),
                });
            }
        }

        if let Ok(val_array) = value.cast::<PyArray1<f64>>() {
            let val_readonly = val_array.try_readonly().map_err(|_| {
                pyo3::exceptions::PyRuntimeError::new_err(
                    "Cannot create UncertainValue: the value array is currently in use by another operation",
                )
            })?;
            let unc_arc: NumpyArray1D = match uncertainty.cast::<PyUntypedArray>() {
                Ok(unc_array) => {
                    if val_array.shape() != unc_array.shape() {
                        return Err(pyo3::exceptions::PyValueError::new_err(format!(
                            "Value shape {:?} does not match uncertainty shape {:?}",
                            val_array.shape(),
                            unc_array.shape(),
                        )));
                    }
                    let unc_array = uncertainty.cast::<PyArray1<f64>>()?;
                    let unc_readonly = unc_array.try_readonly().map_err(|_| {
                        pyo3::exceptions::PyRuntimeError::new_err(
                            "Cannot create UncertainValue: the uncertainty array is currently in use by another operation",
                        )
                    })?;
                    unc_readonly.as_array().to_owned().into()
                }
                Err(_) => {
                    let unc = uncertainty.extract::<f64>()?;
                    numpy::ndarray::Array1::from_elem(val_array.shape()[0], unc).into_shared()
                }
            };
            let val_arc: NumpyArray1D = val_readonly.as_array().to_owned().into();

            return Ok(Self {
                inner: UncertainValue::new_independent_array(val_arc, unc_arc),
            });
        }

        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "value must be a scalar or a one-dimensional float64 numpy array; uncertainty must be a scalar or a matching float64 numpy array",
        ))
    }

    /// Get the value component
    ///
    /// Returns:
    ///     float or ndarray: The central value(s)
    #[getter]
    fn value(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        match &self.inner.value {
            ValueType::Scalar(v) => v.into_py_any(py),
            ValueType::Array(arr) => PyArray1::from_array(py, &arr).into_py_any(py),
        }
    }

    /// Get the uncertainty (standard deviation)
    ///
    /// Returns:
    ///     float or ndarray: The uncertainty/uncertainties
    #[getter]
    fn uncertainty(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let unc = self.inner.uncertainty();
        match unc {
            ValueType::Scalar(v) => v.into_py_any(py),
            ValueType::Array(arr) => PyArray1::from_array(py, &arr).into_py_any(py),
        }
    }

    /// Get the variable ID (if this is an independent variable)
    ///
    /// Returns:
    ///     int or None: The variable ID for independent variables, None for computed values
    #[getter]
    fn variable_id(&self) -> Option<String> {
        self.inner.variable_id.map(|u| u.to_string())
    }

    /// Check if this is an independent variable
    ///
    /// Returns:
    ///     bool: True if this is an independent variable, False if computed
    fn is_independent(&self) -> bool {
        self.inner.is_independent()
    }

    /// Multiply this uncertain value by another value
    ///
    /// Supports multiplication by:
    /// - Another UncertainValue (with uncertainty propagation)
    /// - A scalar float
    ///
    /// Args:
    ///     other: The value to multiply by
    ///
    /// Returns:
    ///     UncertainValue: The result with propagated uncertainty
    fn __mul__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        // Try to multiply with another UncertainValue
        if let Ok(other_uv) = other.extract::<PyRef<PyUncertainValue>>() {
            self.check_binary_shapes(&other_uv)?;
            return Ok(Self {
                inner: &self.inner * &other_uv.inner,
            });
        }

        // Try to multiply with a scalar
        if let Ok(scalar) = other.extract::<f64>() {
            return Ok(Self {
                inner: &self.inner * scalar,
            });
        }

        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Unsupported operand type for *",
        ))
    }

    /// Right multiplication (other * self)
    fn __rmul__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        // Multiplication is commutative
        self.__mul__(other)
    }

    /// Add two uncertain values or add a scalar
    fn __add__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        // Try to add with another UncertainValue
        if let Ok(other_uv) = other.extract::<PyRef<PyUncertainValue>>() {
            self.check_binary_shapes(&other_uv)?;
            return Ok(Self {
                inner: &self.inner + &other_uv.inner,
            });
        }

        // Try to add with a scalar
        if let Ok(scalar) = other.extract::<f64>() {
            // Create a zero-uncertainty scalar
            let scalar_uv = UncertainValue::new_independent(scalar, 0.0);
            return Ok(Self {
                inner: &self.inner + &scalar_uv,
            });
        }

        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Unsupported operand type for +",
        ))
    }

    fn __radd__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        self.__add__(other)
    }

    /// Subtract two uncertain values or subtract a scalar
    fn __sub__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        // Try to subtract another UncertainValue
        if let Ok(other_uv) = other.extract::<PyRef<PyUncertainValue>>() {
            self.check_binary_shapes(&other_uv)?;
            return Ok(Self {
                inner: &self.inner - &other_uv.inner,
            });
        }

        // Try to subtract a scalar
        if let Ok(scalar) = other.extract::<f64>() {
            let scalar_uv = UncertainValue::new_independent(scalar, 0.0);
            return Ok(Self {
                inner: &self.inner - &scalar_uv,
            });
        }

        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Unsupported operand type for -",
        ))
    }

    fn __rsub__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        // For scalar - UncertainValue
        if let Ok(scalar) = other.extract::<f64>() {
            let scalar_uv = UncertainValue::new_independent(scalar, 0.0);
            return Ok(Self {
                inner: &scalar_uv - &self.inner,
            });
        }

        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Unsupported operand type for -",
        ))
    }

    /// Divide two uncertain values or divide by a scalar
    fn __truediv__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        // Try to divide by another UncertainValue
        if let Ok(other_uv) = other.extract::<PyRef<PyUncertainValue>>() {
            self.check_binary_shapes(&other_uv)?;
            return Ok(Self {
                inner: &self.inner / &other_uv.inner,
            });
        }

        // Try to divide by a scalar
        if let Ok(scalar) = other.extract::<f64>() {
            let scalar_uv = UncertainValue::new_independent(scalar, 0.0);
            return Ok(Self {
                inner: &self.inner / &scalar_uv,
            });
        }

        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Unsupported operand type for /",
        ))
    }

    fn __rtruediv__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> {
        // For scalar / UncertainValue
        if let Ok(scalar) = other.extract::<f64>() {
            let scalar_uv = UncertainValue::new_independent(scalar, 0.0);
            return Ok(Self {
                inner: &scalar_uv / &self.inner,
            });
        }

        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Unsupported operand type for /",
        ))
    }

    /// Negate an uncertain value
    fn __neg__(&self) -> PyResult<Self> {
        Ok(Self {
            inner: -&self.inner,
        })
    }

    /// Raise to a scalar power, propagating derivatives (correlation-aware)
    fn __pow__(&self, n: f64, modulo: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        if modulo.is_some() {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "pow() with modulo is not supported for UncertainValue",
            ));
        }
        Ok(Self {
            inner: self.inner.pow(n),
        })
    }

    /// String representation of the uncertain value
    fn __repr__(&self) -> PyResult<String> {
        let unc = self.inner.uncertainty();

        Ok(match (&self.inner.value, &unc) {
            (ValueType::Scalar(v), ValueType::Scalar(u)) => {
                format!("UncertainValue({} ± {})", v, u)
            }
            (ValueType::Array(v), ValueType::Array(u)) => {
                format!("UncertainValue({} ± {}, shape=({},))", v, u, v.len())
            }
            (ValueType::Array(v), ValueType::Scalar(u)) => {
                format!("UncertainValue({} ± {}, shape=({},))", v, u, v.len())
            }
            _ => "UncertainValue(array(...) ± array(...))".to_string(),
        })
    }
}

// ============================================================================
// Future Python Methods
// ============================================================================
//
// As you add more operations to ops.rs, add corresponding Python bindings here:
//
// #[pymethods]
// impl PyUncertainValue {
//     fn __add__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> { ... }
//     fn __sub__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> { ... }
//     fn __truediv__(&self, other: &Bound<'_, PyAny>) -> PyResult<Self> { ... }
//     fn __neg__(&self) -> PyResult<Self> { ... }
//
//     #[getter]
//     fn uncertainty(&self, py: Python<'_>) -> PyResult<Py<PyAny>> { ... }
// }

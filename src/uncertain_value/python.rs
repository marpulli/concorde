use super::{NumpyArray1D, UncertainValue, ValueType};
use numpy::{PyArray1, PyArrayMethods};
use pyo3::{IntoPyObjectExt, prelude::*};

/// Python wrapper for UncertainValue
///
/// This struct provides Python bindings for the Rust UncertainValue type,
/// allowing it to be used from Python with numpy arrays or scalar values.
#[pyclass(name = "UncertainValue")]
pub struct PyUncertainValue {
    pub(crate) inner: UncertainValue,
}

#[pymethods]
impl PyUncertainValue {
    /// Create a new UncertainValue from Python
    ///
    /// Args:
    ///     value: Either a float or numpy array of values
    ///     uncertainty: Either a float or numpy array of uncertainties
    ///
    /// Returns:
    ///     UncertainValue: A new uncertain value instance
    ///
    /// Raises:
    ///     TypeError: If value and uncertainty are not both floats or both numpy arrays
    #[new]
    fn new(value: &Bound<'_, PyAny>, uncertainty: &Bound<'_, PyAny>) -> PyResult<Self> {
        // Try to extract as scalar first
        if let (Ok(val), Ok(unc)) = (value.extract::<f64>(), uncertainty.extract::<f64>()) {
            return Ok(Self {
                inner: UncertainValue::new_independent(val, unc),
            });
        }

        // Try to extract as numpy arrays
        if let (Ok(val_array), Ok(unc_array)) = (
            value.cast::<PyArray1<f64>>(),
            uncertainty.cast::<PyArray1<f64>>(),
        ) {
            // TODO: should I be unwrapping here? There could be a borrow error
            let val_readonly = val_array.try_readonly().unwrap();
            let unc_readonly = unc_array.try_readonly().unwrap();

            let val_arc: NumpyArray1D = val_readonly.as_array().to_owned().into();
            let unc_arc: NumpyArray1D = unc_readonly.as_array().to_owned().into();

            return Ok(Self {
                inner: UncertainValue::new_independent_array(val_arc, unc_arc),
            });
        }

        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "value and uncertainty must both be either floats or numpy arrays",
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

    /// String representation of the uncertain value
    fn __repr__(&self) -> PyResult<String> {
        let unc = self.inner.uncertainty();

        Ok(match (&self.inner.value, &unc) {
            (ValueType::Scalar(v), ValueType::Scalar(u)) => {
                format!("UncertainValue({} ± {})", v, u)
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

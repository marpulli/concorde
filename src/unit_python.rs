use crate::unit::{DefinedUnit, Unit};
use crate::unit_file_parser;
use crate::unit_registry::UnitRegistry;
use pyo3::prelude::*;
use std::collections::HashMap;

#[pyclass(name = "Unit")]
#[derive(Clone)]
pub struct PyUnit {
    pub(crate) inner: Unit,
}

#[pymethods]
impl PyUnit {
    fn __mul__(&self, other: &PyUnit) -> PyUnit {
        PyUnit {
            inner: self.inner.clone() * other.inner.clone(),
        }
    }

    fn __truediv__(&self, other: &PyUnit) -> PyUnit {
        PyUnit {
            inner: self.inner.clone() / other.inner.clone(),
        }
    }

    fn __pow__(&self, exponent: f64, _modulo: Option<u32>) -> PyUnit {
        PyUnit {
            inner: self.inner.pow(exponent),
        }
    }

    fn __repr__(&self) -> String {
        format!("Unit({})", self.inner)
    }

    fn __eq__(&self, other: &PyUnit) -> bool {
        // Two units are equal if they have the same components with the same exponents
        if self.inner.components.len() != other.inner.components.len() {
            return false;
        }
        for (unit, &exp) in &self.inner.components {
            match other.inner.components.get(unit) {
                Some(&other_exp) if (exp - other_exp).abs() < f64::EPSILON => {}
                _ => return false,
            }
        }
        true
    }

    fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        let mut names: Vec<_> = self
            .inner
            .components
            .iter()
            .map(|(u, &e)| (u.name.clone(), ordered_float::OrderedFloat(e)))
            .collect();
        names.sort_by(|a, b| a.0.cmp(&b.0));
        for (name, exp) in &names {
            name.hash(&mut hasher);
            exp.hash(&mut hasher);
        }
        hasher.finish()
    }

    #[getter]
    fn components(&self) -> HashMap<String, f64> {
        self.inner
            .components
            .iter()
            .map(|(unit, &exp)| (unit.name.clone(), exp))
            .collect()
    }

    fn get_exponent(&self, name: &str) -> Option<f64> {
        self.inner.get_exponent(&name.to_string())
    }
}

#[pyclass(name = "UnitRegistry")]
pub struct PyUnitRegistry {
    inner: UnitRegistry,
}

#[pymethods]
impl PyUnitRegistry {
    #[new]
    fn new() -> Self {
        PyUnitRegistry {
            inner: UnitRegistry::new_with_si(),
        }
    }

    fn parse(&self, unit_string: &str) -> PyResult<PyUnit> {
        self.inner
            .parse_string(unit_string.to_string())
            .map(|unit| PyUnit { inner: unit })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    }

    #[pyo3(signature = (name, dimensions, scale, aliases=None))]
    fn define_unit(
        &mut self,
        name: &str,
        dimensions: [f64; 7],
        scale: f64,
        aliases: Option<Vec<String>>,
    ) -> PyResult<()> {
        let unit = DefinedUnit::new(
            name.to_string(),
            dimensions,
            scale,
            aliases.unwrap_or_default(),
        );
        self.inner.define_unit(unit);
        Ok(())
    }

    fn load_definitions(&mut self, path: &str) -> PyResult<usize> {
        unit_file_parser::load_from_file(&mut self.inner, path)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))
    }

    fn load_definitions_from_string(&mut self, toml_str: &str) -> PyResult<usize> {
        unit_file_parser::load_from_string(&mut self.inner, toml_str)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))
    }
}

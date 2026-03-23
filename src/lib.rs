mod parser;
mod quantity;
mod quantity_python;
mod tokenizer;
mod uncertain_value;
mod unit;
mod unit_python;
mod unit_registry;

use pyo3::prelude::*;
use uncertain_value::PyUncertainValue;

pyo3::create_exception!(_concorde, IncompatibleUnitError, pyo3::exceptions::PyException);

#[pymodule]
fn _concorde(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyUncertainValue>()?;
    m.add_class::<unit_python::PyUnit>()?;
    m.add_class::<unit_python::PyUnitRegistry>()?;
    m.add_class::<quantity_python::PyQuantity>()?;
    m.add(
        "IncompatibleUnitError",
        m.py().get_type::<IncompatibleUnitError>(),
    )?;
    Ok(())
}

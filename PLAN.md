Implementation Plan: High-Performance Unit Library
This document outlines the development plan for creating a Rust-based, strongly-typed Python library for unit-aware calculations with uncertainty propagation.

Phase 1: Core Rust Crate (unit-core)
Goal: Implement all core logic in a self-contained Rust library. No Python bindings yet.

[ ] Task 1.1: Define the internal Unit representation.

[ ] Create a Unit struct.

[ ] The primary field will be a fixed-size array representing the exponents of the base SI units (e.g., [f64; 7] for [m, kg, s, A, K, mol, cd]).

[ ] Add a scale_factor: f64 field to handle conversions (e.g., for kilometer, the scale is 1000.0).

[ ] Implement PartialEq to check for dimensional equality.

[ ] Task 1.2: Implement the UnitParser.

[ ] Choose a parsing library (e.g., pest for its grammar-based approach).

[ ] Define a grammar (.pest file) to handle expressions like kg * m / s\*\*2, kilometer, N*m.

[ ] Write Rust code to traverse the parsed syntax tree and construct a Unit struct. This will involve resolving symbols like "kg" to their base dimensions.

[ ] Task 1.3: Create the UnitRegistry struct.

[ ] The core will be a HashMap<String, Unit> to store definitions (e.g., "N" -> Unit { dimensions: [1, 1, -2, ...], scale: 1.0 }).

[ ] Implement a define(&mut self, definition: &str) method that parses a string like "N = kg \* m / s\*\*2" and populates the map.

[ ] Implement a parse(&self, unit_str: &str) method that uses the UnitParser and the internal map to resolve any string into a final Unit struct.

[ ] Task 1.4: Implement the core Quantity struct.

[ ] Create an enum ValueHolder { Scalar(f64), Array(ndarray::ArrayD<f64>) } to hold the magnitude.

[ ] Create the Quantity struct containing:

magnitude: ValueHolder

std_dev: Option<ValueHolder>

unit: Unit

[ ] Implement methods for basic construction.

[ ] Task 1.5: Implement arithmetic and uncertainty propagation logic.

[ ] Implement std::ops::Add, Sub, Mul, Div for &Quantity.

[ ] Addition/Subtraction:

Check that unit fields are dimensionally equal. If not, return Err.

Perform the operation on the magnitudes.

Propagate uncertainty: new_std_dev = sqrt(a.std_dev^2 + b.std_dev^2).

[ ] Multiplication/Division:

Calculate the new Unit by adding/subtracting the dimension arrays.

Perform the operation on the magnitudes.

Propagate uncertainty (e.g., for A * B): new_std_dev = |A*B| \* sqrt((a.std_dev/A)^2 + (b.std_dev/B)^2).

[ ] Implement a pow(self, exponent: i32) method for exponentiation, including unit and uncertainty updates.

[ ] Write comprehensive Rust unit tests (#[test]) for all logic.

Phase 2: Python Bindings with PyO3 & Maturin
Goal: Expose the Rust core to Python as a native module.

[ ] Task 2.1: Set up the project with maturin.

[ ] Run maturin new --bindings pyo3 to create the project structure.

[ ] Add pyo3, numpy, and ndarray to Cargo.toml.

[ ] Task 2.2: Expose the Quantity and UQuantity classes.

[ ] Add #[pyclass] to the Rust Quantity struct.

[ ] Implement a #[new] method that accepts a PyObject for the magnitude.

Inside, perform runtime checks to see if the object is a float or numpy.ndarray.

Accept an Optional PyObject for std_dev.

Accept a String for the unit.

[ ] Use getter properties to expose magnitude, std_dev, and unit to Python. The unit property will convert the internal Unit struct back to a string representation.

[ ] Implement **repr** for clean printing.

[ ] The distinction between Quantity and UQuantity will be handled in the .pyi file; the Rust class can be singular.

[ ] Task 2.3: Expose the UnitRegistry class.

[ ] Add #[pyclass] to the Rust UnitRegistry struct.

[ ] Implement a #[new] method.

[ ] Expose the define method.

[ ] Add a factory method (e.g., make_quantity) that takes magnitude, unit string, etc., and returns a Python Quantity object. This will be the user-facing ureg.Quantity(...).

[ ] Task 2.4: Implement Python operators.

[ ] In the #[pymethods] block for Quantity, implement **add**, **sub**, **mul**, **truediv**, and **pow**.

[ ] These methods will call the underlying Rust std::ops implementations and handle converting the results (or errors) back to Python types.

[ ] Task 2.5: Implement Serialization.

[ ] Implement **getstate** and **setstate** methods to allow for pickle support.

[ ] **getstate** should return a tuple of Python objects (e.g., magnitude, std_dev, unit string).

[ ] **setstate** will take this tuple and reconstruct the Rust object.

Phase 3: Python Typing and Packaging
Goal: Create a polished, statically-typed package for users.

[ ] Task 3.1: Create the .pyi stub file.

[ ] Create the package structure my_unit_lib/**init**.pyi.

[ ] Define Magnitude = TypeVar("Magnitude", float, np.ndarray).

[ ] Define the class Quantity(Generic[Magnitude]) with all its methods and properties, using ... for bodies.

[ ] Define class UQuantity(Quantity[Magnitude]) that inherits from Quantity and overrides std_dev to be non-optional.

[ ] Use @overload on all arithmetic operators (**add**, etc.) to correctly type-hint the results of scalar-scalar, array-array, and scalar-array operations.

[ ] Define the UnitRegistry class interface.

[ ] Task 3.2: Configure the build and packaging.

[ ] Create an empty my_unit_lib/py.typed marker file.

[ ] Update pyproject.toml to tell maturin where the Python module source is located so it includes the .pyi and py.typed files in the wheel.

Phase 4: Testing and Documentation
Goal: Ensure correctness, robustness, and usability.

[ ] Task 4.1: Write comprehensive Python integration tests.

[ ] Use pytest to create a tests/ directory.

[ ] Write tests that cover every use case from the unit_library_examples.py file.

[ ] Test scalar, array, and mixed operations.

[ ] Test uncertainty propagation against known values.

[ ] Test unit definition and parsing.

[ ] Test that dimensionality errors are raised correctly.

[ ] Task 4.2: Set up static analysis in CI.

[ ] Create a GitHub Actions workflow.

[ ] Add a step to run mypy on the examples and tests to ensure type hints are correct and valid.

[ ] Task 4.3: Write user documentation.

[ ] Create a detailed README.md with installation instructions and basic usage.

[ ] Add a section explaining the statically-typed features (Quantity[float], UQuantity) with examples.

[ ] Document any limitations (e.g., supported number types).

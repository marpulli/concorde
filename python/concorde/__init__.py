"""
Concorde: A Python package for physical quantities with units and uncertainties.

This package provides classes for working with uncertain values and physical units,
with Rust-accelerated core functionality.
"""

# Import the Rust extension module
from concorde._concorde import UncertainValue, Unit, UnitRegistry, Quantity, IncompatibleUnitError

# Re-export for public API
__all__ = [
    "UncertainValue",
    "Unit",
    "UnitRegistry",
    "Quantity",
    "IncompatibleUnitError",
]

__version__ = "0.1.0"

"""Check typing resources in the built wheel, not the editable installation."""

import sys
from pathlib import Path
from zipfile import ZipFile


wheel_directory = Path(sys.argv[1])
(wheel,) = wheel_directory.glob("*.whl")
with ZipFile(wheel) as archive:
    assert "concorde/py.typed" in archive.namelist()
    stub = archive.read("concorde/_concorde.pyi").decode()
    for name in ("Unit", "UnitRegistry", "Quantity", "UncertainValue", "IncompatibleUnitError", "AffineUnitError"):
        assert f"class {name}" in stub, f"Missing stub for {name} in {wheel.name}"
print(f"Typing resources verified in {wheel.name}")

"""Runs the doctests in every capability module and the submodules under them.
`pamoja` is a namespace package spread over one distribution per capability, so
the modules are found by their package directories rather than by walking one
tree, and each is imported as it is installed. `doctest.testmod` stops at the
module it is given, so a submodule is named and run in its own right."""

import doctest
import importlib
import pathlib

import pytest

PACKAGES = pathlib.Path(__file__).resolve().parents[1] / "packages"
CAPABILITIES = [
    (f"pamoja.{module.name}", module)
    for package in PACKAGES.iterdir()
    for module in (package / "pamoja").glob("*")
    if module.is_dir() and not module.name.startswith("_")
]
MODULES = sorted(
    {name for name, _ in CAPABILITIES}
    | {
        f"{name}.{source.stem}"
        for name, directory in CAPABILITIES
        for source in directory.glob("*.py")
        if not source.stem.startswith("_")
    }
)


@pytest.mark.parametrize("name", MODULES)
def test_doctests_pass(name: str) -> None:
    module = importlib.import_module(name)
    result = doctest.testmod(module, optionflags=doctest.ELLIPSIS)
    assert result.failed == 0, f"{result.failed} doctest(s) failed in {name}"

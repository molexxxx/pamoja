"""Runs the doctests in every capability module. `pamoja` is a namespace package
spread over one distribution per capability, so the modules are found by their
package directories rather than by walking one tree, and each is imported as it
is installed."""

import doctest
import importlib
import pathlib

import pytest

PACKAGES = pathlib.Path(__file__).resolve().parents[1] / "packages"
MODULES = sorted(
    f"pamoja.{module.name}"
    for package in PACKAGES.iterdir()
    for module in (package / "pamoja").glob("*")
    if module.is_dir() and not module.name.startswith("_")
)


@pytest.mark.parametrize("name", MODULES)
def test_doctests_pass(name: str) -> None:
    module = importlib.import_module(name)
    result = doctest.testmod(module, optionflags=doctest.ELLIPSIS)
    assert result.failed == 0, f"{result.failed} doctest(s) failed in {name}"

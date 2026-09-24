"""Runs every guide example under guides/ as a script. Each one is spliced into a
page of the documentation site by `cargo xtask docs`, so every Python example the
site shows is code that ran here."""

import pathlib
import runpy

import pytest

GUIDES = sorted((pathlib.Path(__file__).resolve().parents[1] / "guides").glob("*.py"))

#: The repository root, which every guide runs from, as a clone runs it, so a guide reads a
#: shipped file such as ``profiles/brooder-heater.json`` by the path a reader would type.
ROOT = pathlib.Path(__file__).resolve().parents[3]


@pytest.mark.parametrize("guide", GUIDES, ids=[guide.stem for guide in GUIDES])
def test_guide_runs(guide: pathlib.Path, monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.chdir(ROOT)
    runpy.run_path(str(guide), run_name="__main__")

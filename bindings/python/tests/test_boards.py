"""Loads every board program under boards/ without running it. A board program drives real
pins and radios, so it cannot run here, but loading it resolves every import and name at
the top of the module, which is where a renamed or missing API shows up. Each one is
spliced into a board page of the documentation site by `cargo xtask docs`."""

import pathlib
import runpy

import pytest

BOARDS = sorted((pathlib.Path(__file__).resolve().parents[1] / "boards").rglob("*.py"))


@pytest.mark.parametrize("board", BOARDS, ids=[board.stem for board in BOARDS])
def test_board_program_loads(board: pathlib.Path) -> None:
    program = runpy.run_path(str(board), run_name="board")
    assert callable(program["main"]), f"{board.name} has no main()"

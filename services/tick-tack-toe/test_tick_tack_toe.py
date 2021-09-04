import pytest
import gradesta_service

from tick_tack_toe import Board


@pytest.fixture
def middle_board():
    return Board(None, 0, "gradesta://example.com/en/tick-tack-toe/board/x o.o x. x ")


def test_load(middle_board):
    u, f = middle_board.load()
    assert u.dataUpdates[0].data == "x o\no x\n x ".encode("utf8")
    assert u.portUpdates[0].direction == -1
    # assert u.portUpdates[1].direction == 1

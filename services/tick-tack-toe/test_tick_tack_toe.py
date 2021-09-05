import pytest
import gradesta_service

from tick_tack_toe import Board, NextMove


@pytest.fixture
def middle_board():
    return Board("x o.o x. x ")


def test_load(middle_board):
    pass
    # u, f = middle_board.load()
    # assert u.dataUpdates[0].data == "x o\no x\n x ".encode("utf8")
    # assert u.portUpdates[0].direction == -1
    # assert u.portUpdates[1].direction == 1


def test_next_move():
    nm = NextMove("x o.o x. x ", 1)
    assert nm.whos_move() == "o"
    assert nm.path == "next/1"
    assert nm.place("ö") == "xöo.o x. x "
    nm.move = 2
    assert nm.place("ö") == "x o.oöx. x "
    assert (
        nm.draw()
        == "x o\n\
oöx\n\
 x "
    )
    assert nm.right() == "x o.oox. x /prev/3"
    assert nm.marker() == "ö"

import pytest
import gradesta_service

from tick_tack_toe import Board, NextMove, PreviousMove


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


def test_previous_move():
    m = PreviousMove("x o.o x. x ", 1)
    assert m.path == "prev/1"
    assert m.whos_move() == "x"
    assert m.marker() == "X"
    assert m.place("X") == "X o.o x. x "
    m.move = 3
    assert m.place("X") == "x o.o x. X "
    assert m.left() == "x o.o x.   /next/4"
    assert m.draw() == "x o\no x\n X "


def test_board():
    b = Board("x o.o x. x ")
    assert b.path == ""
    assert b.draw() == "x o\no x\n x "

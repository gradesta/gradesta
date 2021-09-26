import pytest
import gradesta_service

from tick_tack_toe import (
    Board,
    NextMove,
    PreviousMove,
    BoardsAndMoves,
    who_won,
    TickTackToe,
)

from cell_reference_db import CellReferenceDB
from gradesta_locales import Localizer


@pytest.fixture
def localizer():
    return Localizer("en", "tick-tack-toe")


def test_resolve(localizer):
    ttt = TickTackToe()
    bnm = BoardsAndMoves(1, localizer, "x o.o x. x ")
    board = bnm.resolve("")
    assert board.draw() == "x o\no x\n x "
    board1 = ttt.resolve("x o.o x. x /", 1)
    assert board1.draw() == "x o\no x\n x "
    board2 = ttt.resolve("x o.o x. x /", 1)
    assert board1 == board2
    board3 = ttt.resolve("x o.o x. x /", 2)
    assert board2 != board3


def test_load(localizer):
    ttt = TickTackToe()
    bnm = BoardsAndMoves(1, localizer, "x o.o x. x ")
    pbnm = gradesta_service.ProtocolPage(ttt, "^en/tick-tack-toe/x o.o x. x /", bnm)
    u, fs = pbnm.load([""])
    assert u["dataUpdates"][0].data == "x o\no x\n x ".encode("utf8")
    assert len(u["portUpdates"]) == 2
    assert u["portUpdates"][0].direction == 1
    assert u["portUpdates"][1].direction == -1
    updates, fs = pbnm.load(["Next/2"])
    assert updates["dataUpdates"][0].data == "x o\noöx\n x ".encode("utf8")
    assert len(updates["portUpdates"]) == 3
    left = [l for l in updates["portUpdates"] if l.direction == 2][0]
    assert left.connectedVertex.symlink.address == "x o.oox. x /Previous/3"
    up = [l for l in updates["portUpdates"] if l.direction == -1][0]
    assert type(up.connectedVertex.vertex) == int
    down = [l for l in updates["portUpdates"] if l.direction == 1][0]
    assert type(down.connectedVertex.vertex) == int


def test_next_move(localizer):
    bnm = BoardsAndMoves(1, localizer, "x o.o x. x ")
    nm = NextMove(bnm, 1)
    assert nm.whos_move() == "o"
    assert nm.path == "Next/1"
    assert nm.place("ö") == "xöo.o x. x "
    nm.move = 2
    assert nm.place("ö") == "x o.oöx. x "
    assert (
        nm.draw()
        == "x o\n\
oöx\n\
 x "
    )
    assert nm.right() == {"symlink": ("x o.oox. x /Previous/3", 1)}
    assert nm.marker() == "ö"


def test_previous_move(localizer):
    bnm = BoardsAndMoves(1, localizer, "x o.o x. x ")
    m = PreviousMove(bnm, 1)
    assert m.path == "Previous/1"
    assert m.whos_move() == "x"
    assert m.marker() == "X"
    assert m.place("X") == "X o.o x. x "
    m.move = 3
    assert m.place("X") == "x o.o x. X "
    assert m.left() == {"symlink": ("x o.o x.   /Next/4", 1)}
    assert m.draw() == "x o\no x\n X "


def test_board(localizer):
    bnm = BoardsAndMoves(1, localizer, "x o.o x. x ")
    b = Board(bnm)
    assert b.path == ""
    assert b.draw() == "x o\no x\n x "
    bnmxw = BoardsAndMoves(1, localizer, "xxx.o o.   ")
    xwins = Board(bnmxw)
    assert xwins.draw() == "X wins!\nxxx\no o\n   "


def test_boards_and_moves(localizer):
    ttt = TickTackToe()
    bnm = BoardsAndMoves(1, localizer, "x o.o x. x ")
    assert len(bnm.prev_moves()) == 3
    assert [m.move for m in bnm.prev_moves()] == [1, 2, 3]
    assert len(bnm.next_moves()) == 4
    assert [m.move for m in bnm.next_moves()] == [1, 2, 3, 4]
    assert len(list(bnm.cells())) == 8
    assert [c[0] for c in bnm.cells()] == ["P", "P", "P", "B", "N", "N", "N", "N"]
    start = BoardsAndMoves(1, localizer, "   .   .   ")
    assert len(start.prev_moves()) == 0
    assert len(start.next_moves()) == 9
    xwins = BoardsAndMoves(1, localizer, "x o. x .o x")
    assert len(xwins.prev_moves()) == 3
    assert len(xwins.next_moves()) == 0


def test_who_won():
    assert "x" == who_won("xxx.   .oo ")
    assert "o" == who_won("xxo. o .oxx")
    assert None == who_won("xxo.o x.   ")

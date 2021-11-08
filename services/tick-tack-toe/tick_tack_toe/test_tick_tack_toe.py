import pytest
import capnp
from gradestalib import gradesta_service
import gradestalib.parse_address as parse_address
from gradesta_level0.level0 import level0

from tick_tack_toe import (
    Board,
    NextMove,
    PreviousMove,
    BoardsAndMoves,
    who_won,
    TickTackToe,
)

from gradestalib.cell_reference_db import CellReferenceDB
from gradestalib.gradesta_locales import Localizer


@pytest.fixture
def localizer():
    return Localizer("en", "tick-tack-toe")


def test_load_vertex(localizer):
    ttt = TickTackToe()
    bnm = BoardsAndMoves(1, localizer, "x o.o x. x ")
    board = bnm.resolve("")
    assert board.draw() == "x o\no x\n x "
    b1a = parse_address.parse_address("gradesta://example.com/en/tick-tack-toe/x o.o x. x /")
    b1a.identity = 1
    board1 = ttt.load_vertex(b1a)
    assert board1.cell.draw() == "x o\no x\n x "
    board2 = ttt.load_vertex(b1a)
    assert board1 == board2
    b3a = parse_address.parse_address("gradesta://example.com/en/tick-tack-toe/x o.o x. x /")
    b3a.identity = 2
    board3 = ttt.load_vertex(b3a)
    assert board2 != board3


def test_load(localizer):
    ttt = TickTackToe()
    bnm = BoardsAndMoves(1, localizer, "x o.o x. x ")
    pbnm = gradesta_service.ProtocolPage(ttt, bnm, ["x o.o x. x "])
    _ = pbnm.load("", 1, parse_address.parse_address("gradesta://example.com/en/tick-tack-toe/x o.o x. x /"))
    qmfc = ttt.queued_message.forClient
    assert qmfc.dataUpdates[0].data == "x o\no x\n x ".encode("utf8")
    assert len(qmfc.portUpdates) == 2
    assert qmfc.portUpdates[0].direction == 1
    assert qmfc.portUpdates[1].direction == -1
    ttt.reset_queue()
    _ = pbnm.load("Next/2", 2,  parse_address.parse_address("gradesta://example.com/en/tick-tack-toe/x o.o x. x /Next/2"))
    qmfc = ttt.queued_message.forClient
    assert qmfc.dataUpdates[0].data == "x o\noöx\n x ".encode("utf8")
    assert len(qmfc.portUpdates) == 3
    left = [l for l in qmfc.portUpdates if l.direction == 2][0]
    assert "/".join(left.connectedVertex.symlink.vertexPath) == "x o.oox. x /Previous/3"
    up = [l for l in qmfc.portUpdates if l.direction == -1][0]
    assert type(up.connectedVertex.vertex) == capnp.lib.capnp._DynamicStructBuilder
    down = [l for l in qmfc.portUpdates if l.direction == 1][0]
    assert type(down.connectedVertex.vertex) == capnp.lib.capnp._DynamicStructBuilder


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

import pytest
import gradesta_service

from tick_tack_toe import Board, NextMove, PreviousMove, BoardsAndMoves, who_won, TickTackToe


@pytest.fixture
def middle_board():
    return Board(1, "x o.o x. x ")


def test_resolve(middle_board):
    bnm = BoardsAndMoves(1, "x o.o x. x ")
    board = bnm.resolve("")
    assert board.draw() == "x o\no x\n x "
    ttt = TickTackToe()
    board1 = ttt.resolve("x o.o x. x /", 1)
    assert board1.draw() == "x o\no x\n x "
    board2 = ttt.resolve("x o.o x. x /", 1)
    assert board1 == board2
    board3 = ttt.resolve("x o.o x. x /", 2)
    assert board2 != board3

    #assert u.dataUpdates[0].data == "x o\no x\n x ".encode("utf8")
    #assert u.portUpdates[0].direction == -1
    #assert u.portUpdates[1].direction == 1


def test_next_move():
    nm = NextMove(1, "x o.o x. x ", 1)
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
    m = PreviousMove(1, "x o.o x. x ", 1)
    assert m.path == "prev/1"
    assert m.whos_move() == "x"
    assert m.marker() == "X"
    assert m.place("X") == "X o.o x. x "
    m.move = 3
    assert m.place("X") == "x o.o x. X "
    assert m.left() == "x o.o x.   /next/4"
    assert m.draw() == "x o\no x\n X "


def test_board():
    b = Board(1, "x o.o x. x ")
    assert b.path == ""
    assert b.draw() == "x o\no x\n x "
    xwins = Board(1, "xxx.o o.   ")
    assert xwins.draw() == "X wins!\nxxx\no o\n   "


def test_boards_and_moves():
    bnm = BoardsAndMoves(1, "x o.o x. x ")
    assert len(bnm.prev_moves()) == 3
    assert [m.move for m in bnm.prev_moves()] == [1,2,3]
    assert len(bnm.next_moves()) == 4
    assert [m.move for m in bnm.next_moves()] == [1,2,3,4]
    assert len(list(bnm.cells())) == 8
    assert [c[0] for c in bnm.cells()] == ["P", "P", "P", "B", "N", "N", "N", "N"]
    start = BoardsAndMoves(1, "   .   .   ")
    assert len(start.prev_moves()) == 0
    assert len(start.next_moves()) == 9
    xwins = BoardsAndMoves(1, "x o. x .o x")
    assert len(xwins.prev_moves()) == 3
    assert len(xwins.next_moves()) == 0


def test_who_won():
    assert "x" == who_won("xxx.   .oo ")
    assert "o" == who_won("xxo. o .oxx")
    assert None == who_won("xxo.o x.   ")

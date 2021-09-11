#!/usr/bin/python3
import gradesta_service
from dataclasses import dataclass

from typing import Optional


def who_won(board: Optional[str]):
    """
    Returns "x" if x won, "o" if o won and None if the win condition has not been met.
    """
    def check_letter(letter: str):
        if letter * 3 in board:
            return letter
        if board[5] == letter:
            if board[0] == board[10] == letter:
                return letter
            if board[2] == board[8] == letter:
                return letter
        return None
    x = check_letter("x")
    o = check_letter("o")
    return x or o


@dataclass
class Board(gradesta_service.Cell):
    pieces: str
    path: str = ""

    def draw(self):
        winner = who_won(self.pieces)
        head = ""
        if winner:
            head = "{} wins!\n".format(winner.upper())
        return head + self.pieces.replace(".", "\n")


@dataclass
class Move(gradesta_service.Cell):
    pieces: str
    move: int

    def draw(self):
        return self.place(self.marker()).replace(".", "\n")


@dataclass
class PreviousMove(Move):
    @property
    def path(self):
        return "prev/{}".format(self.move)

    def whos_move(self):
        xs = self.pieces.count("x")
        os = self.pieces.count("o")
        if os > xs:
            return "o"
        else:
            return "x"

    def marker(self):
        return self.whos_move().upper()

    def place(self, marker):
        ps = list(self.pieces)
        i = 0
        placed = 0
        while True:
            if ps[i] == self.whos_move():
                placed += 1
            if placed == self.move:
                ps[i] = marker
                return "".join(ps)
            i += 1

    def left(self):
        placed = self.place(self.marker())
        which_move = placed.split(self.marker())[0].count(" ") + 1
        return "{pieces}/next/{move}".format(pieces=self.place(" "), move=which_move)


class NextMove(Move):
    def whos_move(self):
        xs = self.pieces.count("x")
        os = self.pieces.count("o")
        if os > xs:
            return "x"
        else:
            return "o"

    @property
    def path(self):
        return "next/{}".format(self.move)

    def place(self, marker):
        ps = list(self.pieces)
        i = 0
        empties = 0
        while True:
            if ps[i] == " ":
                empties += 1
            if empties == self.move:
                ps[i] = marker
                return "".join(ps)
            i += 1

    def right(self):
        placed = self.place(self.marker())
        which_move = placed.split(self.marker())[0].count(self.whos_move()) + 1
        return "{pieces}/prev/{move}".format(
            pieces=self.place(self.whos_move()), move=which_move
        )

    def marker(self):
        return "ẍ" if self.whos_move() == "x" else "ö"


@dataclass
class BoardsAndMoves(gradesta_service.Page):
    pieces: str

    geometry = """
P
.
B
N
.
"""

    def prev_moves(self):
        xs = self.pieces.count("x")
        os = self.pieces.count("o")
        num_prev_moves = max(xs, os)
        return [
            PreviousMove(self.identity, self.pieces, move) for move in range(1, num_prev_moves + 1)
        ]

    def next_moves(self):
        if who_won(self.pieces) is not None:
            return []
        num_next_moves = self.pieces.count(" ")
        return [NextMove(self.identity, self.pieces, move) for move in range(1, num_next_moves + 1)]

    def cells(self):
        for prev_move in self.prev_moves():
            yield ("P", prev_move)
        yield ("B", Board(self.identity, self.pieces))
        for next_move in self.next_moves():
            yield ("N", next_move)


class TickTackToe(gradesta_service.Actor):
    service_name = "tick-tack-toe"
    pages = {"<pieces>/": BoardsAndMoves}

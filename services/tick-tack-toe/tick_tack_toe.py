#!/usr/bin/python3
import gradesta_service
import asyncio
from dataclasses import dataclass


@dataclass
class Board(gradesta_service.Cell):
    pieces: str
    path: str = ""

    def draw(self):
        self.pieces.replace(".", "\n")


@dataclass
class Move(gradesta_service.Cell):
    pieces: str
    move: int


@dataclass
class PreviousMove(Move):
    @property
    def path(self):
        return "prev/" + self.move

    def marker(self):
        return self.whos_move().upper()


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

    def draw(self):
        return self.place(self.marker()).replace(".", "\n")

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

    def vertexes(self):
        for prev_move in self.prev_moves():
            yield ("P", prev_move)
        yield ("B", Board(self.pieces))
        for next_move in self.next_moves():
            yield ("N", next_move)


class TickTackToe(gradesta_service.Actor):
    service_name = "tick-tack-toe"
    pages = {"<pieces>/", BoardsAndMoves}

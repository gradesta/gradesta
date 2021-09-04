#!/usr/bin/python3
import gradesta_service
import asyncio


class Board(gradesta_service.Vertex):
    def load(self):
        return super().load(
            data_mime="text/plain",
            data=self.address.split("/")[6].replace(".", "\n"),
            up=self.stage_cell(up_addr),
        )


class TickTackToe(gradesta_service.Actor):
    service_name = "tick-tack-toe"
    vertex_class = Board

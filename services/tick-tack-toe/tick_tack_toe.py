#!/usr/bin/python3
import gradesta_service
import asyncio


class Board(gradesta_service.Vertex):
    def load(self):
        u, f = super().load()
        u.init("dataUpdates", 1)
        u.dataUpdates[0] = self.data_update(
            "text/plain",
            self.address.split("/")[6].replace(".", "\n")
        )
        return u, f


class TickTackToe(gradesta_service.Actor):
    service_name = "tick-tack-toe"
    vertex_class = Board

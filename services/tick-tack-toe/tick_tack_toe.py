#!/usr/bin/python3
import gradesta_service

class TickTackToe(gradesta_service.Actor):
    service_name = "tick-tack-toe"

    def get_vertex_class(self):
        return Board


class Board(gradesta_service.Vertex):
    @property
    def data(self):
        return self.address["internal"]["state"]

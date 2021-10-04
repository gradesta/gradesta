import zmq
import os
import sys
import pathlib
import capnp
import asyncio
from concurrent.futures import *
from dataclasses import dataclass

import level0_capnp as level0
from paths import match_path
from cell_reference_db import CellReferenceDB
import simple_topology_expressions
from gradesta_locales import Localizer

from typing import *


@dataclass
class ProtocolPage:
    actor: "Actor"
    address: str
    page: "Page"

    def load(self, paths: List[str]) -> Tuple[DefaultDict[str, List[any]], Set[Future]]:
        updates: DefaultDict[str, List[any]] = forClientTemplate()
        futures: Set[Future] = set()
        directions = {
            "up": -1,
            "down": 1,
            "left": -2,
            "right": 2,
        }
        cells = set()
        for path in paths:
            cell = self.page.resolve(path)
            pcell = ProtocolCell(self, cell)
            cells.add(pcell.cid)
            updates["vertexes"].append(pcell.vertex())
            updates["vertexStates"].append(pcell.vertexState())
            du = pcell.data_update()
            if du is not None:
                updates["dataUpdates"].append(du)
            for direction, val in directions.items():
                try:
                    custom_direction_value = eval("cell." + direction + "()")
                except AttributeError:
                    custom_direction_value = None
                if custom_direction_value is not None:
                    updates["portUpdates"].append(
                        pcell.port_update(val, **custom_direction_value)
                    )
        for (pcell1, pcell2, dim) in self.connections():
            if pcell1.cid in cells:
                updates["portUpdates"].append(
                    pcell1.port_update(dim, vertexId=pcell2.cid)
                )

        return updates, futures

    def connections(self) -> Iterator[Tuple["ProtocolCell", "ProtocolCell", int]]:
        connections = self.page.connections()
        for (cell1, cell2, dim) in connections:
            pcell1 = ProtocolCell(self, cell1)
            pcell2 = ProtocolCell(self, cell2)
            yield (pcell1, pcell2, dim)


@dataclass
class Page:
    identity: int
    localizer: Localizer

    def resolve(self, path):
        for ctype, cell in self.cells():
            if cell.path == path:
                return cell

    def connections(self) -> Iterator[Tuple["Cell", "Cell", int]]:
        cells = list(self.cells())
        connections = simple_topology_expressions.get_connections(
            self.layout, [a for (a, b) in cells]
        )
        for (index1, index2, dim) in connections:
            (_, cell1) = cells[index1]
            (_, cell2) = cells[index2]
            yield (cell1, cell2, dim)
            yield (cell2, cell1, -dim)


@dataclass
class ProtocolCell:
    page: "ProtocolPage"
    cell: "Cell"

    @property
    def cid(self):
        return self.page.actor.crdb.lookup_cell(self.address, self.cell.page.identity)[
            0
        ]

    @property
    def address(self):
        return self.page.address + self.cell.path

    @property
    def path(self):
        return self.page.page.path + self.cell.path

    def data_update(self):
        data = self.cell.draw()
        if type(data) is str:
            data = data.encode("utf8")
        data_mime = self.cell.data_mime()
        if data is not None and data_mime is not None:
            du = level0.DataUpdate()
            du.updateId = next(self.page.actor.updateCounter)
            du.vertexId = self.cid
            du.mime = data_mime
            du.data = data
            return du

    def port_update(
        self,
        direction: int,
        closed: bool = False,
        disconnected: bool = False,
        vertex: Union[bool, Tuple[str, int]] = False,
        symlink: Union[bool, Tuple[str, Tuple[str, int]]] = False,
    ):
        assert (
            len([x for x in [closed, disconnected, vertexId, symlink] if x is False])
            == 3
        )
        pu = level0.PortUpdate()
        pu.updateId = next(self.page.actor.updateCounter)
        pu.vertexId = self.cid
        pu.direction = direction
        if closed:
            pu.connectedVertex.closed = None
        if disconnected:
            pu.connectedVertex.disconnected = None
        if vertexId:
            pu.connectedVertex.vertex = vertexId
        if symlink:
            pu.connectedVertex.init("symlink")
            pu.connectedVertex.symlink.serviceAddress = symlink[0]
            pu.connectedVertex.symlink.path = level0.Path()
            pu.connectedVertex.symlink.path.path = symlink[1][0]
            pu.connectedVertex.symlink.path.identity = symlink[1][1]
        return pu

    def vertex(self):
        v = level0.Vertex()
        a = level0.Address()
        a. = self.path
        a.identity = self.cell.page.identity
        v.address = a
        v.instanceId = self.cid
        v.view = self.cell.view
        return v

    def vertexState(self):
        vs = level0.VertexState()
        vs.instanceId = self.cid
        vs.status = 200
        vs.reaped = False
        return vs


@dataclass
class Cell:
    page: "Page"

    def fmt(self, *args, **kwargs):
        return self.page.localizer.fmt(*args, **kwargs)

    def data_mime(self):
        return "text/plain"

    @property
    def view(self):
        return ""

    def recv(self, event):
        updates = level0.ForClient()
        futures = set()
        return updates, futures


class Selections:
    def __init__(self, service):
        self.service = service
        self.selections = {}
        self.vertexes = {}

    def select(self, selection, vertex):
        if selection not in self.selections:
            self.selections[selection] = set()
        self.selections[selection].add(vertex)
        if vertex not in self.vertexes:
            self.vertexes[vertex] = set()
        self.vertexes[vertex].add(selection)

    def deselect(self, selection):
        try:
            for vertex in self.selections[selection]:
                vertex_selections = self.vertexes[vertex]
                vertex_selections.remove(selection)
                if not vertex_selections:
                    self.service.vertexes[vertex].reap()
        except KeyError:
            sys.stderr.write(
                "Received deselect command for non-existant selection {}.\n".format(
                    selection
                )
            )
        del self.selections[selection]


class Actor:
    def __init__(self):
        self.crdb = CellReferenceDB()
        self.cells: DefaultDict[int, Cell] = {}

        def update_counter():
            uc = 0
            while True:
                uc -= 1
                yield uc

        self.updateCounter: Iterator[int] = update_counter()

    def start(self):
        self.vertex_counter = 0
        services_dir = os.path.expanduser("~/.cache/gradesta/services/sockets")
        pathlib.Path(services_dir).mkdir(parents=True, exist_ok=True)

        context = zmq.Context()
        socket = context.socket(zmq.REP)
        socket_path = "ipc://{services_dir}/{service_name}".format(
            services_dir=services_dir,
            service_name=self.service_name,
        )
        print("Connecting to socket {}".format(socket_path))
        socket.bind(socket_path)

        while True:
            message = level0_capnp.Message.from_bytes(socket.recv())
            fs = message.forService
            cell_message_types = {
                "vertexMessages": (lambda c: c.recv),
                "portUpdates": (lambda c: c.recv_port_update),
                "dataUpdates": (lambda c: c.recv_data_update),
                "encryptionUpdates": (lambda c: c.recv_encryption_update),
            }
            for cmv, call in cell_message_types.items():
                call(self.cells[eval("fs.{cmv}.vertexId".format(cmv=cmv))])(eval("fs.{cmv}".format(cmv=cmv)))
            self.send_queued()

    def send_queued(self):
        mfs = level0_capnp.Message(
            messages=self.queued_messages,
            vertexes=self.queued_vertexes,
            vertexStates=self.queued_vertex_states,
        )
        socket.send(mfs.to_bytes())
        self.reset_queue()

    def reset_queue(self):
        self.queued_messages = []
        self.queued_vertexes = []
        self.queued_vertex_states = []

    def recv_vertex_message(self, vm):
        try:
            self.vertexes[vm.vertexId].write(vm.data)
        except KeyError:
            sys.stderr.write(
                "Received message for vertex id {} but vertex does not exist.\n".format(
                    vm.vertexId
                )
            )

    def load_vertex(self, address):
        for vertex in self.vertexes:
            if address == vertex.address:
                return vertex.id
        id = self.next_id()
        self.vertexes[id] = create_vertex(id, address)

    def next_id(self):
        r = self.vertex_counter
        self.vertex_counter += 1
        return r

    def create_vertex(self, id, address):
        return self.vertex_class(self, id, address)

    def set_cursor(self, cursor):
        vid = self.load_vertex(cursor.address)
        self.selections.select(cursor.selectionId, vid)

    def move_cursor(self, cm):
        try:
            vertex = self.vertexes[cm.vertexId]
        except KeyError:
            sys.stderr.write(
                "Received move cursor command for non-existant vertex {}.\n".format(
                    cm.vertexId
                )
            )
        try:
            self.selections.select(
                cm.selectionId, vertex.ports[cm.direction].get_vertex().id
            )
        except KeyError:
            sys.stderr.write(
                "Received move cursor command for vertex {} in non-existant direction {}.\n".format(
                    cm.vertexId, cm.direction
                )
            )

    def resolve(self, path: str, identity: int) -> Cell:
        cid, created = self.crdb.lookup_cell(path, identity)
        localizer = Localizer(self.service_name)
        if created or cid not in self.cells:
            for pattern, page in self.pages.items():
                match = match_path(path, pattern)
                if match is not None:
                    cell = page(identity, localizer, **match[0]).resolve(match[1])
                    self.cells[cid] = cell
                    return cell
        return self.cells[cid]

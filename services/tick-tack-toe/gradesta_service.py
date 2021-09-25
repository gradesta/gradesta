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

from typing import *


def to_addr_field(k, v):
    af = level0.AddressField()
    af.name = k
    af.value = v
    return af


def address_fields_to_dict(address_fields):
    d = {}
    for field in address_fields:
        d[field.name] = field.value
    return d


def forClientTemplate() -> DefaultDict[str, List[any]]:
    fc = {}
    fields = [
        "vertexMessages",
        "vertexes",
        "vertexStates",
        "updateStatuses",
        "portUpdates",
        "dataUpdates",
        "encryptionUpdates",
        "cursors",
    ]
    for f in fields:
        fc[f] = []
    return fc


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
        for path in paths:
            cell = self.page.resolve(path)
            pcell = ProtocolCell(self, cell)
            v = level0.Vertex()
            a = level0.Address()
            a.address = self.address + cell.path
            a.identity = cell.identity
            v.address = a
            v.instanceId = pcell.cid
            v.view = cell.view
            updates["vertexes"].append(v)
            vs = level0.VertexState()
            vs.instanceId = pcell.cid
            vs.status = 200
            vs.reaped = False
            updates["vertexStates"].append(vs)
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

        return updates, futures


@dataclass
class Page:
    identity: int

    def resolve(self, path):
        for ctype, cell in self.cells():
            if cell.path == path:
                return cell


@dataclass
class ProtocolCell:
    page: "ProtocolPage"
    cell: "Cell"

    @property
    def cid(self):
        return self.page.actor.crdb.lookup_cell(self.address, self.cell.identity)[0]

    @property
    def address(self):
        return self.page.address + self.cell.path

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
        self, direction, closed=False, disconnected=False, vertexId=False, symlink=False
    ):
        assert (
            len([x for x in [closed, disconnected, vertexId, symlink] if x == False])
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
            pu.connectedVertex.symlink.address = symlink.address
            pu.connectedVertex.identity = symlink.identity
        return pu


@dataclass
class Cell:
    identity: int

    def data_mime(self):
        return "text/plain"

    @property
    def view(self):
        return ""


class Vertex:
    def __init__(self, service, id, address):
        self.service = service
        self.address = address
        self.id = id
        self.__updateId = -1

    def reap(self):
        vs = level0.VertexState(instanceId=self.id, reaped=True)
        self.service.queued_vertex_states.append(vs)
        del self.service.vertexes[self.id]

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
        self.address = "^en/tick-tack-toe/"

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

        self.vertexes = {}
        self.selections = Selections(self)
        self.reset_queue()

        while True:
            message = level0_capnp.Message.from_bytes(socket.recv())
            for vm in message.vertexMessagesFromService:
                self.recv_vertex_message_from_service(vm)
            for v in message.vertexes:
                self.init_vertex()
            for vm in vertexMessagesFromClient:
                self.recv_vertex_message_from_client(vm)
            for vm in message.vertexMessagesFromClient:
                self.recv_vertex_message_from_client(vm)
            for cursor in message.setCursor:
                self.set_cursor(cursor)
            for cm in message.moveCursor:
                self.move_cursor(cm)
            for selection in message.deselect:
                self.selections.deselect(selection)
            self.send_queued()

    def send_queued(self):
        mfs = level0_capnp.Message(
            messages=self.queued_messages,
            vertexes=self.queued_vertexes,
            vertexStates=self.queued_vertex_states,
            level1Messages=self.queued_level1_messages,
        )
        socket.send(mfs.to_bytes())
        self.reset_queue()

    def reset_queue(self):
        self.queued_messages = []
        self.queued_vertexes = []
        self.queued_vertex_states = []
        self.queued_level1_messages = []

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
        if created or cid not in self.cells:
            for pattern, page in self.pages.items():
                match = match_path(path, pattern)
                if match is not None:
                    cell = page(identity, **match[0]).resolve(match[1])
                    self.cells[cid] = cell
                    return cell
        return self.cells[cid]

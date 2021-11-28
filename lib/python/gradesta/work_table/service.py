import zmq
import os
import sys
import pathlib
#import asyncio
#from concurrent.futures import *
from dataclasses import dataclass

from ..ageing_cellar.level0.capnp import *
from .paths import match_path
from .cell_reference_db import CellReferenceDB
from . import simple_topology_expressions
from .locales import Localizer
from .level0.yaml import to_capnp
from . import parse_address

from typing import *


@dataclass
class ProtocolPage:
    actor: "Actor"
    page: "Page"
    path: List[str]


    def load(self, path: str, cid: int, address: level0.Address) -> "ProtocolCell":
        directions = {
            "up": -1,
            "down": 1,
            "left": -2,
            "right": 2,
        }
        cell = self.page.resolve(path)
        pcell = ProtocolCell(page=self, cell=cell, address=address)
        pcell.queue_vertex()
        pcell.queue_vertex_state()
        pcell.queue_data_update()
        for direction, val in directions.items():
            try:
                custom_direction_value = eval("cell." + direction + "()")
            except AttributeError:
                custom_direction_value = None
            if custom_direction_value is not None:
                if "symlink" in custom_direction_value:
                    if type(custom_direction_value["symlink"]) == tuple:
                        path_str, sym_session = custom_direction_value["symlink"]
                        def load_symlink(symlink):
                            sym_path = path_str.split("/")
                            symlink.socket = address.socket
                            symlink.locale = address.locale
                            symlink.serviceName = address.locale
                            vp = symlink.init("vertexPath", len(sym_path))
                            for n in range(0, len(sym_path)):
                                vp[n] = sym_path[n]
                                symlink.session = sym_session
                                # TODO quargs
                        custom_direction_value["symlinkLoader"] = load_symlink
                        del custom_direction_value["symlink"]
                    else:
                        import pdb;pdb.set_trace()

                pcell.queue_port_update(val, **custom_direction_value)

        for (cell1, cell2, dim) in self.page.connections():
            if cell1.path == path:
                def load_connected_address(connected_address):
                    connected_path = self.path + cell2.path.split("/")
                    connected_address.socket = address.socket
                    connected_address.locale = address.locale
                    connected_address.serviceName = address.serviceName
                    vp = connected_address.init("vertexPath", len(connected_path))
                    for n in range(0, len(connected_path)):
                        vp[n] = connected_path[n]
                        # TODO decide what to do with quargs
                    connected_address.session = address.session
                pcell.queue_port_update(dim, vertexLoader=load_connected_address)
        return pcell


@dataclass
class Page:
    session: int
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
    address: level0.Address

    @property
    def cid(self):
        return self.page.actor.crdb.lookup_cell(self.address)[
            0
        ]

    @property
    def path(self):
        return self.page.page.path + self.cell.path

    def queue_data_update(self):
        data = self.cell.draw()
        if type(data) is str:
            data = data.encode("utf8")
        data_mime = self.cell.data_mime()
        if data is not None and data_mime is not None:
            du = self.page.actor.queued_message.forClient.dataUpdates.add()
            du.updateId = next(self.page.actor.updateCounter)
            du.instanceId = self.cid
            du.mime = data_mime
            du.data = data
            return du

    def queue_port_update(
        self,
        direction: int,
        closed: bool = False,
        disconnected: bool = False,
        vertexLoader: Union[bool, Callable[[level0.Address], None]] = False,
        symlinkLoader: Union[bool, Callable[[level0.Address], None]] = False,
    ):
        assert (
            len([x for x in [closed, disconnected, vertexLoader, symlinkLoader] if x is False])
            == 3
        )
        pu = self.page.actor.queued_message.forClient.portUpdates.add()
        pu.updateId = next(self.page.actor.updateCounter)
        pu.instanceId = self.cid
        pu.direction = direction
        if closed:
            pu.connectedVertex.closed = None
        if disconnected:
            pu.connectedVertex.disconnected = None
        if vertexLoader:
            v = pu.connectedVertex.init("vertex")
            vertexLoader(v)
        if symlinkLoader:
            symlink = pu.connectedVertex.init("symlink")
            symlinkLoader(symlink)
        return pu

    def queue_vertex(self):
        v = self.page.actor.queued_message.forClient.vertexes.add()
        v.address = self.address
        v.instanceId = self.cid
        v.view = self.cell.view
        return v

    def queue_vertex_state(self):
        vs = self.page.actor.queued_message.forClient.vertexStates.add()
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


class Actor:
    def __init__(self):
        self.crdb = CellReferenceDB()
        self.cells: DefaultDict[int, ProtocolCell] = {}

        def update_counter():
            uc = 0
            while True:
                uc -= 1
                yield uc

        self.updateCounter: Iterator[int] = update_counter()
        self.vertex_counter = 0
        self.reset_queue()

    def start(self):
        service_dir = os.path.expanduser(f"~/.cache/gradesta/services/{self.service_name}/")
        pathlib.Path(service_dir).mkdir(parents=True, exist_ok=True)

        self.context = zmq.Context()
        self.socket = self.context.socket(zmq.PAIR)
        socket_path = "ipc://{service_dir}/PAIR.zmq".format(
            service_dir=service_dir,
            service_name=self.service_name,
        )
        print("Binding socket {}".format(socket_path))
        self.socket.bind(socket_path)
        while True:
            print("Waiting for a message from the client.")
            message = self.socket.recv()
            print("Message recieved")
            message = level0.Message.from_bytes(message)
            fs = message.forService
            cell_message_types = {
                "vertexMessages": (lambda c: c.recv),
                "portUpdates": (lambda c: c.recv_port_update),
                "dataUpdates": (lambda c: c.recv_data_update),
                "encryptionUpdates": (lambda c: c.recv_encryption_update),
            }
            for address in fs.select:
                self.load_vertex(address)
            for cid in fs.deselect:
                # TODO cancel futures
                del self.cells[cid]
            for cmv, call in cell_message_types.items():
                for update in eval("fs.{cmv}".format(cmv=cmv)):
                    call(self.cells[update.instanceId])(update)
            self.send_queued()

    def send_queued(self):
        gm = self.queued_message.serialize()
        print("Sending queued message.", gm.to_dict())
        self.socket.send(gm.to_bytes())
        self.reset_queue()

    def reset_queue(self):
        self.queued_message = MessageMutable()

    def load_vertex(self, address: level0.Address) -> ProtocolCell:
        cid, created = self.crdb.lookup_cell(address)
        localizer = Localizer(self.service_name)
        if created or cid not in self.cells:
            for pattern, page_cls in self.pages.items():
                print("Matching path ", address, " to pattern ", pattern)
                match = match_path(address.vertexPath, pattern)
                if match is not None:
                    match, path, cellPath = match
                    page = page_cls(address.session, localizer, **match)
                    protocol_page = ProtocolPage(self, page, path)
                    pcell = protocol_page.load(cellPath, cid, address)
                    self.cells[cid] = pcell
                    return pcell
                else:
                    pass # TODO stage 404 vertex state.
        else:
            return self.cells[cid]


    def next_id(self):
        r = self.vertex_counter
        self.vertex_counter += 1
        return r

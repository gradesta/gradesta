#!/usr/bin/python3
import urwid
import zmq
import asyncio
import zmq.asyncio
import argparse
import capnp
from gradesta import level0__capnp
level0 = level0__capnp.level0
from gradesta import level0__yaml
from gradesta import parse_address
from gradesta import vertex as gradesta_vertex
import yaml
import weakref
from dataclasses import dataclass

@dataclass
class Client:
    cid: int = None
    cells = None
    mode: str = "data"


def main():
    parser = argparse.ArgumentParser(
        description="""
    Full screen very basic client for gradesta.
    """
    )
    parser.add_argument("socket", metavar="socket", type=str, help="socket to connect to")
    parser.add_argument("address", metavar="address", type=str, help="address of initial cell")

    args = parser.parse_args()

    context = zmq.asyncio.Context()
    socket = context.socket(zmq.PAIR)
    socket_path = args.socket
    print("Connecting to", socket_path)
    socket.connect(socket_path)
    loop=asyncio.get_event_loop()

    client = Client
    client.cells = {}

    def get_cell(client: Client, address: level0.Address):
        m = level0.Message()
        selects = m.forService.init("select", 1)
        selects[0] = address
        if client.cid:
            deselects = m.forService.init("deselect", 1)
            deselects[0] = client.cid
            del client.cells[client.cid]
        socket.send(m.to_bytes())

    get_cell(client, parse_address.parse_address(args.address))


    def show_or_exit(key):
        if key in ('q', 'Q'):
            raise urwid.ExitMainLoop()
        if key in ("y"):
            mode = "yaml"
            redraw()
        if key in ("d"):
            mode = "data"
            redraw()
        dims = {
            "h": -2,
            "j": 1,
            "k": -1,
            "l": 2
        }
        dim = dims.get(str(key), None)
        cell = client.cells[client.cid]
        if dim:
            if cell:
                try:
                    pu = cell.ports[dim]
                except:
                    return
                if pu.connectedVertex.which() == "vertex":
                    get_cell(client, pu.connectedVertex.vertex)
                elif pu.connectedVertex.which() == "symlink":
                    get_cell(client, pu.connectedVertex.symlink)

    def redraw():
        if client.cid is not None:
            cell = client.cells[client.cid]
            if client.mode == "yaml":
                txt.set_text(str(cell))
            elif client.mode == "data":
                txt.set_text(cell.data.data.decode("utf8"))
            else:
                txt.set_text("Mode set wrong.")
        else:
            txt.set_text("Failed redraw, no cid.")


    txt = urwid.Text("Loading...")
    def build_widgets():
        def update_cell(widget_ref):
            widget = widget_ref()
            if not widget:
                return

            redraw()

            msg_future = socket.recv()

            def process_message(msgf):
                msg = level0.Message.from_bytes(msgf.result())
                def init_cell(cid):
                    if cid in client.cells:
                        return client.cells[cid]
                    vertex = gradesta_vertex.Vertex()
                    client.cells[cid] = vertex
                    return vertex
                fc = msg.forClient
                for v in fc.vertexes:
                    cell = init_cell(v.instanceId)
                    client.cid = v.instanceId
                    cell.vertex = v
                for vs in fc.vertexStates:
                    cell = init_cell(vs.instanceId)
                    cell.vertexState = cell
                for du in fc.dataUpdates:
                    cell = init_cell(du.instanceId)
                    cell.data = du
                for pu in fc.portUpdates:
                    cell = init_cell(pu.instanceId)
                    cell.ports[pu.direction] = pu
                for eu in fc.encryptionUpdates:
                    cell = init_cell(eu.instanceId)
                    cell.encryption = eu
                update_cell(widget_ref)

            msg_future.add_done_callback(process_message)

        update_cell(weakref.ref(txt))
        return urwid.Filler(txt, 'top')

    evl = urwid.AsyncioEventLoop(loop=loop)
    loop = urwid.MainLoop(build_widgets(), unhandled_input=show_or_exit, event_loop=evl)
    loop.run()

#!/usr/bin/python3
import urwid
import zmq
import asyncio
import zmq.asyncio
import argparse
import capnp
import level0_capnp as level0
import level0_yaml
import parse_address
import yaml
import weakref
import gradesta_vertex

parser = argparse.ArgumentParser(
    description="""
Full screen very basic client for gradesta.
"""
)
parser.add_argument("socket", metavar="socket", type=str, help="socket to connect to")

args = parser.parse_args()

context = zmq.asyncio.Context()
socket = context.socket(zmq.PAIR)
socket_path = args.socket
print("Connecting to", socket_path)
socket.connect(socket_path)
loop=asyncio.get_event_loop()

cid = None
cells = {}
mode = "data"

def get_cell(address: level0.Address):
    global cid
    m = level0.Message()
    selects = m.forService.init("select", 1)
    selects[0] = address
    if cid:
        deselects = m.forService.init("deselect", 1)
        deselects[0] = cid
        del cells[cid]
    socket.send(m.to_bytes())

get_cell(parse_address.parse_address("gradesta://example.com/en/tick-tack-toe/   .   .   "))


def show_or_exit(key):
    global mode, cells, cid
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
    cell = cells[cid]
    if dim:
        if cell:
            try:
                pu = cell.ports[dim]
            except:
                return
            if pu.connectedVertex.which() == "vertex":
                get_cell(pu.connectedVertex.vertex)
            elif pu.connectedVertex.which() == "symlink":
                get_cell(pu.connectedVertex.symlink)

def redraw():
    if cid is not None:
        cell = cells[cid]
        if mode == "yaml":
            txt.set_text(str(cell))
        elif mode == "data":
            txt.set_text(cell.data.data.decode("utf8"))


txt = urwid.Text("Loading...")
def build_widgets():
    def update_cell(widget_ref):
        widget = widget_ref()
        if not widget:
            return

        redraw()

        msg_future = socket.recv()

        # Schedule us to update the clock again in one second
        def process_message(msgf):
            global cid
            msg = level0.Message.from_bytes(msgf.result())
            def init_cell(cid):
                global cells
                if cid in cells:
                    return cells[cid]
                vertex = gradesta_vertex.Vertex()
                cells[cid] = vertex
                return vertex
            fc = msg.forClient
            for v in fc.vertexes:
                cell = init_cell(v.instanceId)
                cid = v.instanceId
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

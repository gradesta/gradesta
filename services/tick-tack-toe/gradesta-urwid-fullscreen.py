#!/usr/bin/python3
import urwid
import zmq
import argparse
import capnp
import level0_capnp as level0
import level0_yaml
import parse_address
import yaml

parser = argparse.ArgumentParser(
    description="""
Full screen very basic client for gradesta.
"""
)
parser.add_argument("socket", metavar="socket", type=str, help="socket to connect to")

args = parser.parse_args()

context = zmq.Context()
push_socket = context.socket(zmq.PULL)
pull_socket = context.socket(zmq.PUSH)
push_socket_path = args.socket+".push"
print("Connecting to", push_socket_path)
push_socket.connect(push_socket_path)
pull_socket_path = args.socket+".pull"
print("Connecting to", pull_socket_path)
pull_socket.connect(pull_socket_path)

cell = None
mode = "yaml"

def get_cell(address: level0.Address):
    global cell
    m = level0.Message()
    selects = m.forService.init("select", 1)
    selects[0] = address
    if cell:
        deselects = m.forService.init("deselect", 1)
        deselects[0] = cell.forClient.vertexes[0].instanceId
    pull_socket.send(m.to_bytes())
    cell = level0.Message.from_bytes(push_socket.recv())

get_cell(parse_address.parse_address("gradesta://example.com/en/tick-tack-toe/   .   .   "))


def show_or_exit(key):
    global mode
    if key in ('q', 'Q'):
        raise urwid.ExitMainLoop()
    if key in ("y"):
        mode = "yaml"
    if key in ("d"):
        mode = "data"
    dims = {
        "h": -2,
        "j": 1,
        "k": -1,
        "l": 2
    }
    dim = dims.get(str(key), None)
    if dim:
        if cell:
            for pu in cell.forClient.portUpdates:
                if pu.direction == dim:
                    if pu.connectedVertex.which() == "vertex":
                        get_cell(pu.connectedVertex.vertex)
                    elif pu.connectedVertex.which() == "symlink":
                        get_cell(pu.connectedVertex.symlink)
    if mode == "yaml":
        txt.set_text(yaml.dump(level0_yaml.to_dict(cell)))
    elif mode == "data":
        txt.set_text(cell.forClient.dataUpdates[0].data.decode("utf8"))


txt = urwid.Text(yaml.dump(level0_yaml.to_dict(cell)))
fill = urwid.Filler(txt, 'top')
loop = urwid.MainLoop(fill, unhandled_input=show_or_exit)
loop.run()

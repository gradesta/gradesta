#!/usr/bin/python3
import argparse
import base64
import capnp
import yaml

import level0_capnp as level0


def loadList(loader, field, target, node):
    if field in node:
        lst = [loader(item) for item in node[field]]
        target.init(field, len(lst))
        i = 0
        for item in lst:
            target.__getattribute__(field)[i] = item
            i += 1


def decode_data(yml):
    if "data" in yml:
        return base64.base64decode(yml["data"])
    elif "stringData" in yml:
        return yml["data"].encode("utf8")


def set_attrs(capnp, yml, attrs):
    for attr in attrs:
        capnp.__setattr__(attr, yml[attr])


def load_address_field(yml):
    af = level0.AddressField()
    af.name = yml[0]
    af.value = yml[1]


def load_address(yml, address):
    address.host=yml["host"]
    address.service=yml["service"]
    loadList(load_address_field, "user", address, yml)
    loadList(load_address_field, "internal", address, yml)
    loadList(load_address_field, "credential", address, yml)


def load_vertex_message(yml):
    vm = level0.VertexMessage()
    vm.vertexId = yml["vertexId"]
    vm.data = decode_data(yml)
    return vm


def load_vertex(yml):
    v = level0.Vertex()
    v.address = load_address(yml["address"])
    set_attrs(v, yml, ["instanceId", "view", "clientsideEncryption"])
    return v


def load_vertex_state(yml):
    vs = level0.VertexState()
    set_attrs(vs, yml, ["instanceId", "status", "reaped"])
    return vs


def load_update_status(yml):
    us = level0.UpdateStatus()
    set_attrs(us, yml, ["updateId", "status"])
    us.explanation = load_address()
    return us


def load_port_update(yml):
    pu = level0.PortUpdate()
    set_attrs(pu, yml, ["updateId", "vertexId"])
    if yml["connectedVertex"] == "disconnected":
        pu.connectedVertex.disconnected = None
    elif yml["connectedVertex"] == "closed":
        pu.connectedVertex.closed = None
    elif "symlink" in yml["connectedVertex"]:
        load_address(yml["connectedVertex"]["symlink"], pu.connectedVertex.init("symlink"))
    else:
        pu.connectedVertex.vertex = yml["connectedVertex"]
    return pu


def load_data_update(yml):
    du = level0.DataUpdate()
    set_attrs(du, yml, ["updateId", "vertexId", "mime"])
    du.data = decode_data(yml)
    return du


def load_cursor(yml):
    c = level0.Cursor()
    set_attrs(c, yml, ["cursorId", "vertexId"])


def load_place_cursor(yml):
    cp = level0.CursorPlacement()
    set_attrs(cp, yml, ["cursorId", "selectionId"])
    load_address(yml["address"], cp.address)
    return cp


def load_expand_selection(yml):
    se = level0.SelectionExpansion()
    set_attrs(se, yml, ["selectionId", "vertexId", "direction"])
    return se


def load_for_client(y, fc):
    loadList(load_vertex_message, "vertexMessages", fc, y)
    loadList(load_vertex, "vertexes", fc, y)
    loadList(load_vertex_state, "vertexeStates", fc, y)
    loadList(load_update_status, "updateStatuses", fc, y)
    loadList(load_port_update, "portUpdates", fc, y)
    loadList(load_data_update, "dataUpdates", fc, y)
    loadList(load_cursor, "cursors", fc, y)


def load_deselect(yml):
    return yml


def load_for_service(y, fs):
    loadList(load_vertex_message, "vertexMessages", fs, y)
    loadList(load_port_update, "portUpdates", fs, y)
    loadList(load_data_update, "dataUpdates", fs, y)
    loadList(load_place_cursor, "cursorPlacement", fs, y)
    loadList(load_expand_selection, "selectionExpansion", fs, y)
    loadList(load_deselect, "deselect", fs, y)


def to_capnp(fdi, fdo):
    y = yaml.load(fdi)
    m = level0.Message()
    if "forClient" in y:
        load_for_client(y["forClient"], m.forClient)
    if "forService" in y:
        load_for_service(y["forService"], m.forService)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="""
    Convert gradesta level0 messages from yaml to capnp or compare them.
    """)
    parser.add_argument('command', metavar='command', type=str,
                        help='command [2capnp, 2yaml, compare]')
    parser.add_argument('input', metavar='input', type=str,
                        help='input file')
    parser.add_argument('out', metavar='out', type=str,
                        help='output file')


    args = parser.parse_args()

    with open(args.input, "rb") as fdi:
        with open(args.out, "wb") as fdo:
            if args.command == "2capnp":
                to_capnp(fdi, fdo)
            if args.command == "2yaml":
                to_yaml(fdi, fdo)
            if args.command == "compare":
                compare(fdi, fdo)


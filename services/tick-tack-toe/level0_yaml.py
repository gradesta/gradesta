#!/usr/bin/python3
import argparse
import base64
import capnp
import io
import yaml
import sys

import level0_capnp as level0


def loadList(loader, field, target, node):
    if field in node:
        lst = [loader(item) for item in node[field]]
        target.init(field, len(lst))
        i = 0
        for item in lst:
            target.__getattribute__(field)[i] = item
            i += 1


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
    vm.data = yml["data"]
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
    set_attrs(du, yml, ["updateId", "vertexId", "mime", "data"])
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
    y = yaml.safe_load(fdi)
    m = level0.Message()
    if "forClient" in y:
        load_for_client(y["forClient"], m.forClient)
    if "forService" in y:
        load_for_service(y["forService"], m.forService)
    m.write(fdo)


def to_yaml(fdi, fdo):
    m = level0.Message.read(fdi)
    d = m.to_dict()
    def try_decode_datas(l1, l2):
        try:
            for struct in d[l1][l2]:
                try:
                    struct["data"] = struct["data"].decode("utf8")
                except ValueError:
                    pass
        except KeyError:
            pass
    try_decode_datas("forClient", "dataUpdates")
    try_decode_datas("forClient", "vertexMessages")
    try_decode_datas("forService", "dataUpdates")
    try_decode_datas("forService", "vertexMessages")
    fdo.write(yaml.dump(d, default_flow_style=False).encode("utf8"))


def compare(fd1, fd2):
    from deepdiff import DeepDiff
    t1 = yaml.safe_load(fd1)
    t2 = yaml.safe_load(fd2)
    print(t1)
    print("---")
    print(t2)
    print("---")
    return DeepDiff(t1, t2, verbose_level=0, view='tree')

def ensure_yaml(fd):
    if fd.name.endswith(".yml") or fd.name.endswith(".yaml"):
        return fd
    else:
        fdo = io.BytesIO()
        to_yaml(fd, fdo)
        fdo.seek(0)
        return fdo

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

    fdo_modes = {
        "2capnp": "wb",
        "2yaml": "wb",
        "compare": "rb",
    }

    with open(args.input, "rb") as fdi:
        with open(args.out, fdo_modes[args.command]) as fdo:
            if args.command == "2capnp":
                to_capnp(fdi, fdo)
            if args.command == "2yaml":
                to_yaml(fdi, fdo)
            if args.command == "compare":
                fd1 = ensure_yaml(fdi)
                fd2 = ensure_yaml(fdo)
                differences = compare(fd1, fd2)
                if differences:
                    sys.exit(str(differences))

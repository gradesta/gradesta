#!/usr/bin/env python3
import argparse
import base64
import capnp
import io
import yaml
import sys

import level0_capnp as level0


def loadList(loader, field, target, node):
    if field in node:
        try:
            items = node[field].items()
        except AttributeError:
            items = node[field]
        lst = [loader(item) for item in items]
        target.init(field, len(lst))
        i = 0
        for item in lst:
            target.__getattribute__(field)[i] = item
            i += 1


def set_attrs(capnp, yml, attrs):
    for attr in attrs:
        capnp.__setattr__(attr, yml[attr])


def load_vertex_message(yml):
    vm = level0.VertexMessage()
    vm.vertexId = yml["vertexId"]
    vm.data = yml["data"]
    return vm


def load_address(yml):
    a = level0.Address()
    set_attrs(a, yml, ["address", "identity"])
    return a


def load_vertex(yml):
    v = level0.Vertex()
    set_attrs(v, yml, ["instanceId", "view"])
    v.address = load_address(yml["address"])
    return v


def load_vertex_state(yml):
    vs = level0.VertexState()
    set_attrs(vs, yml, ["instanceId", "status", "reaped"])
    return vs


def load_update_status(yml):
    us = level0.UpdateStatus()
    set_attrs(us, yml, ["updateId", "status"])
    us.explanation = load_address(yml["explanation"])
    return us


def load_port_update(yml):
    pu = level0.PortUpdate()
    set_attrs(pu, yml, ["updateId", "vertexId", "direction"])
    try:
        if yml["connectedVertex"] == "disconnected":
            pu.connectedVertex.disconnected = None
        elif yml["connectedVertex"] == "closed":
            pu.connectedVertex.closed = None
        elif "symlink" in yml["connectedVertex"]:
            pu.connectedVertex.symlink = load_address(yml["connectedVertex"]["symlink"])
    except TypeError:
        pu.connectedVertex.vertex = yml["connectedVertex"]
    return pu


def load_data_update(yml):
    du = level0.DataUpdate()
    set_attrs(du, yml, ["updateId", "vertexId", "mime", "data"])
    return du


def load_encryption_update(yml):
    eu = level0.EncryptionUpdate()
    set_attrs(eu, yml, ["updateId", "vertexId", "keys"])
    return eu


def load_cursor(yml):
    c = level0.Cursor()
    set_attrs(c, yml, ["cursorId", "vertexId"])
    return c


def load_place_cursor(yml):
    cp = level0.CursorPlacement()
    set_attrs(cp, yml, ["cursorId", "selectionId"])
    cp.address = load_address(yml["address"])
    return cp


def load_expand_selection(yml):
    se = level0.SelectionExpansion()
    set_attrs(se, yml, ["selectionId", "vertexId", "direction"])
    return se


def load_for_client(y, fc):
    loadList(load_vertex_message, "vertexMessages", fc, y)
    loadList(load_vertex, "vertexes", fc, y)
    loadList(load_vertex_state, "vertexStates", fc, y)
    loadList(load_update_status, "updateStatuses", fc, y)
    loadList(load_port_update, "portUpdates", fc, y)
    loadList(load_data_update, "dataUpdates", fc, y)
    loadList(load_encryption_update, "encryptionUpdates", fc, y)
    loadList(load_cursor, "cursors", fc, y)


def load_deselect(yml):
    return yml


def load_for_service(y, fs):
    loadList(load_vertex_message, "vertexMessages", fs, y)
    loadList(load_port_update, "portUpdates", fs, y)
    loadList(load_data_update, "dataUpdates", fs, y)
    loadList(load_encryption_update, "encryptionUpdates", fs, y)
    loadList(load_place_cursor, "placeCursor", fs, y)
    loadList(load_expand_selection, "expandSelection", fs, y)
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

    def decode_port_updates(top_level):
        if top_level in d and "portUpdates" in d[top_level]:
            for update in d[top_level]["portUpdates"]:
                if "connectedVertex" in update:
                    try:
                        if "closed" in update["connectedVertex"]:
                            update["connectedVertex"] = "closed"
                    except TypeError:
                        pass
                    try:
                        if "disconnected" in update["connectedVertex"]:
                            update["connectedVertex"] = "disconnected"
                    except TypeError:
                        pass
                    try:
                        if "vertex" in update["connectedVertex"]:
                            update["connectedVertex"] = update["connectedVertex"][
                                "vertex"
                            ]
                    except TypeError:
                        pass

    decode_port_updates("forClient")
    decode_port_updates("forClient")
    decode_port_updates("forClient")
    decode_port_updates("forService")
    fdo.write(yaml.dump(d, default_flow_style=False).encode("utf8"))


def compare(fd1, fd2):
    fd1 = ensure_yaml(fd1)
    fd2 = ensure_yaml(fd2)
    from deepdiff import DeepDiff

    t1 = yaml.safe_load(fd1)
    t2 = yaml.safe_load(fd2)
    print(yaml.dump(t1))
    print("---")
    print(yaml.dump(t2))
    print("---")
    return DeepDiff(t1, t2, verbose_level=0, view="tree")


def ensure_yaml(fd):
    if fd.name.endswith(".yml") or fd.name.endswith(".yaml"):
        return fd
    else:
        fdo = io.BytesIO()
        to_yaml(fd, fdo)
        fdo.seek(0)
        return fdo


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="""
    Convert gradesta level0 messages from yaml to capnp or compare them.
    """
    )
    parser.add_argument(
        "command", metavar="command", type=str, help="command [2capnp, 2yaml, compare]"
    )
    parser.add_argument("input", metavar="input", type=str, help="input file")
    parser.add_argument("out", metavar="out", type=str, help="output file")

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
                differences = compare(fdi, fdo)
                if differences:
                    sys.exit(str(differences))

from typing import *

from ..ageing_cellar.level0 import capnp
level0 = capnp.level0


class InvalidAddress(Exception):
    pass


def parse_address(address: str) -> level0.Address:
    capnp_addr = level0.Address()
    starts = {
        "gradesta://": "/",
        "/": ":",
        "^": None,
    }
    for start, sep in starts.items():
        if address.startswith(start):
            if sep:
                split = address.index(sep, len(start))
            else:
                split = 1
            capnp_addr.socket = address[:split]
            parse_path(address[split+1:], capnp_addr)
            return capnp_addr
    raise InvalidAddress(address)

def parse_path(path: str, capnp_addr: level0.Address):
    path = path.split("/")
    capnp_addr.locale = path[0]
    capnp_addr.serviceName = path[1]
    qargs = None
    if "?" in path[-1]:
        end_parts = path[-1].split("?")
        qargs = end_parts[1].split("&")
        cqas = capnp_addr.init("qargs", len(qargs))
        cqvs = capnp_addr.init("qvals", len(qargs))
        for qa in range(0, len(qargs)):
            k, v = qargs[qa].split("=")
            cqas[qa] = k
            cqvs[qa] = v
        path[-1] = end_parts[0]
    vp = capnp_addr.init("vertexPath", len(path) - 2)
    for n in range(0, len(path) -2):
        vp[n] = path[n+2]

def to_string(address: level0.Address) -> str:
    return from_dict_to_string(address.to_dict())

def from_dict_to_string(address: DefaultDict[str, any]):
    socket = address["socket"]
    string = [socket]
    if socket.startswith("gradesta://"):
        string.append("/")
    if  socket.startswith("/"):
        string.append(":")
    string.append("/".join([address["locale"], address["serviceName"]] + list(address["vertexPath"])))
    if "qargs" in address and len(address["qargs"]) > 0:
        string.append("?")
        for qa, qv in zip(address["qargs"], address["qvals"]):
            string += [qa, "=", qv, "&"]
        string = string[:-1]
    return "".join(string)

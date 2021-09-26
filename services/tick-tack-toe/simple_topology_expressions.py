import numpy as np
from typing import *


def numpy_topology(topology):
    rows = [list(row) for row in topology.split("\n")]
    row_max = max([len(row) for row in rows])
    rows = [row + [" "] * (row_max - len(row)) for row in rows]
    return np.array(rows)


def get_connections(topology, stream):
    topology = numpy_topology(topology)
    connections = set()
    for row in topology:
        connections = connections.union(
            [(a, b, 2) for (a, b) in get_connections_1D(row, stream)]
        )
    for col in np.rot90(topology):
        connections = connections.union(
            [(a, b, 1) for (a, b) in get_connections_1D(col, stream)]
        )
    return connections


def get_connections_1D(top, stream):
    connections = set()
    for tripple in threes_and_twos(top):
        first = None
        if tripple[0].isalpha():
            first = stream.index(tripple[0])
            if tripple[1].isalpha():
                connections.add((first, stream.index(tripple[1])))
        if tripple[1] == "*":
            if first is not None:
                stream_index = first + 1
                while stream[stream_index] == tripple[0]:
                    connections.add((stream_index - 1, stream_index))
                    stream_index += 1
                    if stream_index >= len(stream):
                        break
                if len(tripple) == 3 and tripple[2].isalpha():
                    connections.add((stream_index - 1, stream.index(tripple[2])))
    return connections


def threes_and_twos(l: List):
    it = 0
    while it + 1 < len(l):
        yield l[it : it + 3]
        it += 1

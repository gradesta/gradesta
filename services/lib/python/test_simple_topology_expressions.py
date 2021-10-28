from simple_topology_expressions import *
from pytest import *


@fixture
def stream():
    return ["A", "B", "C", "C", "C", "D"]


@fixture
def top1():
    return """
A
B
C
*
D
"""


def test_np_topology(top1):
    assert numpy_topology(top1)[1][0] == "A"


def test_get_connections1(stream, top1):
    assert get_connections(top1, stream) == set(
        [
            (0, 1, 1),
            (1, 2, 1),
            (2, 3, 1),
            (3, 4, 1),
            (4, 5, 1),
        ]
    )


@fixture
def top2():
    return """
AD
B
C*
"""


def test_get_connections2(stream, top2):
    assert get_connections(top2, stream) == set(
        [
            (0, 5, 2),
            (0, 1, 1),
            (1, 2, 1),
            (2, 3, 2),
            (3, 4, 2),
        ]
    )


def test_threes_and_twos():
    iterator = threes_and_twos([1, 2, 3, 4, 5])
    assert next(iterator) == [1, 2, 3]
    assert next(iterator) == [2, 3, 4]
    assert next(iterator) == [3, 4, 5]
    assert next(iterator) == [4, 5]
    with raises(StopIteration):
        next(iterator)
    iterator = threes_and_twos([1, 2, 3])
    assert next(iterator) == [1, 2, 3]
    assert next(iterator) == [2, 3]
    with raises(StopIteration):
        next(iterator)
    iterator = threes_and_twos([1, 2])
    assert next(iterator) == [1, 2]
    with raises(StopIteration):
        next(iterator)
    iterator = threes_and_twos([1])
    with raises(StopIteration):
        next(iterator)
    iterator = threes_and_twos([])
    with raises(StopIteration):
        next(iterator)

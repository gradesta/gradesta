import pytest
from cell_reference_db import CellReferenceDB


def test_cell_reference_db():
    crdb = CellReferenceDB()
    instance0 = crdb.new_instance("foo", 1)
    assert instance0 == 0
    assert crdb.lookup_cell("foo", 1) == (instance0, False)
    assert crdb.lookup_cell_address(instance0) == ("foo", 1)
    cell1 = crdb.add_reference(instance0, "bar", 1)
    assert cell1 == 1
    instance1 = crdb.new_instance("bar", 1)
    assert cell1 == instance1
    cell2 = crdb.add_reference(instance0, "baz", 1)
    assert cell2 == 2
    cell2_1 = crdb.add_reference(instance1, "baz", 1)
    assert cell2_1 == cell2
    cell3 = crdb.add_reference(instance1, "baf", 1)
    assert cell3 == 3
    assert crdb.lookup_cell_address(cell3) == ("baf", 1)
    crdb.clear_instance(instance1)
    assert crdb.lookup_cell_address(cell3) is None
    cell4 = crdb.add_reference(instance0, "lol", 1)
    assert crdb.lookup_cell_address(cell4) == ("lol", 1)
    crdb.remove_reference(instance0, "lol", 1)
    assert crdb.lookup_cell_address(cell4) is None

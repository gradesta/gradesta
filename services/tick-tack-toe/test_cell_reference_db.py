import pytest
from cell_reference_db import CellReferenceDB


def test_cell_reference_db():
    crdb = CellReferenceDB()
    instance0, created = crdb.lookup_cell("foo", 1)
    assert created == True
    assert instance0 == 0
    assert crdb.lookup_cell("foo", 1) == (instance0, False)
    assert crdb.lookup_cell_path(instance0) == ("foo", 1)
    crdb.clear_cell(instance0)
    assert crdb.lookup_cell("foo", 1) == (1, True)

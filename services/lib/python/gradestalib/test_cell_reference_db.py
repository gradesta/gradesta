import pytest
from cell_reference_db import CellReferenceDB
import parse_address


def test_cell_reference_db():
    crdb = CellReferenceDB()
    instance0, created = crdb.lookup_cell(parse_address.parse_address("gradesta://example.com/en/foo/bar"))
    assert created == True
    assert instance0 == 0
    assert crdb.lookup_cell(parse_address.parse_address("gradesta://example.com/en/foo/bar")) == (instance0, False)
    assert parse_address.to_string(crdb.lookup_cell_path(instance0)) == "gradesta://example.com/en/foo/bar"
    crdb.clear_cell(instance0)
    assert crdb.lookup_cell(parse_address.parse_address("gradesta://example.com/en/foo/bar")) == (1, True)

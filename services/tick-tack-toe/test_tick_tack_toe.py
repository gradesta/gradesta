import pytest
import gradesta_service

from tick_tack_toe import Board

@pytest.fixture
def middle_board():
    return Board(None, 0,
        gradesta_service.Address(dic = {
            "internal": {
            "state": "x o\no x\n x "
        }
    }))

def test_load(middle_board):
    u, f = middle_board.load()
    assert u.dataUpdates[0].data == "x o\no x\n x ".encode("utf8")

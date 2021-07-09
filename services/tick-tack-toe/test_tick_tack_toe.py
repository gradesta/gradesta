import pytest

from tick_tack_toe import Board

@pytest.fixture
def middle_board():
    return Board(None, 0, {
        "internal": {
            "state": "x o\no x\n x "
        }
    })

def test_test(middle_board):
    assert middle_board.data == "x o\no x\n x "


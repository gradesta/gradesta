from dataclasses import dataclass, field
from .level0 import capnp
level0 = capnp.level0

from . import parse_address

from typing import DefaultDict, Set, Optional, Tuple


@dataclass
class CellReferenceDB:
    """
    A database of cell references mapping paths (and identities) to cell ids.
    """

    __path_table: DefaultDict[Tuple[str, int], int] = field(
        default_factory=dict
    )  # [address, cell_id]
    __id_table: DefaultDict[int, Tuple[str, int]] = field(
        default_factory=dict
    )  # [cell_id, address]
    __cell_id_counter: int = -1

    def lookup_cell(self, address: level0.Address) -> Tuple[int, bool]:
        """
        Given an path and session return the cell's id. If no cell id is registered for that path yet, then create one. The second element of the returned tuple is true if a new cell was created.
        """
        address = (parse_address.to_string(address), address.session)
        try:
            return (self.__path_table[address], False)
        except KeyError:
            self.__cell_id_counter += 1
            self.__path_table[address] = self.__cell_id_counter
            self.__id_table[self.__cell_id_counter] = address
            return (self.__cell_id_counter, True)

    def lookup_cell_path(self, cell_id: int) -> Optional[level0.Address]:
        """
        Given a cell_id return it's path and session.
        """
        for ((address, session), loop_cell_id) in self.__path_table.items():
            if cell_id == loop_cell_id:
                address = parse_address.parse_address(address)
                address.session = session
                return address

    def clear_cell(self, cell_id: int):
        address = self.__id_table[cell_id]
        del self.__path_table[address]
        del self.__id_table[cell_id]

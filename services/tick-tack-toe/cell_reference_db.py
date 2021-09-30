from dataclasses import dataclass, field

from typing import DefaultDict, Set, Optional, Tuple


@dataclass
class CellReferenceDB:
    """
    A database of cell references mapping paths (and identities) to cell ids.
    """

    __path_table: DefaultDict[Tuple[str, int], int] = field(
        default_factory=dict
    )  # [(path, identity), cell_id]
    __id_table: DefaultDict[int, Tuple[str, int]] = field(
        default_factory=dict
    )  # [cell_id, [(path, identity)]]
    __cell_id_counter: int = -1

    def lookup_cell(self, cell_path: str, identity: int) -> Tuple[int, bool]:
        """
        Given an path and identity return the cell's id. If no cell id is registered for that path yet, then create one. The second element of the returned tuple is true if a new cell was created.
        """
        try:
            return (self.__path_table[(cell_path, identity)], False)
        except KeyError:
            self.__cell_id_counter += 1
            self.__path_table[(cell_path, identity)] = self.__cell_id_counter
            self.__id_table[self.__cell_id_counter] = (cell_path, identity)
            return (self.__cell_id_counter, True)

    def lookup_cell_path(self, cell_id: int) -> Optional[Tuple[str, int]]:
        """
        Given a cell_id return it's path and identity.
        """
        for ((path, identity), loop_cell_id) in self.__path_table.items():
            if cell_id == loop_cell_id:
                return (path, identity)

    def clear_cell(self, cell_id: int):
        path = self.__id_table[cell_id]
        del self.__path_table[path]
        del self.__id_table[cell_id]

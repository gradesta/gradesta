from dataclasses import dataclass, field

from typing import DefaultDict, Set, Optional, Tuple


@dataclass
class CellReferenceDB:
    """
    A database of cell references mapping addresses (and identities) to cell ids.
    """

    __address_table: DefaultDict[Tuple[str, int], int] = field(
        default_factory=dict
    )  # [(address, identity), cell_id]
    __reference_table: Set[Tuple[int, int]] = field(
        default_factory=set
    )  # referrer: Referenced cell
    __instances: Set[int] = field(default_factory=set)
    __cell_id_counter: int = -1

    def new_instance(self, cell_address: str, identity: int) -> int:
        """
        Registers a new instance with the db. Returns the id of that instance.
        """
        try:
            instance_id = self.__address_table[(cell_address, identity)]
        except KeyError:
            self.__cell_id_counter += 1
            self.__address_table[(cell_address, identity)] = self.__cell_id_counter
            instance_id = self.__cell_id_counter
        self.__instances.add(instance_id)
        return instance_id

    def clear_instance(self, cell_id: int) -> None:
        """
        Removes an instance from the db and clears all references owned by that instance.
        """
        cleared_references = [cell_id]
        # Clear any references made by this cell from the reference table.
        self.__reference_table = set(
            [
                (referrer, referenced)
                for (referrer, referenced) in self.__reference_table
                if referrer != cell_id or (cleared_references.append(referenced))
            ]
        )
        self.__instances.remove(cell_id)
        for cleared_reference in cleared_references:
            # Search for references to this cell and if there are any, return immediately.
            still_referenced_elsewhere = False
            for (refferer, referenced) in self.__reference_table:
                if referenced == cleared_reference:
                    still_referenced_elsewhere = True
            if still_referenced_elsewhere:
                continue
            # There are no references to this cell so it's id can be removed from the address table.
            del self.__address_table[self.lookup_cell_address(cleared_reference)]

    def add_reference_by_id(self, referrer: int, refered: int) -> None:
        self.__reference_table.add((referrer, refered))

    def add_reference(self, referrer: int, cell_address: str, identity: int) -> int:
        refered, _ = self.lookup_cell(cell_address, identity)
        self.add_reference_by_id(referrer, refered)
        return refered

    def remove_reference_by_id(self, referrer, refered) -> bool:
        """
        Returns True if the refered cell is not an instance and that was the last reference to the cell.
        """
        self.__reference_table.remove((referrer, refered))
        if refered in self.__instances:
            return False
        for (_, loop_refered) in self.__reference_table:
            if loop_refered == refered:
                return False
        del self.__address_table[self.lookup_cell_address(refered)]
        return True

    def remove_reference(self, referrer: int, cell_address: str, identity: int) -> int:
        refered, _ = self.lookup_cell(cell_address, identity)
        self.remove_reference_by_id(referrer, refered)
        return refered

    def lookup_cell(self, cell_address: str, identity: int) -> Tuple[int, bool]:
        """
        Given an address and identity return the cell's id. If no cell id is registered for that address yet, then create one. The second element of the returned tuple is true if a new cell was created.
        """
        try:
            return (self.__address_table[(cell_address, identity)], False)
        except KeyError:
            self.__cell_id_counter += 1
            self.__address_table[(cell_address, identity)] = self.__cell_id_counter
            return (self.__cell_id_counter, True)

    def lookup_cell_address(self, cell_id: int) -> Optional[Tuple[str, int]]:
        """
        Given a cell_id return it's address and identity.
        """
        for ((address, identity), loop_cell_id) in self.__address_table.items():
            if cell_id == loop_cell_id:
                return (address, identity)

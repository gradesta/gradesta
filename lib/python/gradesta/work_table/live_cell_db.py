from dataclasses import dataclass, field
from typing import *
from concurrent.futures import Future


@dataclass
class LiveCellDB:
    cells: DefaultDict[int, List[Future]] = field(default_factory=dict)

    def reap(cell: int):
        try:
            for future in self.cells[cell]:
                future.cancel()
                del self.cells[cell]
        except KeyError:
            pass

    def register(cell: int, future: Future):
        if cell not in self.cells:
            self.cells[cell] = []
        self.cells[cell].append(future)

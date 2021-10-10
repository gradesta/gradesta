import os

from typing import Tuple, DefaultDict, Optional, List


def match_path(path: List[str], pattern: str) -> Optional[Tuple[DefaultDict[str, str], str]]:
    """
    Returns None if paths don't match otherwise returns a tuple with the a dict of path_vars and the remaining path segment.
    """
    patt_segs = pattern.split("/")
    path_vars = {}
    for (path_seg, patts) in zip(path, patt_segs):
        if patts.startswith("<") and patts.endswith(">"):
            path_vars[patts[1:-1]] = path_seg
        elif path_seg != patts:
            import pdb;pdb.set_trace()
            return None
    path = list(path)
    page_path = path[:len(patt_segs)]
    remainder = path[len(patt_segs):]
    return path_vars, page_path, "/".join(remainder)

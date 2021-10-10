import os

from typing import Tuple, DefaultDict, Optional, List


def match_path(path: List[str], pattern: str) -> Optional[Tuple[DefaultDict[str, str], str]]:
    """
    Returns None if paths don't match otherwise returns a tuple with the a dict of path_vars and the remaining path segment.
    """
    patt_segs = os.path.split(pattern)
    path_vars = {}
    for (s, patts) in zip(path, patt_segs):
        if patts.startswith("<") and patts.endswith(">"):
            path_vars[patts[1:-1]] = s
        elif s != patts:
            return None
    remainder = list(path)[len(patt_segs):]
    return path_vars, "/".join(remainder)

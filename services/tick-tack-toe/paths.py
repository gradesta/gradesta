import os

from typing import Tuple, DefaultDict, Optional


def match_path(path: str, pattern: str) -> Optional[Tuple[DefaultDict[str, str], str]]:
    """
    Returns None if paths don't match otherwise returns a tuple with the a dict of path_vars and the remaining path segment.
    """
    segs = os.path.split(path)
    patt_segs = os.path.split(pattern)
    path_vars = {}
    for (s, patts) in zip(segs, patt_segs):
        if patts.startswith("<") and patts.endswith(">"):
            path_vars[patts[1:-1]] = s
        elif s != patts:
            return None
    return path_vars, "/".join(segs[len(patt_segs) :])

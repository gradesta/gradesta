from ..ageing_cellar.level0 import capnp
level0 = capnp.level0
from dataclasses import dataclass, field
from typing import *

@dataclass
class Vertex:
    vertex: Union[level0.Vertex, None] = None
    vertexState: Union[level0.VertexState, None] = None
    data: Union[level0.DataUpdate, None] = None
    encryption: Union[level0.EncryptionUpdate, None] = None
    ports: DefaultDict[int, level0.PortUpdate] = field(default_factory=dict)

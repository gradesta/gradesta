from .capnp_mutable import CapnpMutable
from .load_level0 import level0

def ForServiceMutable():
    """
    A version of the ForService capnp object with mutable lists.
    """
    return CapnpMutable(level0.ForService, primitive_fields = ["deselect"])

### Tests
def test_for_service_mutable():
    fsm = ForServiceMutable()
    vm = fsm.vertexMessages.add()
    vm.instanceId = 1
    vm1 = fsm.vertexMessages.add()
    vm1.instanceId = 2
    fsm.deselect.append(1)
    fsm.deselect.append(2)
    message = fsm.serialize()
    assert len(message.vertexMessages) == 2
    assert list(message.deselect) == [1, 2]



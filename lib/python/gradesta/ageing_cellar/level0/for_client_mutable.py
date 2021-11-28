from .capnp_mutable import CapnpMutable
from .load_level0 import level0

def ForClientMutable():
    """
    A more mutable version of ForClient that allows you to work with lists more easilly.
    """
    return CapnpMutable(level0.ForClient)

### Tests

def test_for_client_mutable():
    fcm = ForClientMutable()
    vm = fcm.vertexMessages.add()
    vm.instanceId = 1
    vm1 = fcm.vertexMessages.add()
    vm1.instanceId = 2
    message = fcm.serialize()
    assert len(message.vertexMessages) == 2

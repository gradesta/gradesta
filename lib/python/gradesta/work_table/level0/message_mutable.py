from .load_level0 import level0
from .for_client_mutable import ForClientMutable
from .for_service_mutable import ForServiceMutable

class MessageMutable:
    """
    A version of level0.Message with mutable lists.
    """
    def __init__(self):
        self.forClient = ForClientMutable()
        self.forService = ForServiceMutable()

    def serialize(self) -> level0.Message:
        m = level0.Message()
        m.forClient = self.forClient.serialize()
        m.forService = self.forService.serialize()
        return m

### Tests
def test_message_mutable():
    mm = MessageMutable()
    vm = mm.forClient.vertexMessages.add()
    vm.instanceId = 1
    vm1 = mm.forClient.vertexMessages.add()
    vm1.instanceId = 2
    mm.forService.deselect.append(1)
    mm.forService.deselect.append(2)
    message = mm.serialize()
    assert len(message.forClient.vertexMessages) == 2
    assert list(message.forService.deselect) == [1, 2]

import pytest
import level0_extras

def test_message_mutable():
    mm = level0_extras.MessageMutable()
    vm = mm.forClient.vertexMessages.add()
    vm.instanceId = 1
    vm1 = mm.forClient.vertexMessages.add()
    vm1.instanceId = 2
    mm.forService.deselect.append(1)
    mm.forService.deselect.append(2)
    message = mm.serialize()
    assert len(message.forClient.vertexMessages) == 2
    assert list(message.forService.deselect) == [1, 2]

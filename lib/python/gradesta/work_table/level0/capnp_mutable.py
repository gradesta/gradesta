import capnp

class CapnpMutable:
    """
    A utility class for creating a mutable version of a capnp object.

    This will allow you to mutate capnp lists of capnp structs much more easilly.

    This module assumes that you have a module where all fields are lists of capnp structs.

    If your object contains lists of primitive fields as well, you'll need to explictily set
    `primitive_fields` to exclude those lists.

    It only effects top level lists.
    """
    def __init__(self, cls, primitive_fields=None):
        r = super().__init__()
        self.obj = cls()
        self.primitive_fields = primitive_fields
        if primitive_fields is None:
            self.primitive_fields = []
        potential_fields = [field for field in dir(self.obj) if field not in ["which", "total_size", "copy", "init", "from_dict"] + self.primitive_fields]
        self.fields = [
            field for field in potential_fields if
            type(self.obj.__getattribute__(field)) == capnp.lib.capnp._DynamicListBuilder
        ]
        for field in self.fields:
            self.__setattr__(field, self.obj.init_resizable_list(field))
        for primitive_field in self.primitive_fields:
            self.__setattr__(primitive_field, [])
        return r

    def serialize(self):
        for field in self.fields:
            self.__getattribute__(field).finish()
        for primitive_field in self.primitive_fields:
            plst = self.__getattribute__(primitive_field)
            plst_dest = self.obj.init(primitive_field, len(plst))
            for i in range(0, len(plst)):
                plst_dest[i] = plst[i]
        return self.obj


### Tests
def test_capnp_mutable():
    from .load_level0 import level0
    fsm = CapnpMutable(level0.ForService, primitive_fields = ["deselect"])
    vm = fsm.vertexMessages.add()
    vm.instanceId = 1
    vm1 = fsm.vertexMessages.add()
    vm1.instanceId = 2
    fsm.deselect.append(1)
    fsm.deselect.append(2)
    message = fsm.serialize()
    assert len(message.vertexMessages) == 2
    assert list(message.deselect) == [1, 2]
    # Cover primitive_fields not initialized codepath
    fsm2 = CapnpMutable(level0.ForClient)



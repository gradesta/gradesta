import capnp
import level0_capnp as level0


class CapnpMutable(dict):
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
            self[field] = self.obj.init_resizable_list(field)
        for primitive_field in self.primitive_fields:
            self[primitive_field] = []
        return r

    def serialize(self):
        for field in self.fields:
            self[field].finish()
        for primitive_field in self.primitive_fields:
            plst = self[primitive_field]
            plst_dest = self.obj.init(primitive_field, len(plst))
            for i in range(0, len(plst)):
                plst_dest[i] = plst[i]
        return self.obj

def ForClientMutable():
    return CapnpMutable(level0.ForClient)

def ForServiceMutable():
    return CapnpMutable(level0.ForService, primitive_fields = ["deselect"])

class MessageMutable:
    def __init__(self):
        self.forClient = ForClientMutable()
        self.forService = ForServiceMutable()

    def serialize(self) -> level0.Message:
        m = level0.Message()
        m.forClient = self.forClient.serialize()
        m.forService = self.forService.serialize()
        return m
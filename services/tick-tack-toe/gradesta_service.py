import zmq
import os
import sys
import pathlib
import capnp
import asyncio

import level0_capnp

level0 = level0_capnp


def to_addr_field(k, v):
    af = level0.AddressField()
    af.name = k
    af.value = v
    return af


def address_fields_to_dict(address_fields):
    d = {}
    for field in address_fields:
        d[field.name] = field.value
    return d


class Address:
    def __init__(self, capnp=None, dic=None):
        if capnp:
            self.capnp = capnp
        if dic:
            c = level0.Address()
            c.host = dic.get("host", "")
            c.service = dic.get("service", "")
            c.user = [to_addr_field(k, v) for k, v in dic.get("user", {}).items()]
            c.internal = [to_addr_field(k, v) for k, v in dic.get("internal", {}).items()]
            c.credential = [to_addr_field(k, v) for k, v in dic.get("credential", {}).items()]
            self.capnp = c

    def __getitem__(self, key):
        if key == "host":
            return self.capnp.host
        if key == "service":
            return self.capnp.service
        if key == "internal":
            return address_fields_to_dict(self.capnp.internal)
        if key == "user":
            return address_fields_to_dict(self.capnp.user)
        if key == "credentials":
            return address_fields_to_dict(self.capnp.credentials)
        raise KeyError(key)



class Vertex:
    def __init__(self, service, id, address):
        self.service = service
        self.address = address
        self.id = id
        self.__updateId = -1

    def reap(self):
        vs = level0.VertexState(instanceId = self.id, reaped=True)
        self.service.queued_vertex_states.append(vs)
        del self.service.vertexes[self.id]

    def load(self):
        updates = level0.ForClient()
        futures = set()
        v = level0.Vertex()
        v.address = self.address.capnp
        v.instanceId = self.id
        v.view = self.view
        v.clientsideEncryption = self.clientside_encryption
        updates.init("vertexes", 1)
        updates.vertexes[0] = v
        vs = level0.VertexState()
        vs.instanceId = self.id
        vs.status = 200
        vs.reaped = False
        updates.init("vertexStates", 1)
        updates.vertexStates[0] = vs
        return updates, futures

    def recv(self, event):
        updates = level0.ForClient()
        futures = set()
        return updates, futures

    @property
    def view(self):
        return ""

    @property
    def clientside_encryption(self):
        return ""

    def data_update(self, mime, data):
        du = level0.DataUpdate()
        du.updateId = self.__updateId
        self.__updateId -= 1
        du.vertexId = self.id
        du.mime = mime
        du.data = data
        return du


class Port:
    def __init__(self, vertex):
        self.vertex = vertex

    def get_vertex(self):
        return self.vertex


class Selections:
    def __init__(self, service):
        self.service = service
        self.selections = {}
        self.vertexes = {}

    def select(self, selection, vertex):
        if selection not in self.selections:
            self.selections[selection] = set()
        self.selections[selection].add(vertex)
        if vertex not in self.vertexes:
            self.vertexes[vertex] = set()
        self.vertexes[vertex].add(selection)

    def deselect(self, selection):
        try:
            for vertex in self.selections[selection]:
                vertex_selections = self.vertexes[vertex]
                vertex_selections.remove(selection)
                if not vertex_selections:
                    self.service.vertexes[vertex].reap()
        except KeyError:
            sys.stderr.write("Received deselect command for non-existant selection {}.\n".format(selection))
        del self.selections[selection]


class Actor:
    def __init__(self):
        self.vertex_counter = 0
        services_dir=os.path.expanduser("~/.cache/gradesta/services/sockets")
        pathlib.Path(services_dir).mkdir(parents=True, exist_ok=True)

        context = zmq.Context()
        socket = context.socket(zmq.REP)
        socket_path = "ipc://{services_dir}/{service_name}".format(
            services_dir=services_dir,
            service_name=self.service_name,
        )
        print("Connecting to socket {}".format(socket_path))
        socket.bind(socket_path)

        self.vertexes = {}
        self.selections = Selections(self)
        self.reset_queue()

        while True:
            message = level0_capnp.Message.from_bytes(socket.recv())
            for vm in message.vertexMessagesFromService:
                self.recv_vertex_message_from_service(vm)
            for v in message.vertexes:
                self.init_vertex()
            for vm in vertexMessagesFromClient:
                self.recv_vertex_message_from_client(vm)
            for vm in message.vertexMessagesFromClient:
                self.recv_vertex_message_from_client(vm)
            for cursor in message.setCursor:
                self.set_cursor(cursor)
            for cm in message.moveCursor:
                self.move_cursor(cm)
            for selection in message.deselect:
                self.selections.deselect(selection)
            self.send_queued()

    def send_queued(self):
        mfs = level0_capnp.Message(
            messages = self.queued_messages,
            vertexes = self.queued_vertexes,
            vertexStates = self.queued_vertex_states,
            level1Messages = self.queued_level1_messages,
        )
        socket.send(mfs.to_bytes())
        self.reset_queue()

    def reset_queue(self):
        self.queued_messages = []
        self.queued_vertexes = []
        self.queued_vertex_states = []
        self.queued_level1_messages = []

    def recv_vertex_message(self, vm):
        try:
            self.vertexes[vm.vertexId].write(vm.data)
        except KeyError:
            sys.stderr.write("Received message for vertex id {} but vertex does not exist.\n".format(vm.vertexId))

    def load_vertex(self, address):
        for vertex in self.vertexes:
            if address == vertex.address:
                return vertex.id
        id = self.next_id()
        self.vertexes[id] = create_vertex(id, address)

    def next_id(self):
        r = self.vertex_counter
        self.vertex_counter += 1
        return r

    def create_vertex(self, id, address):
        return self.vertex_class(self, id, address)

    def set_cursor(self, cursor):
        vid = self.load_vertex(cursor.address)
        self.selections.select(cursor.selectionId, vid)

    def move_cursor(self, cm):
        try:
            vertex = self.vertexes[cm.vertexId]
        except KeyError:
             sys.stderr.write("Received move cursor command for non-existant vertex {}.\n".format(cm.vertexId))
        try:
            self.selections.select(cm.selectionId, vertex.ports[cm.direction].get_vertex().id)
        except KeyError:
            sys.stderr.write("Received move cursor command for vertex {} in non-existant direction {}.\n".format(cm.vertexId, cm.direction))

import zmq
import os
import sys
import pathlib
import capnp

import level0_capnp


class Vertex:
    def __init__(self, service, id, address):
        self.service = service
        self.address = address
        self.id = id

    def reap(self):
        vs = level0_capnp.VertexState(instanceId = self.id, reaped=True)
        self.service.queued_vertex_states.append(vs)
        del self.service.vertexes[self.id]

    def write(self, data):
        sys.stderr.write("Received message for vertex {} but write method for that vertex is not implemented. Message data was {}.".format(self.id, data))

    def __attribute_error__(self, attr):
        sys.stderr.write("Error: No {attr} set for vertex {vid}.".format(attr=attr, vid=self.id))

    @property
    def javascript(self):
        self.__attribute_error__("javascript")

    @property
    def status(self):
        self.__attribute_error__("status")

    @property
    def ports(self):
        self.__attribute_error__("ports")

    @property
    def data(self):
        self.__attribute_error__("data")

    def queue_state(self):
        ports = [port.capnp() for port in self.ports]
        vs = level0_capnp.VertexState(
            ports)
        self.service.queued_vertex_states.append()

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
        return Vertex(self, id, address)

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

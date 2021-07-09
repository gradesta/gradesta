@0xa838a0f012aecc79;

# First I will define cell addressess.

# Cell addresses are dictionaries of fields
# This is a divergence from the POSIX concept of
# a path. This new mechanism of addressing objects
# is based on/inspired by the modern use of query params
# in order to save interactive website state in the
# URL.
struct Address {
  # This is the host where the service is running.
  host             @0 :Text;
  # This is the name of the service which the address points to.
  service          @1 :Text;
  # Unlike query params, I decided to have three types
  # of fields in the Address dictionary:
  #  One type is user-visible/user-friendly
  #    It appears in the gradesta equivalent of the address bar
  userFields       @2 :List(AddressField);
  #  A seccond type is also public, but hidden from view of the user
  #  by default (though of course is user accessible and is not secret)
  #  fields of the second state are also copied when the address is copied
  #  and sent to a different user
  internalFields   @3 :List(AddressField);
  #  The third type is secret and belongs to the user alone. It is not
  #  copied when the user copies the address. It is used to store various forms
  #  of credentials.
  #  In a higher level of the gradesta API I plan to add mechanisms by which a
  #  client application may automatically fill in some credential fields of
  #  addresses, replacing user visible passwords with an internally managed
  #  credential system which is more user friendly ( because users do not need
  #  to remember indiviual passwords ) and more secure ( becuase users do not need
  #  to remember individual passwords )
  credentialFields @4 :List(AddressField);
}

# Field values are stored as Text. This means that adresses can be easilly serialized
# as JSON dictionaries when sharing via other mediums such as email.
struct AddressField {
  name          @0 :Text;
  value         @1 :Text;
}
# Here is an example JSON gradesta address serialization:
# {
#   "service": "gradesta://example.com/my-gradesta-service"
#   "user": {  }
#   "internal": {  }
# }

# Messages are passed between the client and the vertex. These are arbitrary
# data and are not specified at this level of the protocol. This level of the
# protocol only specifies the routing of messages to vertexes.
struct VertexMessage {
  vertexId      @0 :UInt64;
  data          @1 :Data;
}

struct PortUpdate {
  updateId      @0 :UInt64;
  vertexId      @1 :UInt64;
  port          @2 :Port;
}

struct DataUpdate {
  updateId      @0 :UInt64;
  vertexId      @1 :UInt64;
  mime          @2 :Text;
  data          @3 :Data;
}

# One of the main design goals of gradesta is the ability to support arbitrarily
# large graphs. This wouldn't be possible if the service automatically served
# all vertexes at once. Instead, vertexes are selected using cursors.
struct Cursor {
  selectionId   @0 :Int64;
  address       @1 :Address;
}

# Cursors can be used to select regions of a graph by "moving" them around from one
# vertex or another by making steps in directions. Each vertex which is stepped into
# is added to the selection.
struct CursorMovement {
  selectionId   @0 :Int64;
  veretexId     @1 :Int64;
  direction     @2 :Int64;
}

# Vertexes have ports. These ports can either be connected or disconnected.
struct Port {
  # Each port has a direction. Directions are important for walk trees which are
  # defined in a higher level of the API.
  direction     @0 :Int64;
  # Ordinarilly there are only 4 directions -1, 1 (up/down), -2, and 2(left, right)
  # however other directions are allowed by the protocal and may be used, for example
  # by a version control system to link to a previous version of a cell
  # or by a citation system to link to a source. In these cases it is typical to refer
  # direction 3 to a context menu with these choices rather than creating new ports
  # for every single purpose.
  connectedVertex :union {
    disconnected @1 :Void;
    vertex       @2 :UInt64;
    symlink      @3 :Address;
  }
}

struct Vertex {
  address       @0 :Address;
  instanceId    @1 :UInt64;
  # view is an IPFS link to javascript used for viewing and intracting with data
  view          @2 :Text;
}

struct VertexState {
  instanceId    @0 :UInt64;
  ports         @1 :List(Port);
  mime          @2 :Text;
  data          @3 :Data;
  # Status is similart to in HTTP.
  # 200 is OK
  # 404 is not found
  # There is one special response code
  # 222
  # Which referes to 'phantom' cells.
  # Phantom cells work just like normal cells, only they don't exist.
  # There is no way for clients to create cells in gradesta.
  # But a service can provide phantom cells, which can become real when the
  # client interacts with them.
  status        @4 :UInt64;
  reaped        @5 :Bool;
}

struct UpdateStatus {
  updateId    @0 :UInt64;
  # Status is similart to in HTTP.
  # 200 is OK
  status      @1 :UInt64;
  explanation @2 :Address;
}


struct Message {
### From service
  vertexMessagesFromService  @0 :List(VertexMessage);
  vertexes                   @1 :List(Vertex);
  vertexStates               @2 :List(VertexState);
  updateStatuses             @3 :List(UpdateStatus);
### From client
  vertexMessagesFromClient   @4 :List(VertexMessage);
  portUpdates                @5 :List(PortUpdate);
  dataUpdates                @6 :List(DataUpdate);
  placeCursor                @7 :List(Cursor);
  moveCursor                 @8 :List(CursorMovement);
  deselect                   @9 :List(Int64);
}
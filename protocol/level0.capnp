@0xa838a0f012aecc79;

# This is level 0 of the Gradesta protocol

# The purpose of level 0 is to provide the absolute minimal required functionality.

# If you want to add new functionality, add it to level 1.

# While this source file is licensed under the LGPLv3,
# You should consider this document to be read only.
# Changes to level 0 of the protocol are not welcome.
# The reason for this is not animosity towards others,
# but due to a desire to maintain compatibility as widely as possible.

struct Address {
  socket            @0 :Text;
  # socket is  either gradesta://example.com:<port-number>
             # or     gradesta://example.com
             # or     /path/to/unix/socket
             # can also be `^` if the socket is a standard local gradsta manager service
  locale            @1 :Text;
  serviceName       @2 :Text;
  vertexPath        @3 :List(Text);
  qargs             @4 :List(Text); # Names of quargs
  qvals             @5 :List(Text); # Values of quargs
  # qargs and qvals can be zipped together to get the pairs you want.
  session          @6 :UInt64; # The id of the session.
}

struct Time {
  timeTaiSecs    @0 :Int64; # Actual number of seconds since 00:00:00 1.1.1970 None of that leap second nonsense, we're interested in linear time.
  timeTaiNs      @1 :Int32; # Nanosecond part of time
  # Note, it can be quite a problem to find the correct TAI time.
  # These fields are somewhat optional. The protocol functions without them set.
  # Prefer in this order:
  #  1. TAI time.
  #  2. Linear UNIX time (no leapseconds while the program is running)
  #  3. Non-linear UNIX time
}


struct Vertex {
  address              @0 :Address;
  instanceId           @1 :UInt64;
  # view is an IPFS link to javascript used for viewing and intracting with data
  # This is an IPFS directory. It can also contain documentation for the vertex's
  # messaging API. In the future other types of frontends besides javascript may
  # be supported.
  # The entry to the javascript should be found in the path:
  # webview/js/index.js
  view                 @2 :Text;
}

# Messages are passed between the client and the vertex. These are arbitrary
# data and are not specified at this level of the protocol. This level of the
# protocol only specifies the routing of messages to vertexes.
struct VertexMessage {
  instanceId    @0 :UInt64;
  data          @1 :Data;
}

struct DataUpdate {
  # Updates origionating from the service have negative ids
  # updates origionating from the client have positive ids
  updateId      @0 :Int64;
  instanceId    @1 :UInt64;
  mime          @2 :Text;
  data          @3 :Data;
}

struct EncryptionUpdate {
  # Updates origionating from the service have negative ids
  # updates origionating from the client have positive ids
  updateId      @0 :Int64;
  instanceId      @1 :UInt64;
  # Blank string for unencrypted, otherwise a list of GNUPG public keys signed by a trusted key.
  # Each public key is then used to encrypt a shared private key ( specific to this vertex ) which is used to encrypt all message data and vertex data.
  # A vertex's data and messages are considered to be end to end encrypted if the keys Text is signed by a trusted key and the messages and data are correctly encrypted by the secret key.
  keys          @2 :Text;
}

# Vertexes have ports. These ports can either be connected or disconnected.
struct PortUpdate {
  # Updates origionating from the service have negative ids
  # updates origionating from the client have positive ids
  updateId      @0 :Int64;
  # Vertex from wence the port connects
  instanceId      @1 :UInt64;
  # Each port has a direction. Directions are important for walk trees which are
  # defined in a higher level of the API.
  direction     @2 :Int64;
  # Ordinarilly there are only 4 directions -1, 1 (up/down), -2, and 2(left, right)
  # however other directions are allowed by the protocal and may be used, for example
  # by a version control system to link to a previous version of a cell
  # or by a citation system to link to a source. In these cases it is typical to refer
  # direction 3 to a context menu with these choices rather than creating new ports
  # for every single purpose.
  connectedVertex :union {
    disconnected @3 :Void;
    closed       @4 :Void;
    vertex       @5 :Address;
    symlink      @6 :Address;
  }
}

struct VertexState {
  instanceId    @0 :UInt64;
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
  status        @1 :UInt64;
  reaped        @2 :Bool;
}

struct UpdateStatus {
  updateId    @0 :UInt64;
  # Status is similart to in HTTP.
  # 200 is OK
  status      @1 :UInt64;
  explanation @2 :Address; # The address of a vertex providing an explanation
}

struct ForClient {
  vertexMessages    @0 :List(VertexMessage);
  vertexes          @1 :List(Vertex);
  vertexStates      @2 :List(VertexState);
  updateStatuses    @3 :List(UpdateStatus);
  portUpdates       @4 :List(PortUpdate);
  dataUpdates       @5 :List(DataUpdate);
  encryptionUpdates @6 :List(EncryptionUpdate);
  timestamp         @7 :List(Time);
  # timestamp is sent as a list, but it is really just an optional value.
  message_number    @8 :Int64;
  # Must always increase by one or restart from zero
  # If restarted from zero this means that the connection
  # has been reset and the state of the protocol should
  # reset to a clean slate.
}

struct ForService {
  vertexMessages    @0 :List(VertexMessage);
  portUpdates       @1 :List(PortUpdate);
  dataUpdates       @2 :List(DataUpdate);
  encryptionUpdates @3 :List(EncryptionUpdate);
  select            @4 :List(Address);
  deselect          @5 :List(Int64); # Instance ids
  timestamp         @6 :List(Time);
  message_number    @7 :Int64;
  # Each time the for client message number is zero the client
  # should reset its state and send a zero numbered message
  # to let the service know that it has been reset.
}

struct Message {
  forClient        @0 :ForClient;
  forService       @1 :ForService;
  track            @2 :List(Text);
  # A LIFO list of socket forwarding addresses.
  # Does not need to be set by clients/services.
  # Must be cleared by websocket bridges at network boundaries
  # to prevent leakage of info about local network topologies!
}
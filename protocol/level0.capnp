@0xa838a0f012aecc79;

# This is level 0 of the Gradesta protocol
# The purpose of level 0 is to provide the absolute minimal required functionality.
# If you want to add new functionality, add it to level 1.
# You should consider this document to be read only.

struct Address {
  # addresses are strings with no maximum length. They can include utf-8 emoji's.
  # They have five or six segments.
  # There form is:
  # gradesta://<host or path to unix socket>(:)(port)/<locale>/<service name>/<service specific vertex path>?<query>#<state to be passed to view>
  # the anchor after the # is cut off by the client and not actually sent to the service
  # Unlike on the web, guis should default to url DECODING the strings so instead of showing
  # Each segment may contain any valid utf-8 character except newline, `/` and `:`.
  # The service specific address may contain `/`. If it does contain `/` then prefix substrings
  # of the address when using `/` as a separator must also be valid addresses. That means
  # if `gradesta://example.com/en-us/foo/bar/baz/baf` is a valid address:
  # `gradesta://example.com/en-us/foo/bar/baz` and `gradesta://example.com/en-us/foo/bar`
  # must also be valid addresses.
  # Addresses should be urlencoded when copied to the clipboard but should not be urlencoded on the wire.
  # PS: Of course the host/path segment must be a valid hostname or path and hostnames typically don't contain emojis ;)

  # service address contains the first part of the vertex address
  # gradesta://<host or path to unix socket>(:)(port)/<locale>/<service name>/
  serviceAddress              @0 :Text;
  path                        @1 :Path;
}

struct Path {
  # A path to a vertex.
  # The path contains the second half of the the vertex address.
  # From
  # gradesta://<host or path to unix socket>(:)(port)/<locale>/<service name>/<service specific vertex path>?<query>#<state to be passed to view>
  # The path contains
  # <service specific vertex path>?<query>#<state to be passed to view>
  path                 @0 :Text;
  identity             @1 :UInt64; # The id of the identity. Identities are gnupg derived and are specified at level1 of the protocol
}

struct Vertex {
  path                 @0 :Path;
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
  vertexId      @0 :UInt64;
  data          @1 :Data;
}

struct DataUpdate {
  # Updates origionating from the service have negative ids
  # updates origionating from the client have positive ids
  updateId      @0 :Int64;
  vertexId      @1 :UInt64;
  mime          @2 :Text;
  data          @3 :Data;
}

struct EncryptionUpdate {
  # Updates origionating from the service have negative ids
  # updates origionating from the client have positive ids
  updateId      @0 :Int64;
  vertexId      @1 :UInt64;
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
  vertexId      @1 :UInt64;
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
    vertex       @5 :Path;
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
}

struct ForService {
  vertexMessages    @0 :List(VertexMessage);
  portUpdates       @1 :List(PortUpdate);
  dataUpdates       @2 :List(DataUpdate);
  encryptionUpdates @3 :List(EncryptionUpdate);
  select            @4 :List(Path);
  deselect          @5 :List(Int64); # Vertex ids
}

struct Message {
  forClient        @0 :ForClient;
  forService       @1 :ForService;
}

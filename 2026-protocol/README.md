The gradesta protocol is for browsing procedurally generated graphs. It is a graph synchronizing protocol for graphs with labeled edges, where edges represent the cardinal directions.

The 2026 gradesta protocol works over WebSockets and is binary. Each WebSocket message is one protocol message.

Common field definitions
- Action id: 8 bytes.
- Vertex id: 8 bytes.
- Edge id: 8 bytes (0 means unset).
- Edge order: west, east, north, south, up, down.
- UTF-8 strings: raw bytes; for mime-type fields, the UTF-8 string is null-terminated and the label/media bytes continue to the end of the WebSocket message.

Client to server
1. Watch landmark
   - Type: 1 byte = 0b1000 0001
   - Action id: 8 bytes
   - Landmark URI: UTF-8 bytes to end of message
2. Stop watching landmark
   - Type: 1 byte = 0b1000 0010
   - Action id: 8 bytes
   - Landmark URI: UTF-8 bytes to end of message
3. Set edges
   - Type: 1 byte = 0b1000 0011
   - Action id: 8 bytes
   - Vertex id: 8 bytes
   - Edges: 6x8 bytes in order west, east, north, south, up, down
4. Click vertex
   - Type: 1 byte = 0b1000 0100
   - Action id: 8 bytes
   - Vertex id: 8 bytes
5. Set vertex label
   - Type: 1 byte = 0b1000 0101
   - Vertex id: 8 bytes
   - Mime-type: UTF-8 bytes, null-terminated
   - Label/media: bytes to end of message

Server to client
1. Set context (tells the client which landmark the following messages are connected to)
   - Type: 1 byte = 0b0000 0001
   - Action id: 8 bytes
   - Landmark URI: UTF-8 bytes to end of message
2. Set vertex label
   - Type: 1 byte = 0b0000 0101
   - Action id: 8 bytes
   - Vertex id: 8 bytes
   - Mime-type: UTF-8 bytes, null-terminated
   - Label/media: bytes to end of message
3. Set edges
   - Type: 1 byte = 0b0000 0011
   - Action id: 8 bytes
   - Vertex id: 8 bytes
   - Edges: 6x8 bytes in order west, east, north, south, up, down
   - Editability bitmask: 1 byte (see below)
4. Log message to client
   - Type: 1 byte = 0b0000 1111
   - Action id: 8 bytes
   - HTTP status: 1 byte
   - Vertex id: 8 bytes
   - Log message: UTF-8 bytes to end of message

Editability bitmask (server Set edges)
- Bit 0 (LSB): vertex label editable
- Bits 1-6: edge editable in order west, east, north, south, up, down

Notes
- A vertex with mime-type text/gradesta-url should be replaced by the landmark vertex that the contained URI points to, allowing patches to stitch into an ongoing graph.
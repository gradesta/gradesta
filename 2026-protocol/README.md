The gradesta protocol is for browsing procedurally generated graphs. It is a graph synchronizing protocol for graphs with labeled edges, where edges represent the cardinal directions.

The 2026 gradesta protocol works over WebSockets and is binary. Each WebSocket message is one protocol message.

Common field definitions
- Action id: 8 bytes.
- Vertex id: 8 bytes.
- Edge id: 8 bytes.
  - 0 = no edge (unset/cleared)
  - 0xFFFFFFFFFFFFFFFF = unchanged (keep existing value when patching)
  - Any other value = vertex ID of adjacent vertex
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
6. Create vertex
   - Type: 1 byte = 0b1000 0110
   - Action id: 8 bytes
   - From vertex id: 8 bytes (source vertex to connect from, 0 for standalone)
   - Direction: 1 byte (0=west, 1=east, 2=north, 3=south, 4=up, 5=down)
   - Layer: 4 bytes (big-endian uint32)
   - Mime-type: UTF-8 bytes, null-terminated
   - Content: bytes to end of message
7. Delete vertex
   - Type: 1 byte = 0b1000 0111
   - Action id: 8 bytes
   - Vertex id: 8 bytes
8. Identification response
   - Type: 1 byte = 0b1001 0000
   - Action id: 8 bytes (must match the request)
   - Identity URL: UTF-8 bytes, null-terminated (public share URL of identity.pub)
   - Signature: 64 bytes (ECDSA P-256 signature of nonce || timestamp from request)
9. Identification refused
   - Type: 1 byte = 0b1001 0001
   - Action id: 8 bytes (must match the request)
10. Introduce elf
   - Type: 1 byte = 0b1010 0000
   - Action id: 8 bytes
   - Elf URL: UTF-8 bytes, null-terminated (base URL of the elf service)
   - Command: UTF-8 bytes, null-terminated (command name from elf manifest)
   - Cursor landmark: UTF-8 bytes, null-terminated (landmark URI where cursor is)
   - Cursor vertex: 8 bytes (vertex ID of current cursor position)
   - Origin landmark: UTF-8 bytes, null-terminated (landmark URI for region origin)
   - Origin vertex: 8 bytes (vertex ID for region origin)
   - Allowed directions: 1 byte bitmask (bit 0=west, 1=east, 2=north, 3=south, 4=up, 5=down)
   - Max depth: 4 bytes (big-endian int32, -1 for unlimited)
   - Permissions: 1 byte bitmask (bit 0=read, 1=write, 2=create, 3=delete)
   - Param count: 4 bytes (big-endian uint32)
   - For each param:
     - Key: UTF-8 bytes, null-terminated
     - Value length: 4 bytes (big-endian uint32)
     - Value: UTF-8 bytes (not null-terminated, uses length)

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
   - HTTP status: 4 bytes (big-endian uint32)
   - Vertex id: 8 bytes
   - Log message: UTF-8 bytes to end of message
5. Request identification
   - Type: 1 byte = 0b0001 0000
   - Action id: 8 bytes
   - Nonce: 32 bytes (random, for replay protection)
   - Timestamp: 8 bytes (Unix seconds, big-endian)
   - Reason: UTF-8 bytes to end of message (human-readable explanation)
6. Introduction token (response to Introduce elf)
   - Type: 1 byte = 0b0010 0000
   - Action id: 8 bytes (matches the Introduce elf request)
   - Token: UTF-8 bytes, null-terminated (one-time authentication token for elf)
   - Server WebSocket URL: UTF-8 bytes to end of message (URL for elf to connect to)

Editability bitmask (server Set edges)
- Bit 0 (LSB): vertex label editable
- Bits 1-6: edge editable in order west, east, north, south, up, down

Layers
------
Vertices can have multiple layers of content. Each layer is identified by a 32-bit unsigned integer in the Set vertex label message. When no layer is specified, layer 0 is assumed.

Layer conventions:
- Layer 0: Primary content (text, images, audio, video, portals)
- Layer 1: Metadata/annotations (transcripts for audio/video, gradesta-url for labeled navigation portals)
- Layer 2: Full/high-resolution content (full images when layer 0 contains a thumbnail)
- Layer 3: HTTP streaming URL (for large media that should be fetched out-of-band via HTTP)

Special MIME types
------------------
text/gradesta-url
  In layer 0: Auto-navigate portal (browser follows immediately, replacing vertex with landmark content)
  In layer 1: Preload portal (browser preloads but displays the layer 0 label, user navigates manually)
  Content: gradesta:// or application-specific URI (e.g., nextcloud://)

text/x-http-stream-url
  Used in layer 3 for out-of-band HTTP streaming of large files (videos, large images, etc.)
  Format: Two lines separated by newline:
    Line 1: Expected MIME type of the HTTP response (e.g., video/mp4)
    Line 2: HTTP URL with one-time security token
  Example:
    video/mp4
    http://server:8083/stream/abc123def456789

  Security model:
  - URLs include cryptographically random one-time tokens (256-bit)
  - Tokens are single-use: consumed on first request, streaming continues after consumption
  - Tokens expire after 1 hour if unused
  - Tokens are tied to the WebSocket session that issued them
  - All tokens for a session are revoked when the WebSocket disconnects

Vertex deletion
---------------
When a client sends "Delete vertex":
1. Server checks if the vertex is editable (edit_mask != 0)
2. Server removes the vertex and its content from storage
3. Server reconnects neighbors to maintain graph connectivity:
   - For each axis (E↔W, N↔S, U↔D), if the deleted vertex had neighbors on both sides, connect them to each other
   - Example: If A→X→B (east chain) and X is deleted, A→B is created
4. Server sends "Set edges" updates to all affected neighbors
5. Server signals deletion to clients by sending "Set edges" for the deleted vertex with:
   - All edges = 0
   - Edit mask = 0
6. Clients should remove the vertex from their local graph when receiving this signal

Notes
- A vertex with mime-type text/gradesta-url should be replaced by the landmark vertex that the contained URI points to, allowing patches to stitch into an ongoing graph.

Identification
Servers can request that clients prove control of a Nextcloud account. This provides decentralized identity without requiring OAuth registration between every server and identity provider.

Flow:
1. Server sends "Request identification" with a nonce, timestamp, and reason
2. Client shows user a consent dialog with the reason
3. If user consents, client signs (nonce || timestamp) with their ECDSA P-256 private key
4. Client sends "Identification response" with their public key URL and signature
5. Server fetches public key from the URL (a Nextcloud public share)
6. Server verifies the signature and checks timestamp is within ±5 minutes
7. Server queries Nextcloud OCS API to resolve share owner username

Identity storage:
- Public key: /.gradesta/identity.pub on user's Nextcloud (shared publicly)
- Private key: /.gradesta/identity.key on user's Nextcloud (encrypted)
- Identity format: username@nextcloud-server.example.com

Elves
-----
Elves are external services that can read and modify graph content. They operate as scoped clients, using the same protocol messages as browsers but with limited permissions.

Summoning flow:
1. Browser sends "Introduce elf" to server with region and permission constraints
2. Server generates a token encoding the allowed region/permissions
3. Server sends "Introduction token" to browser with token and server WebSocket URL
4. Browser POSTs to elf's /summon endpoint:
   {
       "token": "<token from server>",
       "server_ws_url": "wss://server.example/ws",
       "command": "<command name>",
       "params": {<key-value parameters>}
   }
5. Elf connects to server_ws_url via WebSocket, authenticating with the token
6. Server validates token and grants elf access to the scoped region
7. Elf uses standard protocol messages (Set vertex label, Set edges, etc.)
8. Changes propagate to browser through normal protocol messages

Elf manifest format (served at {elf_url}/manifest.json):
{
    "elf_id": "unique-identifier",
    "name": "Human-Readable Name",
    "description": "What this elf does",
    "commands": [
        {
            "name": "command_name",
            "description": "What this command does",
            "inputs": ["cursor", "region", "prompt"]
        }
    ]
}

Input types:
- cursor: Current cursor position (vertex)
- region: A region of the graph defined by origin, directions, and depth
- prompt: A text prompt from the user

Direction bitmask (Introduce elf allowed_directions):
- Bit 0: west
- Bit 1: east
- Bit 2: north
- Bit 3: south
- Bit 4: up
- Bit 5: down

Permission bitmask (Introduce elf permissions):
- Bit 0: read
- Bit 1: write
- Bit 2: create
- Bit 3: delete
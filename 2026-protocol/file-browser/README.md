# Gradesta File Browser

This Go app serves a WebSocket endpoint that exposes a directory listing via the 2026 gradesta protocol.

## Run
```
go run . -port 8080
```

## WebSocket
- Endpoint: `ws://localhost:8080/ws`
- Client should send `Watch landmark` with the directory path as the landmark URI.

## Behavior
- Listing is stacked from north to south.
- Each entry vertex is labeled with the file name.
- East edge points to:
  - `text/gradesta-url` vertex for subfolders.
  - File content vertex for files (mime-type from `file --mime-type -b`).
- A `..` entry is included and points west to a `text/gradesta-url` vertex for the parent directory.

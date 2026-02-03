# Gradesta Hello Server

A simple example server that demonstrates the Gradesta identification protocol.

## What it does

When a client connects, this server:
1. Sends an identification request
2. Verifies the client's cryptographic signature against their Nextcloud-hosted public key
3. Queries the Nextcloud OCS API to resolve the username
4. Sends a greeting: "Hello username@nextcloud-server.example.com!"

If the client refuses identification, it sends "Hello stranger!"

## Running

```bash
go run . -port 8081
```

Or:

```bash
make run
```

## Connecting

Connect with the Gradesta browser:

```
ws://localhost:8081/ws
```

The browser will prompt you to identify with your Nextcloud account.

## Protocol

See the [protocol README](../README.md) for message format details.

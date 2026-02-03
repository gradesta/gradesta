# Gradesta Browser (Rust/Bevy)

A graphical client for the 2026 gradesta protocol.

## Run

```bash
cd gradesta-browser
nix-shell
cargo run
```

## UI

The app opens with a graphical interface:

- **URL bar at top**: Type your WebSocket URI here
- **Status line**: Shows connection state and errors
- **File list**: Shows directory contents when connected

## Keyboard Controls

| Key | Action |
|-----|--------|
| Type | Enter WebSocket URL in the URL bar |
| Enter | Connect to the server |
| Backspace | Delete characters from URL |
| Up/Down | Navigate the file list (after connecting) |

## Example URL

```
ws://localhost:8080/ws?landmark=/home/timothy/
```

Make sure your gradesta file-browser server is running on the specified port!

## Requirements

The `assets/fonts/FiraSans-Regular.ttf` font file is bundled. If missing, run:

```bash
curl -L -o assets/fonts/FiraSans-Regular.ttf \
  "https://github.com/mozilla/Fira/raw/master/ttf/FiraSans-Regular.ttf"
```

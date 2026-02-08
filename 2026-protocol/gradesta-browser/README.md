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

### Navigation
| Key | Action |
|-----|--------|
| Arrow keys / WASD | Navigate the graph (N/S/E/W) |
| PageUp / PageDown | Navigate up/down in stacks |
| Enter | Activate current vertex (follow links) |
| Backspace | Go back in history |

### Editing (Notes)
| Key | Action |
|-----|--------|
| I | Edit current vertex (text input mode) |
| Shift+Arrow | Create new text vertex in that direction |
| Space (hold) | Record audio note in last navigation direction |
| Delete | Delete current vertex (if editable) |
| Escape | Cancel current operation |

### Bag (Clipboard)
| Key | Action |
|-----|--------|
| Y | Yank (copy) current vertex to bag |
| G | Go to top of bag |
| P | Pop from bag |

### Connection
| Key | Action |
|-----|--------|
| Type in URL bar | Enter WebSocket URL |
| Enter (in URL bar) | Connect to the server |

## Deleting Vertices

Pressing the **Delete** key will delete the current vertex if it is editable:

- **Editable vertices** have `edit_mask != 0` (sent in the SetEdges message)
- Read-only vertices (like calendar entries, file browser entries) cannot be deleted
- When a vertex is deleted:
  - The browser navigates to a neighboring vertex (or back in history if none)
  - The server reconnects neighbors to maintain graph connectivity (E↔W, N↔S, U↔D chains are preserved)
  - Content files are removed from storage

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

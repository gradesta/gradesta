# Gradesta Browser Architecture

## Overview

Gradesta Browser is a client for viewing and editing gradesta graphs - 6-directional hypergraphs where vertices can contain multimedia content (text, images, audio, video). It connects to gradesta servers via WebSocket and provides keyboard-driven navigation.

## Technology Stack

| Component | Technology | Purpose |
|-----------|------------|---------|
| Engine | Bevy 0.13 | ECS-based application framework |
| UI | bevy-egui 0.25 | Immediate mode GUI |
| Network | tungstenite | WebSocket client |
| Audio | cpal + rodio | Recording and playback |
| Speech-to-Text | whisper-rs | Local transcription (CPU/GPU) |
| Text-to-Speech | tts | System TTS backends |
| Video | openh264 + symphonia | H.264/AAC decoding |
| Identity | p256 | ECDSA signatures |

## Module Structure

```
src/
├── main.rs              # App entry, UI system, Bevy setup
├── state.rs             # AppState, InputMode, state types
├── graph.rs             # Vertex, GraphState, GridView, layout algorithms
├── network.rs           # WebSocket, ServerEvent, WsCommand, protocol
├── audio.rs             # Recording, playback, OGG encoding
├── media.rs             # MediaCache, textures, waveforms, GIF animation
├── commands/            # Command system
│   └── mod.rs           # Command enum, context-aware dispatch
├── keybindings/         # Keybinding configuration
│   ├── mod.rs           # Module re-exports
│   ├── config.rs        # TOML config loading
│   ├── key.rs           # KeyCode, Modifiers types
│   ├── defaults.rs      # Default keybindings
│   ├── presets.rs       # Vim/Emacs/Normal presets
│   └── resolver.rs      # Context-aware resolution
├── sidebar/             # Sidebar UI components
│   ├── mod.rs           # SidebarMode, SidebarState
│   ├── keybindings.rs   # Keybindings editor UI
│   ├── bag.rs           # Clipboard UI
│   ├── recording.rs     # Audio recording UI
│   └── ...              # Other sidebar panels
├── identity.rs          # ECDSA identity, Nextcloud auth
├── video_player.rs      # MP4/H.264 video player
├── whisper.rs           # Whisper model management
└── tts.rs               # Text-to-speech wrapper
```

## Core Concepts

### Graph Model

Vertices have 6 directional edges: West, East, North, South, Up, Down.
- Horizontal/vertical edges form a 2D grid layout
- Up/Down edges form stacks (multiple items at same position)

```rust
struct LayerContent {
    mime: String,             // Each layer has its own MIME type
    data: Vec<u8>,
}

struct Vertex {
    id: u64,
    label: Vec<u8>,           // Primary content (layer 0 data)
    mime: Option<String>,     // Layer 0 MIME type
    edges: [u64; 6],          // [W, E, N, S, U, D]
    layers: HashMap<u32, LayerContent>,  // Additional layers (each with own MIME)
    edit_mask: u8,            // Editability flags
}
```

### Layer Conventions

Each vertex can have multiple layers, each with its own MIME type:
- **Layer 0**: Primary content (label/thumbnail)
- **Layer 1**: Transcript or portal URL (`text/plain` or `text/gradesta-url`)
- **Layer 2**: Full-resolution image
- **Layer 3**: HTTP stream URL (`text/x-http-stream-url`)

### Landmarks

Landmarks are hierarchical graph contexts (like URLs). Navigation across landmarks triggers new WebSocket subscriptions.

### Bag (Clipboard)

A stack-based clipboard for connecting non-adjacent vertices. Push vertices to bag, navigate elsewhere, pop to create edge.

## State Management

Bevy ECS resources hold application state:

- `AppState` - UI state, current vertex, input mode, panels
- `GraphState` - Vertex/edge data, landmark tracking
- `MediaCache` - Cached textures, waveforms, video players
- `AudioPlaybackState` - Current playback control
- `AudioRecordingSignal` - Recording thread communication

## Command System

Commands are identified by context-specific slugs:
- `global.toggle_bag` - Available everywhere
- `graph.navigate_north` - Graph context only
- `bag.pop` - Bag panel context

Keybindings map physical keys to commands based on current context.

## Data Flow

```
Server (WebSocket)
    ↓ binary messages
NetRx (channel)
    ↓ ServerEvent
ingest_server_events (Bevy system)
    ↓ updates
GraphState / AppState
    ↓ reads
ui_system (Bevy system)
    ↓ renders
egui UI
```

## Network Protocol

Binary WebSocket messages with type byte prefix:

| Type | Name | Direction |
|------|------|-----------|
| 0x01 | SetContext | Server→Client |
| 0x03 | SetEdges | Server→Client |
| 0x05 | SetVertexLabel | Bidirectional |
| 0x0F | Log | Server→Client |
| 0x10 | RequestIdentification | Server→Client |
| 0x81 | WatchLandmark | Client→Server |
| 0x84 | ClickVertex | Client→Server |
| 0x86 | CreateVertex | Client→Server |
| 0x87 | DeleteVertex | Client→Server |
| 0x90 | IdentificationResponse | Client→Server |

## Key Files by Size

| File | Lines | Description |
|------|-------|-------------|
| main.rs | ~6000 | Core app logic, UI system (refactoring in progress) |
| network.rs | ~600 | WebSocket and protocol handling |
| video_player.rs | ~700 | Video decoding |
| keybindings/key.rs | ~650 | Key code definitions |
| graph.rs | ~300 | Graph structures and layout |
| commands/mod.rs | ~510 | Command enum |
| identity.rs | ~500 | Nextcloud auth |
| audio.rs | ~350 | Audio recording and playback |
| media.rs | ~300 | Media loading and caching |
| state.rs | ~200 | Application state types |

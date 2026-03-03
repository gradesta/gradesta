# Gradesta Browser - Agent Instructions

## Build Instructions

The project uses Nix for dependency management. To build:

```bash
cd gradesta-browser
nix-shell --run "cargo build"
```

To run:

```bash
nix-shell --run "cargo run"
```

The `shell.nix` file provides all required dependencies including:
- Rust toolchain (rustc, cargo)
- Audio libraries (alsa-lib, speechd)
- Graphics libraries (wayland, vulkan, X11)
- Whisper ML dependencies (libclang for bindgen)
- Gamepad support (udev)

## Architecture Notes

### Audio System

The audio system uses a persistent audio thread for low-latency playback:

- `AudioPreloadCache`: Stores pre-decoded PCM audio for neighboring cells
- The audio thread owns the `OutputStream` (not Send+Sync) and communicates via channels
- Pre-decoding happens in background threads, results collected in `preload_nearby_audio` system
- Playback state tracked via `AudioPlaybackState.playing_vertex` (shared Arc<Mutex>)

### Key Systems

- `auto_play_audio_on_navigate`: Triggers audio playback when navigating to audio cells
- `preload_nearby_audio`: Pre-decodes audio for 1-2 step neighbors
- `auto_expand_nearby_links`: Pre-fetches landmark data for nearby portals

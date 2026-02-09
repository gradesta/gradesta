# Gradesta Android

Android client for Gradesta graph-based content management.

## Features

- **Swipe Navigation**: Navigate the graph with intuitive swipe gestures
  - Swipe left/right: Move East/West
  - Scroll up/down: Move North/South
- **Content Creation**: Create new vertices with the bottom toolbar
  - Audio recording
  - Photo capture
  - Text input
  - Attachments/links
- **Content Display**: View different content types
  - Text with automatic formatting
  - Images with zoom support
  - Audio with waveform visualization
  - Video playback
- **Breadcrumb Navigation**: Track your path through the graph

## Building

### Requirements

- Android Studio Hedgehog (2023.1.1) or newer
- JDK 17+
- Android SDK 34

### Steps

1. Open the project in Android Studio
2. Sync Gradle dependencies
3. Run on emulator or device

```bash
./gradlew assembleDebug
```

## Architecture

- **Kotlin + Jetpack Compose**: Modern declarative UI
- **Hilt**: Dependency injection
- **OkHttp**: WebSocket communication
- **Coil**: Image loading
- **StateFlow**: Reactive state management

## Project Structure

```
app/src/main/java/com/gradesta/android/
├── data/
│   ├── model/          # Data classes (Vertex, GraphState)
│   ├── network/        # Protocol and WebSocket client
│   └── repository/     # Data layer
├── di/                 # Hilt modules
├── ui/
│   ├── components/     # Reusable UI components
│   ├── navigation/     # Navigation graph
│   ├── screens/        # Screen composables
│   └── theme/          # Material theming
└── viewmodel/          # ViewModels
```

## Protocol

The app communicates with Gradesta servers using a binary WebSocket protocol:

### Server Messages
- `0x01`: SetContext - Set current viewing context
- `0x03`: SetEdges - Update vertex connections
- `0x05`: SetVertexLabel - Update vertex content
- `0x0F`: Log - Server log/status messages
- `0x10`: RequestIdentification - Auth request

### Client Messages
- `0x81`: WatchLandmark - Subscribe to a landmark
- `0x84`: ClickVertex - Activate a vertex
- `0x85`: SetVertexLabel - Update content
- `0x86`: CreateVertex - Create new vertex
- `0x87`: DeleteVertex - Delete vertex

## License

See the main Gradesta repository for license information.

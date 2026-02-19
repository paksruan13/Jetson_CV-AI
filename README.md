# Privacy Preserving Edge Speech Intelligence for Wearables (AR/XR)

This project explores a personal, user-owned edge intelligence layer that serves as the cognitive core for wearable AR devices. The system enables perception, learning, and personalization to occur locally, without reliance on cloud-based AI services or centralized data collection.

## System Architecture

The system follows a hybrid edge–wearable architecture:

- **Wearable / XR Frontend (Quest 3 prototype)**  
  Handles audio capture, basic preprocessing, and user interaction. The device remains lightweight and inference-focused.

- **Edge Compute Node (Jetson Orin Nano)**  
  Performs voice inference, computer vision, and local memory storage under resource constraints (8GB unified RAM).

- **Data Plane (WebRTC)**  
  Real-time audio/video streaming between wearable and edge node. Sub-100ms latency for media data with built-in NAT traversal and adaptive bitrate.

- **Control Plane (GraphQL)**  
  Configuration, commands, and state management. Handles non-time-critical operations via queries, mutations, and subscriptions.

This separation allows compute-intensive and privacy-sensitive workloads to run off-device while maintaining low end-to-end latency.

## Features & Components

### Real-Time Streaming Pipeline
- **Reference File:** `gateway/src/main.rs`
- **Pattern:** Tokio broadcast channels (`tokio::sync::broadcast`)
- **Flow:** AR frames → Broadcast → GraphQL Subscription → WebSocket → Wearable

### GraphQL Gateway (Control Plane)
- **Reference File:** `gateway/src/schema.rs`
- **Stack:** `async-graphql`, `axum`, `tower-http`
- **Key Methods:**
  - `ar_stream_query()` - Fetch latest frame
  - `ar_stream_subscription()` - Real-time 30 FPS stream (currently, will optimize)
- **Purpose:** Device registration, configuration, metrics queries

### WebRTC Data Plane
- **Status:** Planned 
- **Purpose:** Replace WebSocket for media streaming (audio/video)
- **Benefits:** Lower latency (<60ms target), built-in jitter buffering, congestion control
- **Integration:** Unity WebRTC plugin for Quest 3, aiortc/pion for Jetson

### Voice Intelligence Pipeline
- **Reference File:** `ml_services/voice_brain/voice_brain.py`
- **Stack:** `faster-whisper`, `webrtcvad`, `CTranslate2` (INT8)
- **Key Components:**
  - `listen_with_vad()` - WebRTC VAD speech detection with false positive filtering
  - `transcribe()` - GPU-accelerated Whisper inference (0.220s avg)
  - `contains_wakeword()` - Keyword detection ("MERLIN", "hey MERLIN")
  - `extract_command()` - Parse command after wake word
- **Performance:** 0.220s-0.405s transcription, 142MB memory footprint
- **Flow:** Mic → WebRTC VAD → Whisper (GPU) → Wake Word Check → Command Extraction

### Audio Processing (Rust)
- **Reference File:** `p1/src/audio/processor.rs`
- **Concurrency:** `Arc<Mutex<AudioMetrics>>` for thread-safe metrics
- **Features:** RMS/peak calculation, noise gate, WAV recording
- **Future Integration:** Bandpass filtering (300-3400 Hz), spectral analysis, quality gating (SNR > 10dB)

### AR Bridge Protocol
- **Reference Files:** `p1/src/ar/protocol.rs`, `p1/src/ar/bridge.rs`
- **Serialization:** `serde_json` for message encoding
- **Transport:** WebSocket with automatic reconnection
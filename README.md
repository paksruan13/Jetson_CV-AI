# Privacy Perserving Edge Speech Intelligence for Wearables(AR/XR)

This project explores a personal, user-owned edge intelligence layer that serves as the cognitive core for wearable AR devices. The system enables perception, learning, and personalization to occur locally, without reliance on cloud-based AI services or centralized data collection.

## System Architecture

The system follows a hybrid edge–wearable architecture:

- **Wearable / XR Frontend (Quest 3 prototype)**  
  Handles audio capture, basic preprocessing, and user interaction. The device remains lightweight and inference-focused.

- **Edge Compute Node (Jetson Orin Nano)**  
  Performs speech inference, storage of derived features, and periodic model adaptation under local resource constraints.

- **Communication Layer**  
  A custom low-latency WebSocket-based protocol streams compressed audio representations and inference results between the frontend and the edge node.


## Features & Components

### Real-Time streaming Pipeline
- **Reference File:** `gateway/src/main.rs`
- **Pattern:** Tokio broadcast channels (`tokio::sync::broadcast`)
- **Flow:** AR frames -> Broadcast -> GraphQL Subscription -> Websocet -> Wearable

### GraphQL Gatweway
- **Reference File:** `gateway/src/schema.rs`
- **Stack:** `async-graphql`, `axum`, `tower-http`
- **Key Methods**
  - `ar_stream_query()` - Fetch latest frame
  - `ar_strean_subscription()` - Real-time 30 FPS stream (currently, will optimize)

### Audio Processing
- **Reference File:** `p1/src/audio/processor.rs`
- **Concurrency:** `Arc<Mutex<AudioMetrics>>` for thread-safe metrics
- **Features:** RMS/peak calculation, noise gate, WAV recording

### AR Bridge Protocol
- **Reference Files:** `p1/src/ar/protocol.rs`, `p1/src/ar/bridge.rs`
- **Serialization:** `serde_json` for message encoding
- **Transport:** Websocket with automatic reconnection

This separation allows compute-intensive and privacy-sensitive workloads to run off-device while maintaining low end-to-end latency.


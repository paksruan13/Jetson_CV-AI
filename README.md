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

This separation allows compute-intensive and privacy-sensitive workloads to run off-device while maintaining low end-to-end latency.


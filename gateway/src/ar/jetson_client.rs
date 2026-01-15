use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, info, warn};

use crate::schema::ARScene;

/// Jetson AR Bridge Client with auto-reconnection
pub struct JetsonARClient {
    jetson_url : String,
    tx: broadcast::Sender<ARScene>,
}

/// Server messages from Jetson
#[derive(Debug, Clone, Serialize, Deserialize)]
enum ServerMessage {
    Connected {
        server_version: String,
        session_id: String,
    },
    Frame(ARFrame),
    Error {
        message: String,
    },
}

/// AR Frame from p1/src/ar/protocol.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ARFrame {
    pub timestamp: u64,
    pub frame_id: u32,
    #[serde(default)]
    pub protocol_version: Option<u16>,
    #[serde(default)]
    pub objects: Vec<String>,
    #[serde(default)]
    pub hands: Option<String>,
    #[serde(default)]
    pub audio_context: Option<String>,
    #[serde(default)]
    pub device_states: Option<String>,
}

impl JetsonARClient {
    /// New AR client function
    pub fn new(jetson_url: String, tx: broadcast::Sender<ARScene>) -> Self {
        Self { jetson_url, tx }
    }

    /// Start client with auto-reconnect loop
    pub async fn start(self) {
        info!("Starting Jetson AR Client: {}", self.jetson_url);
        loop {
            match self.connect_and_stream().await {
                Ok(_) => {
                    warn!("Jetson Connection closed, reconnecting....");
                }
                Err(e) => {
                    error!("Jetson Connection error: {}, reconnecting...", e);
                }
            }
            sleep(Duration::from_secs(5)).await;
        }
    }

    async fn connect_and_stream(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Connecting to Jetson AR Bridge at {}", self.jetson_url);
        
        let (ws_stream, _) = connect_async(&self.jetson_url).await?;
        info!("Connected to Jetson AR Bridge");

        let(_write, mut read) = ws_stream.split(); // Split for concurrency

        while let Some(msg) = read.next().await {
            match msg{
                Ok(Message::Text(text)) => {
                    // Parse Server Message
                    if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
                        match server_msg {
                            ServerMessage::Connected { server_version, session_id } => {
                                info!("Connected - Version: {}, Session ID: {}", server_version, session_id);
                            }
                            ServerMessage::Frame(frame) => {
                                self.broadcast_frame(frame);
                            }
                            ServerMessage::Error { message } => {
                                error!("Jetson Error: {}", message);
                            }
                        }
                    }
                    else if let Ok(frame) = serde_json::from_str::<ARFrame>(&text) {
                        self.broadcast_frame(frame);
                    }
                    else {
                        warn!("Failed to parse message: {}", text);
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("Jetson Connection Closed");
                    break;
                }
                Err(e) => {
                    error!("WebSocket Error: {}", e);
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    //Helper to broadcast ARFrame to GraphQL subscriber
    fn broadcast_frame(&self, frame: ARFrame) {
        let scene = ARScene {
            timestamp: frame.timestamp,
            frame_id: frame.frame_id,
        };

        if let Err(_) = self.tx.send(scene) {
            warn!("No active ARScene subscribers to receive frame");
        }
    }
}
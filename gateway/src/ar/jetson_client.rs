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
#[serde(tag = "type")]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ARFrame {
    //For latency tracking and frame sync
    pub timestamp: u64,
    //Sequential frame ID for deteching dropped frames
    pub frame_id: u32,
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
                    match serde_json::from_str::<ServerMessage>(&text) {
                        Ok(ServerMessage::Connected { server_version, session_id }) => {
                            info!("Connected - Version: {}, SessionId: {}", server_version, session_id);
                        }
                        Ok(ServerMessage::Frame(frame)) => {
                            // Convert ARFrame to ARScene for GraphQL
                            let scene = ARScene {
                                timestamp: frame.timestamp,
                                frame_id: frame.frame_id,
                            };

                            // Broadcast to subscribers
                            if let Err(_) = self.tx.send(scene) {
                                warn!("No subscribers for ARScene");
                            }
                        }
                        Ok(ServerMessage::Error { message }) => {
                            error!("Jetson Error: {}", message);
                        }
                        Err(e) => {
                            warn!("Failed to parse message: {}", e);
                        }
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
}
/// Jetson -> Quest Communication layer
/// 
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message, WebSocketStream};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};

use super::protocol::*;

// AR Bridge Server build
// - Accpet Quest connection on port
// - Stream ARFrames at target FPS
// - Handle client messages
// - Monitor connection health
pub struct ARBridgeServer {
    /// Listening Addy
    bind_addr: String,
    /// Connected clients, thread safe
    clients: Arc<Mutex<Vec<ConnectedClient>>>,
    /// Stream configs
    config: StreamConfig,
}

///Individual client connection state
struct ConnectedClient {
    // WebSocket stream
    ws: WebSocketStream<TcpStream>,
    //unique ID
    client_id: String,
    // Client capabilities
    capabilities: ClientCapabilities,
    // last ping timestamp
    last_ping: std::time::Instant,
}

#[derive(Debug, Clone)]
pub struct StreamConfig {
    // Target fps
    pub target_fps: u32,
    pub quality: QualityPreset,
    pub enable_compression: bool,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            target_fps: 30,
            quality: QualityPreset::Medium,
            enable_compression: false,
        }
    }
}

// Now create the bridge server
/// Args:
/// - bind_addr
impl ARBridgeServer {
    pub fn new(bind_addr: impl Into<String>) -> Self {
        Self {
            bind_addr: bind_addr.into(),
            clients: Arc::new(Mutex::new(Vec::new())),
            config: StreamConfig::default(),
        }
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting AR Bridge...");
        println!("Listening on: {}, TargetFPS: {}", self.bind_addr, self.config.target_fps);
        println!();
        let listener = TcpListener::bind(&self.bind_addr).await?;
        println!("Bridge is ready for Quest 3 connections");

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    println!("New Connection from: {}", addr);

                    // Clone Arc for spawned task
                    let clients = Arc::clone(&self.clients);
                    let config = self.config.clone();

                    //Handle client in separate task
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_client(stream, clients, config).await {
                            eprintln!("Client Error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Connection Error: {}", e);
                }
            }
        }
    }

    /// Handle individual connection
    async fn handle_client(
        stream: TcpStream,
        _clients: Arc<Mutex<Vec<ConnectedClient>>>,
        config: StreamConfig,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // TCP to websocket
        let ws = accept_async(stream).await?;
        println!("Websocket handshake completed");
        let (mut write, mut read) = ws.split();
        //Send acknowledgement
        let welcome = ServerMessage::Connected {
            server_version: "1.0.0".to_string(),
            session_id: uuid::Uuid::new_v4().to_string(),
        };
        let welcome_json = serde_json::to_string(&welcome)?;
        write.send(Message::Text(welcome_json.into())).await?;

        // start streaming task
        let write_handle: tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> =
            tokio::spawn(async move {
                ARBridgeServer::stream_frames(write, config).await
            });

        while let Some(msg) = read.next().await {
            match msg? {
                Message::Text(text) => {
                    //parse message
                    if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                        Self::handle_client_message(client_msg).await;
                    }
                }
                Message::Close(_) => {
                    println!("Client disconnected");
                    break;
                }
                _ => {}
            }
        }
        write_handle.abort();
        Ok(())
    }

    /// Stream ARFrames at Target FPS
    async fn stream_frames(
        mut write: futures_util::stream::SplitSink<WebSocketStream<TcpStream>, Message>,
        config: StreamConfig,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Calculate frame interval
        let frame_interval = Duration::from_secs_f32(1.0 / config.target_fps as f32);
        let mut frame_timer = interval(frame_interval);
        let mut frame_id = 0u32;
        println!("Starting stream at {} FPS", config.target_fps);
        loop {
            frame_timer.tick().await;
            // Generate dummy ARFrame
            let frame = ARFrame::new_dummy(Self::get_timestamp_us());
            let mut frame = frame;
            frame.frame_id = frame_id;
            //Serialize to JSON
            let frame_json = serde_json::to_string(&ServerMessage::Frame(frame))?;
            //Send frame
            if write.send(Message::Text(frame_json.into())).await.is_err() {
                println!("Client disconnected during streaming");
                break;
            }
            // Wrapping to avoid integra overflow without panic
            frame_id = frame_id.wrapping_add(1);

            // log progress every second
            if frame_id % config.target_fps == 0 {
                println!("Streaming: {} frames sent", frame_id);
            }
        }
        Ok(())
    }

    /// Handle client messages
    async fn handle_client_message(msg: ClientMessage) {
        match msg {
            ClientMessage::Connect { client_id, protocol_version, capabilities } => {
                println!("Client connected: {}", client_id);
                println!("Protocol Version: {}", protocol_version);
                println!("Device: {}", capabilities.device_name);
            }
            ClientMessage::ConfigureStream { target_fps, quality } => {
                println!(" Stream Config, {}FPS, {:?}", target_fps, quality);
            }
            ClientMessage::Ping { timestamp: _timestamp } => {

            }
            _ => {}
        }
    }

    //μs timestamp
    fn get_timestamp_us() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64
    }
}



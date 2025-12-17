/// Big List of AR Protocol Structs

use serde::{Deserialize, Serialize};
///Protocol for AR data packets from Jetson -> Quest 3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ARFrame {
    //For latency tracking and frame sync
    pub timestamp: u64,
    //Sequential frame ID for deteching dropped frames
    pub frame_id: u32,
    // Protocol version for compat. checking
    pub protocol_version: u16,
    //Detec objects from CV pipeline, empty vector if no objects detected
    pub objects: Vec<DetectedObject>,
    // Hand tracking data from Jetson Pipe
    pub hands: Option<HandTrackingData>,
    // Audio context
    pub audio_context: Option<AudioContext>,

    pub device_states: Option <Vec<DeviceState>>,
}

impl ARFrame {
    /// Min AR Frame for now
    pub fn new_dummy(timestamp: u64) -> Self {
        Self {
            timestamp,
            frame_id: 0,
            protocol_version: 0x0100,
            objects: Vec::new(),
            hands: None,
            audio_context: None,
            device_states: None,
        }
    }

    // Calculate frame size in bytes
    pub fn estimate_size(&self) -> usize {
        // BaseL 16 bytes for: timestamp, frame_id, version, padding
        // - Object: approx 40 bytes each (class, bbox, confidence)
        // - Hnads: approx 200 bytes (21 joints, 3 coords, 4 bytes each)
        16 + (self.objects.len() * 40) + self.hands.as_ref().map_or(0, |_| 200) + self.audio_context.as_ref().map_or(0, |_| 64)
    }
}

// CV Struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedObject {
    pub class: String,
    pub confidence: f32,
    pub bbox: BoundingBox,
    pub position_3d: Option<Vector3>,
    pub tracking_id: Option<u32>,
}

// 2D Bounding Box (Normalized coords)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f32, // Left Edge [0.0, 1.0]
    pub y: f32, // Top Edge [0.0, 1.0]
    pub width: f32, // Box Width [0.0, 1.0]
    pub height: f32,// Box Height [0.0, 1.0]
}

// 3D Vector
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector3{
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }
}

// Hand Tracking Data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandTrackingData {
    pub left_hand: Option<HandPose>,
    pub right_hand: Option<HandPose>,
    pub confidence: f32,
    pub source: TrackingSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandPose {
    //landmarks are like (wrist, thumb, indx...)
    pub landmarks: [Vector3; 21],
    pub confidences: [f32; 21],
    pub gesture: Option<GestureType>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum GestureType {
    OpenPalm,
    ClosedFist,
    Pointing,
    ThumbsUp,
    ThumbsDown,
    Peace,
    Pinch,
}

/// Tracking data source for sensor fusion
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TrackingSource {
    JetsonMediaPipe, // CV pipeline from Jetson
    Quest3Native, // Quest 3 built-in tracking
    Fused, // Sensor fusion res
}


///Audio Context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioContext {
    pub sources: Vec<AudioSource>,
    pub listener_position: Vector3,
    pub level_db: f32,
}

// Spatial Audio Source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSource {
    pub id: String,
    pub position: Vector3,
    pub source_type: AudioSourceType,
    pub volume: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AudioSourceType {
    Speech,
    Music,
    Alert,
    Environmental,
}

/// Home device for later
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceState {
    pub id: String,
    pub device_type: String,
    pub state: String,
    pub position: Vector3,
    pub attributes: serde_json::Value,
}

// Client to Server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    // Init handshake with quest
    Connect {
        client_id: String,
        protocol_version: u16,
        capabilities: ClientCapabilities,
    },
    // request specific data streams
    Subscribe {
        streams: Vec<StreamType>,
    },
    // update stream quality settings
    ConfigureStream {
        target_fps: u32,
        quality: QualityPreset,
    },
    // For sensor fusion
    HandTrackingUpdate {
        hands: HandTrackingData,
    },

    InteractionEvent {
        event_type: InteractionType,
        target: Option<String>,
    },

    Ping {
        timestamp: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCapabilities {
    pub device_name: String,
    pub supports_hand_tracking: bool,
    pub supports_spatial_audio: bool,
    pub max_fps: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum StreamType {
    ObjectDetection,
    HandTracking,
    SpatialAudio,
    SmartHome,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum QualityPreset {
    Low,
    Medium,
    High,
    Adaptive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InteractionType {
    Tap,
    Pinch, 
    Grab,
    VoiceCommand { command: String },
}

/// Jetson to quest response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    // Connection Acknowledgement
    Connected {
        server_version: String,
        session_id: String,
    },
    Frame(ARFrame),
    Error {
        code: u16,
        message: String,
    },

    Pong {
        client_timestamp: u64,
        server_timestamp: u64,
    },
}

/// Performace Metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameMetrics {
    pub processing_time_us: u64,
    pub send_time_us: u64,
    pub frame_size: usize,
    pub dropped_frames: u32,
    pub actual_fps: f32,
}
use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{mpsc, Mutex};
use tracing::{info, warn};
use webrtc::{
    api::{
        interceptor_registry::register_default_interceptors,
        media_engine::MediaEngine,
        setting_engine::SettingEngine,
        APIBuilder,
    },
    data_channel::{data_channel_init::RTCDataChannelInit, RTCDataChannel},
    ice_transport::{ice_connection_state::RTCIceConnectionState, ice_server::RTCIceServer},
    media::Sample,
    peer_connection::{
        configuration::RTCConfiguration,
        peer_connection_state::RTCPeerConnectionState,
        sdp::session_description::RTCSessionDescription,
        RTCPeerConnection,
    },
    rtp_transceiver::rtp_codec::{RTCRtpCodecCapability, RTPCodecType},
    track::track_local::{
        track_local_static_sample::TrackLocalStaticSample, TrackLocal,
    },
};

// ============================================================================
// Input Event Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum InputEvent {
    Mouse {
        x: u16,
        y: u16,
        button: Option<MouseButton>,
        action: MouseAction,
    },
    Keyboard {
        code: String,
        action: KeyAction,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MouseButton {
    Left,
    Middle,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MouseAction {
    Move,
    Down,
    Up,
    Click,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyAction {
    Press,
    Release,
}

// ============================================================================
// Input Validation
// ============================================================================

const MAX_WIDTH: u16 = 1920;
const MAX_HEIGHT: u16 = 1080;

const ALLOWED_KEYS: &[&str] = &[
    // Letters
    "KeyA", "KeyB", "KeyC", "KeyD", "KeyE", "KeyF", "KeyG", "KeyH", "KeyI", "KeyJ",
    "KeyK", "KeyL", "KeyM", "KeyN", "KeyO", "KeyP", "KeyQ", "KeyR", "KeyS", "KeyT",
    "KeyU", "KeyV", "KeyW", "KeyX", "KeyY", "KeyZ",
    // Numbers
    "Digit0", "Digit1", "Digit2", "Digit3", "Digit4",
    "Digit5", "Digit6", "Digit7", "Digit8", "Digit9",
    // Arrows
    "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight",
    // Common keys
    "Space", "Enter", "Backspace", "Tab", "Escape",
    "ShiftLeft", "ShiftRight", "ControlLeft", "ControlRight",
    "AltLeft", "AltRight",
    // Blocked: F1-F12, Meta (Windows/Cmd), system combos
];

fn validate_mouse_event(event: &InputEvent) -> Result<(), String> {
    match event {
        InputEvent::Mouse { x, y, .. } => {
            if *x >= MAX_WIDTH || *y >= MAX_HEIGHT {
                return Err(format!(
                    "Coordinates out of bounds: ({}, {}) max=({}, {})",
                    x, y, MAX_WIDTH, MAX_HEIGHT
                ));
            }
            Ok(())
        }
        _ => Err("Not a mouse event".to_string()),
    }
}

fn validate_keyboard_event(event: &InputEvent) -> Result<(), String> {
    match event {
        InputEvent::Keyboard { code, .. } => {
            if !ALLOWED_KEYS.contains(&code.as_str()) {
                return Err(format!("Disallowed key: {}", code));
            }
            Ok(())
        }
        _ => Err("Not a keyboard event".to_string()),
    }
}

// ============================================================================
// Rate Limiting
// ============================================================================

struct RateLimiter {
    events: VecDeque<Instant>,
    max_events: usize,
    window: Duration,
}

impl RateLimiter {
    fn new(max_events: usize, window: Duration) -> Self {
        Self {
            events: VecDeque::new(),
            max_events,
            window,
        }
    }

    fn check(&mut self) -> Result<(), String> {
        let now = Instant::now();

        // Remove events outside the window
        while let Some(front) = self.events.front() {
            if now.duration_since(*front) >= self.window {
                self.events.pop_front();
            } else {
                break;
            }
        }

        if self.events.len() >= self.max_events {
            return Err(format!(
                "Rate limit exceeded: {} events in {:?}",
                self.events.len(),
                self.window
            ));
        }

        self.events.push_back(now);
        Ok(())
    }
}

// ============================================================================
// WebRTC State
// ============================================================================

struct WebRtcState {
    peer_connection: Option<Arc<RTCPeerConnection>>,
    data_channel: Option<Arc<RTCDataChannel>>,
    rate_limiter: RateLimiter,
}

impl WebRtcState {
    fn new() -> Self {
        Self {
            peer_connection: None,
            data_channel: None,
            rate_limiter: RateLimiter::new(100, Duration::from_secs(1)), // 100 events/sec
        }
    }
}

type SharedState = Arc<Mutex<WebRtcState>>;

// ============================================================================
// Video Source (Test Pattern)
// ============================================================================

async fn generate_test_video(track: Arc<TrackLocalStaticSample>) {
    info!("Starting test video generator");

    // Generate a simple test pattern (black frame with incrementing counter)
    let width = 1920u32;
    let height = 1080u32;
    let frame_size = (width * height * 3 / 2) as usize; // YUV420 format
    
    let mut frame_counter = 0u32;

    loop {
        // Create black YUV420 frame
        let mut yuv_data = vec![0u8; frame_size];
        
        // Y plane (luminance) - add some pattern based on counter
        for i in 0..(width * height) as usize {
            yuv_data[i] = ((frame_counter / 10) % 256) as u8;
        }

        let sample = Sample {
            data: yuv_data.into(),
            duration: Duration::from_millis(33), // ~30 FPS
            ..Default::default()
        };

        if let Err(e) = track.write_sample(&sample).await {
            warn!("Error writing video sample: {}", e);
            break;
        }

        frame_counter += 1;
        tokio::time::sleep(Duration::from_millis(33)).await; // 30 FPS
    }

    info!("Test video generator stopped");
}

// ============================================================================
// WebRTC Setup
// ============================================================================

async fn create_peer_connection() -> Result<Arc<RTCPeerConnection>> {
    let mut media_engine = MediaEngine::default();

    // Register VP8 codec for video
    media_engine.register_codec(
        RTCRtpCodecCapability {
            mime_type: "video/VP8".to_owned(),
            clock_rate: 90000,
            channels: 0,
            sdp_fmtp_line: "".to_owned(),
            rtcp_feedback: vec![],
        },
        RTPCodecType::Video,
    )?;

    let mut registry = webrtc::api::interceptor_registry::Registry::new();
    registry = register_default_interceptors(registry, &mut media_engine)?;

    let mut setting_engine = SettingEngine::default();
    setting_engine.set_ice_timeouts(
        Some(Duration::from_secs(5)),
        Some(Duration::from_secs(10)),
        Some(Duration::from_millis(200)),
    );

    let api = APIBuilder::new()
        .with_media_engine(media_engine)
        .with_interceptor_registry(registry)
        .with_setting_engine(setting_engine)
        .build();

    let config = RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        }],
        ..Default::default()
    };

    let peer_connection = Arc::new(api.new_peer_connection(config).await?);
    
    info!("Created peer connection");
    Ok(peer_connection)
}

// ============================================================================
// HTTP Handlers
// ============================================================================

async fn index_handler() -> Html<&'static str> {
    Html(include_str!("../client.html"))
}

#[derive(Debug, Deserialize)]
struct SdpOffer {
    sdp: String,
    #[serde(rename = "type")]
    sdp_type: String,
}

#[derive(Debug, Serialize)]
struct SdpAnswer {
    sdp: String,
    #[serde(rename = "type")]
    sdp_type: String,
}

async fn offer_handler(
    State(state): State<SharedState>,
    Json(payload): Json<SdpOffer>,
) -> Result<Json<SdpAnswer>, (StatusCode, String)> {
    info!("Received SDP offer from client");

    let peer_connection = create_peer_connection()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Create video track
    let video_track = Arc::new(TrackLocalStaticSample::new(
        RTCRtpCodecCapability {
            mime_type: "video/VP8".to_owned(),
            clock_rate: 90000,
            channels: 0,
            sdp_fmtp_line: "".to_owned(),
            rtcp_feedback: vec![],
        },
        "video".to_owned(),
        "webrtc-poc".to_owned(),
    ));

    peer_connection
        .add_track(Arc::clone(&video_track) as Arc<dyn TrackLocal + Send + Sync>)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    info!("Added video track");

    // Start test video generator
    tokio::spawn(generate_test_video(video_track));

    // Create data channel for input events
    let data_channel = peer_connection
        .create_data_channel(
            "input",
            Some(RTCDataChannelInit {
                ordered: Some(true),
                ..Default::default()
            }),
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    info!("Created data channel: {}", data_channel.label());

    // Set up data channel handlers
    let dc = Arc::clone(&data_channel);
    let state_clone = Arc::clone(&state);
    
    data_channel.on_open(Box::new(move || {
        info!("Data channel opened: {}", dc.label());
        Box::pin(async {})
    }));

    data_channel.on_message(Box::new(move |msg| {
        let state = Arc::clone(&state_clone);
        Box::pin(async move {
            let data = String::from_utf8_lossy(&msg.data);
            
            match serde_json::from_str::<InputEvent>(&data) {
                Ok(event) => {
                    let mut state_guard = state.lock().await;
                    
                    // Check rate limit
                    if let Err(e) = state_guard.rate_limiter.check() {
                        warn!("Rate limit check failed: {}", e);
                        return;
                    }

                    // Validate event
                    let validation_result = match &event {
                        InputEvent::Mouse { .. } => validate_mouse_event(&event),
                        InputEvent::Keyboard { .. } => validate_keyboard_event(&event),
                    };

                    match validation_result {
                        Ok(_) => {
                            info!("Received valid input event: {:?}", event);
                            // Here you would inject into X11 session
                            // For POC, just log it
                        }
                        Err(e) => {
                            warn!("Invalid input event: {} - {:?}", e, event);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to parse input event: {} - {}", e, data);
                }
            }
        })
    }));

    // Set up connection state handlers
    let pc = Arc::clone(&peer_connection);
    peer_connection.on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
        info!("Peer connection state changed: {}", s);
        
        if s == RTCPeerConnectionState::Failed || s == RTCPeerConnectionState::Disconnected {
            info!("Peer connection lost, cleaning up...");
        }
        
        Box::pin(async {})
    }));

    peer_connection.on_ice_connection_state_change(Box::new(move |s: RTCIceConnectionState| {
        info!("ICE connection state changed: {}", s);
        Box::pin(async {})
    }));

    // Set remote description (client's offer)
    let offer = RTCSessionDescription::offer(payload.sdp)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    peer_connection
        .set_remote_description(offer)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Create answer
    let answer = peer_connection
        .create_answer(None)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    peer_connection
        .set_local_description(answer.clone())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    info!("Created SDP answer");

    // Store state
    {
        let mut state_guard = state.lock().await;
        state_guard.peer_connection = Some(peer_connection);
        state_guard.data_channel = Some(data_channel);
    }

    Ok(Json(SdpAnswer {
        sdp: answer.sdp,
        sdp_type: "answer".to_string(),
    }))
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("webrtc_poc=info,webrtc=info")
        .init();

    info!("WebRTC POC Server starting...");

    // Shared state
    let state = Arc::new(Mutex::new(WebRtcState::new()));

    // Build router
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/offer", post(offer_handler))
        .with_state(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3030));
    info!("Server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

use crate::infrastructure::driven::sandbox::XvfbManager;
use anyhow::Result;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    // Removed unused import: Response
};
use futures_util::{SinkExt, StreamExt, stream::{SplitSink, SplitStream}};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use tokio::io::AsyncReadExt;
use tokio::process::ChildStdout;
use crate::infrastructure::driven::sandbox::GStreamerManager;
// Removed unused import: BufMut
use webrtc::{
    api::{
        media_engine::MediaEngine,
        APIBuilder,
    },
    ice_transport::{
        ice_credential_type::RTCIceCredentialType,
        ice_server::RTCIceServer,
    },
    peer_connection::{
        configuration::RTCConfiguration,
        peer_connection_state::RTCPeerConnectionState,
        sdp::session_description::RTCSessionDescription,
        RTCPeerConnection,
    },
    rtp_transceiver::rtp_codec::RTCRtpCodecCapability,
    track::track_local::{track_local_static_sample::TrackLocalStaticSample, TrackLocal},
};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use std::collections::HashMap;
use crate::infrastructure::driven::input::x11_input::X11InputManager;
// Removed import for deleted trait

/// Signaling message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum SignalingMessage {
    RequestOffer,
    Offer { sdp: String },
    Answer { sdp: String },
    IceCandidate {
        candidate: String,
        #[serde(rename = "sdpMid")]
        sdp_mid: Option<String>,
        #[serde(rename = "sdpMLineIndex")]
        sdp_mline_index: Option<u16>,
    },
    MouseMove { x: i32, y: i32 },
    MouseDown { button: u8 },
    MouseUp { button: u8 },
    MouseScroll { delta_y: f32 },
    KeyDown { key: String, code: String },
    KeyUp { key: String, code: String },
    Resize { width: u32, height: u32 },
    Error { message: String },
}

/// WebRTC session manager (adapter for WebRTC)
pub struct WebRTCAdapter {
    peers: Arc<RwLock<HashMap<String, Arc<RTCPeerConnection>>>>,
    tracks: Arc<RwLock<HashMap<String, Arc<TrackLocalStaticSample>>>>,
    cancel_tokens: Arc<RwLock<HashMap<String, CancellationToken>>>,
    ffmpeg_handles: Arc<RwLock<HashMap<String, ChildStdout>>>,
    input_manager: Arc<X11InputManager>,
    xvfb_manager: Arc<XvfbManager>,
    // Removed unused field ffmpeg_manager
}

impl WebRTCAdapter {
    pub fn new(xvfb_manager: Arc<XvfbManager>) -> Self {
        Self {
            peers: Arc::new(RwLock::new(HashMap::new())),
            tracks: Arc::new(RwLock::new(HashMap::new())),
            cancel_tokens: Arc::new(RwLock::new(HashMap::new())),
            ffmpeg_handles: Arc::new(RwLock::new(HashMap::new())),
            input_manager: Arc::new(X11InputManager::new()),
            xvfb_manager,
            // ffmpeg_manager removed
        }
    }
    

    async fn create_peer_connection(&self, session_id: &str, ws_sender: Arc<tokio::sync::Mutex<SplitSink<WebSocket, Message>>>, gstreamer: Arc<GStreamerManager>, config: &crate::domain::aggregates::VideoConfig) -> Result<(Arc<RTCPeerConnection>, Arc<TrackLocalStaticSample>)> {
        // Create media engine
        let mut media_engine = MediaEngine::default();
        
        // Register VP8 codec (mandatory WebRTC codec, natively supported in all browsers)
        media_engine.register_codec(
            webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecParameters {
                capability: RTCRtpCodecCapability {
                    mime_type: "video/VP8".to_owned(),
                    clock_rate: 90000,
                    channels: 0,
                    sdp_fmtp_line: "".to_owned(),
                    rtcp_feedback: vec![],
                },
                payload_type: 96,
                ..Default::default()
            },
            webrtc::rtp_transceiver::rtp_codec::RTPCodecType::Video,
        )?;
        
        // Create API with media engine
        let api = APIBuilder::new()
            .with_media_engine(media_engine)
            .build();
        
        // Configure ICE servers with STUN and TURN
        let turn_server = std::env::var("TURN_SERVER").unwrap_or_else(|_| "turn:localhost:3478".to_string());
        let turn_username = std::env::var("TURN_USERNAME").unwrap_or_else(|_| "sandbox".to_string());
        let turn_credential = std::env::var("TURN_CREDENTIAL").unwrap_or_else(|_| "dev_turn_secret".to_string());
        
        let rtc_config = RTCConfiguration {
            ice_servers: vec![
                RTCIceServer {
                    urls: vec!["stun:stun.l.google.com:19302".to_owned()],
                    ..Default::default()
                },
                RTCIceServer {
                    urls: vec![turn_server],
                    username: turn_username,
                    credential: turn_credential,
                    credential_type: RTCIceCredentialType::Password,
                },
            ],
            ..Default::default()
        };

        // Create peer connection
        let peer_connection = Arc::new(api.new_peer_connection(rtc_config).await?);

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
            "webrtc-rs".to_owned(),
        ));

        // Add track to peer connection
        peer_connection
            .add_track(Arc::clone(&video_track) as Arc<dyn TrackLocal + Send + Sync>)
            .await?;

        // Start GStreamer pipeline and stream frames to WebRTC track
        let cancel_token = CancellationToken::new();
        let track_clone = Arc::clone(&video_track);
        let token_clone = cancel_token.clone();
        let session_id_string = session_id.to_string();
        // Retrieve session-specific display string from XvfbManager
        // Use the XvfbManager instance passed to this function
        let display_str = match self.xvfb_manager.get_display_str(session_id).await {
            Some(d) => d,
            None => {
                error!("[session {}] No display found in XvfbManager for session; cannot start GStreamer pipeline", session_id);
                return Err(anyhow::anyhow!("No display found for session {}", session_id));
            }
        };
        // Register X11 input session for this WebRTC session
        self.input_manager.register_session(session_id.to_string(), display_str.clone()).await;
        let width = config.width;
        let height = config.height;
        let framerate = config.framerate;
        let gstreamer = Arc::clone(&gstreamer);

        tokio::spawn(async move {
            match gstreamer.start_vp8_ivf_stream(&session_id_string, &display_str, width, height, framerate) {
                Ok(rx) => {
                    info!("Starting VP8 stream from GStreamer for session: {}", session_id_string);
                    let mut frame_count = 0u64;
                    // Read and skip IVF header (32 bytes)
                        if let Ok(header) = rx.recv() {
                            // header: Vec<u8>
                        if header.len() == 32 {
                            // IVF header received and skipped
                        }
                    }
                        while let Ok(frame_data) = rx.recv() {
                            // frame_data: Vec<u8>
                        if token_clone.is_cancelled() {
                            info!("Video stream cancelled after {} frames", frame_count);
                            break;
                        }
                        if let Err(e) = track_clone.write_sample(&webrtc::media::Sample {
                            data: frame_data.into(),
                            duration: std::time::Duration::from_millis(33), // ~30fps
                            ..Default::default()
                        }).await {
                            warn!("Failed to send VP8 sample: {}", e);
                        } else {
                            frame_count += 1;
                            if frame_count % 30 == 0 {
                                debug!("Streamed {} VP8 frames", frame_count);
                            }
                        }
                    }
                    info!("VP8 stream ended after {} frames", frame_count);
                }
                Err(e) => {
                    error!("Failed to start GStreamer VP8 stream: {}", e);
                }
            }
        });
        
        // Store cancel token
        let mut tokens = self.cancel_tokens.write().await;
        tokens.insert(session_id.to_string(), cancel_token.clone());
        drop(tokens);
        
        // Set up ICE candidate handler to send candidates to client
        let ws_sender_clone = Arc::clone(&ws_sender);
        peer_connection.on_ice_candidate(Box::new(move |candidate: Option<webrtc::ice_transport::ice_candidate::RTCIceCandidate>| {
            let sender = Arc::clone(&ws_sender_clone);
            Box::pin(async move {
                if let Some(candidate) = candidate {
                    match candidate.to_json() {
                        Ok(json_candidate) => {
                            let msg = SignalingMessage::IceCandidate {
                                candidate: json_candidate.candidate,
                                sdp_mid: json_candidate.sdp_mid,
                                sdp_mline_index: json_candidate.sdp_mline_index.map(|v| v as u16),
                            };
                            if let Ok(json) = serde_json::to_string(&msg) {
                                let mut sender_lock = sender.lock().await;
                                let _ = sender_lock.send(Message::Text(json)).await;
                            }
                        }
                        Err(e) => {
                            warn!("Failed to serialize ICE candidate: {}", e);
                        }
                    }
                }
            })
        }));
        
        // Set up connection state handler to stop streaming on failure
        let session_id_clone = session_id.to_string();
        let cancel_token_clone = cancel_token.clone();
        peer_connection.on_peer_connection_state_change(Box::new(move |state: RTCPeerConnectionState| {
            let session = session_id_clone.clone();
            let token = cancel_token_clone.clone();
            Box::pin(async move {
                info!("Peer connection state changed: {}", state);
                match state {
                    RTCPeerConnectionState::Failed | RTCPeerConnectionState::Disconnected | RTCPeerConnectionState::Closed => {
                        warn!("Connection {} failed/disconnected/closed, stopping frame sender", session);
                        token.cancel();
                    }
                    _ => {}
                }
            })
        }));
        
        Ok((peer_connection, video_track))
    }

    async fn handle_request_offer(&self, session_id: &str, ws_sender: Arc<tokio::sync::Mutex<SplitSink<WebSocket, Message>>>, gstreamer: Arc<GStreamerManager>, config: &crate::domain::aggregates::VideoConfig) -> Result<String> {
        info!("Creating WebRTC offer for session: {}", session_id);
        
        // Create peer connection
        let (peer_connection, video_track) = self.create_peer_connection(session_id, ws_sender, gstreamer, config).await?;
        
        // Create offer
        let offer = peer_connection.create_offer(None).await?;
        let offer_sdp = offer.sdp.clone();
        
        // Set local description
        peer_connection.set_local_description(offer).await?;
        
        // Store peer connection and track
        let mut peers = self.peers.write().await;
        let mut tracks = self.tracks.write().await;
        peers.insert(session_id.to_string(), peer_connection);
        tracks.insert(session_id.to_string(), video_track);
        
        info!("WebRTC offer created for session: {}", session_id);
        Ok(offer_sdp)
    }

    async fn handle_answer(&self, session_id: &str, sdp: String) -> Result<()> {
        info!("Received answer from client for session: {}", session_id);
        
        let peers = self.peers.read().await;
        if let Some(pc) = peers.get(session_id) {
            let answer = RTCSessionDescription::answer(sdp)?;
            pc.set_remote_description(answer).await?;
            info!("Set remote description for session: {}", session_id);
        } else {
            return Err(anyhow::anyhow!("Peer connection not found"));
        }
        
        Ok(())
    }

        async fn handle_ice_candidate(&self, session_id: &str, candidate: String, sdp_mid: Option<String>, sdp_mline_index: Option<u16>) -> Result<()> {
            debug!("Received ICE candidate from client for session: {}", session_id);
        
            let peers = self.peers.read().await;
            if let Some(pc) = peers.get(session_id) {
                use webrtc::ice_transport::ice_candidate::{RTCIceCandidateInit};
                let ice_candidate = RTCIceCandidateInit {
                    candidate,
                    sdp_mid,
                    sdp_mline_index,
                    ..Default::default()
                };
                pc.add_ice_candidate(ice_candidate).await?;
                debug!("Added ICE candidate for session: {}", session_id);
            } else {
                return Err(anyhow::anyhow!("Peer connection not found"));
            }
        
            Ok(())
        }
    
    /// Store FFmpeg stdout handle and start streaming when peer connects

    pub async fn cleanup(&self, session_id: &str) -> Result<()> {
        info!("Cleaning up WebRTC resources for session: {}", session_id);

        // Cancel the frame sender task first
        let mut tokens = self.cancel_tokens.write().await;
        if let Some(token) = tokens.remove(session_id) {
            token.cancel();
            info!("Cancelled frame sender for session: {}", session_id);
        }
        drop(tokens); // Release lock before cleanup

        // Remove FFmpeg handle
        let mut handles = self.ffmpeg_handles.write().await;
        handles.remove(session_id);
        drop(handles);

        // Explicitly stop FFmpeg encoder for this session
        // use crate::domain::aggregates::VideoSessionId; // removed unused import
            // FFmpeg stop_session feature disabled (method removed)

        // Stop application (X11 input session)
        self.input_manager.unregister_session(session_id).await;

        let mut peers = self.peers.write().await;
        let mut tracks = self.tracks.write().await;

        // Remove ICE state and candidates by dropping peer connection
        if let Some(pc) = peers.remove(session_id) {
            pc.close().await?;
            info!("Closed peer connection for session: {}", session_id);
        }
        tracks.remove(session_id);

        // Optionally: Remove any other ICE-related state here if tracked separately

        Ok(())
    }
}

/// WebSocket handler for signaling
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    State(adapter): State<Arc<WebRTCAdapter>>,
) -> impl axum::response::IntoResponse {
    let session_id = params.get("session").cloned().unwrap_or_else(|| Uuid::new_v4().to_string());
    ws.on_upgrade(move |socket| handle_socket(socket, adapter, session_id))
}

// Removed orphaned code and unexpected closing delimiter

async fn handle_socket(socket: WebSocket, adapter: Arc<WebRTCAdapter>, session_id: String) {
    let (sender, mut receiver): (SplitSink<WebSocket, Message>, SplitStream<WebSocket>) = socket.split();
    let sender = Arc::new(tokio::sync::Mutex::new(sender));

    // You must have gstreamer and config available here. For demo, create new ones (replace with real context as needed):
    let gstreamer = Arc::new(crate::infrastructure::driven::sandbox::GStreamerManager::new().expect("Failed to init GStreamer"));
    let config = crate::domain::aggregates::VideoConfig::default();

    info!("WebSocket connection established for session: {}", session_id);

    loop {
        match receiver.next().await {
            Some(Ok(msg)) => match msg {
                Message::Text(text) => {
                    debug!("Received message: {}", text);
                    match serde_json::from_str::<SignalingMessage>(&text) {
                        Ok(message) => {
                            let response = handle_signaling_message(
                                message,
                                &session_id,
                                &adapter,
                                Arc::clone(&sender),
                                Arc::clone(&gstreamer),
                                config.clone(),
                            ).await;
                            match response {
                                Ok(Some(msg)) => {
                                    if let Ok(json) = serde_json::to_string(&msg) {
                                        let mut sender_lock = sender.lock().await;
                                        let _ = sender_lock.send(Message::Text(json)).await;
                                    }
                                }
                                Ok(None) => {}
                                Err(e) => {
                                    error!("Error handling signaling message: {}", e);
                                    let error_msg = SignalingMessage::Error {
                                        message: e.to_string(),
                                    };
                                    if let Ok(json) = serde_json::to_string(&error_msg) {
                                        let mut sender_lock = sender.lock().await;
                                        let _ = sender_lock.send(Message::Text(json)).await;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse signaling message: {}", e);
                        }
                    }
                }
                Message::Close(_) => {
                    info!("WebSocket closed for session: {}", session_id);
                    let _ = adapter.cleanup(&session_id).await;
                    break;
                }
                _ => {}
            },
            Some(Err(e)) => {
                error!("WebSocket error: {}", e);
                break;
            }
            None => {
                info!("WebSocket stream ended for session: {}", session_id);
                break;
            }
        }
    }

    // Cleanup when WebSocket closes or errors occur
    info!("WebSocket handler ending, cleaning up session: {}", session_id);
    let _ = adapter.cleanup(&session_id).await;
}

async fn handle_signaling_message(
    message: SignalingMessage,
    session_id: &str,
    adapter: &Arc<WebRTCAdapter>,
    ws_sender: Arc<tokio::sync::Mutex<SplitSink<WebSocket, Message>>>,
    gstreamer: Arc<GStreamerManager>,
    config: crate::domain::aggregates::VideoConfig,
) -> Result<Option<SignalingMessage>> {
    match message {
                SignalingMessage::Resize { width, height } => {
                    info!("Received Resize: width={}, height={}", width, height);
                    // TODO: Implement resize handling if needed
                    Ok(None)
                }
        SignalingMessage::RequestOffer => {
            let sdp = adapter.handle_request_offer(session_id, ws_sender, gstreamer, &config).await?;
            Ok(Some(SignalingMessage::Offer { sdp }))
        }
        SignalingMessage::Answer { sdp } => {
            adapter.handle_answer(session_id, sdp).await?;
            Ok(None)
        }
        SignalingMessage::IceCandidate { candidate, sdp_mid, sdp_mline_index } => {
            adapter.handle_ice_candidate(session_id, candidate, sdp_mid, sdp_mline_index).await?;
            Ok(None)
        }
        SignalingMessage::MouseMove { x, y } => {
            debug!("Received MouseMove: x={}, y={}", x, y);
            let _ = adapter.input_manager.handle_mouse_move(session_id, x, y).await;
            Ok(None)
        }
        SignalingMessage::MouseDown { button } => {
            info!("Received MouseDown: button={}", button);
            let _ = adapter.input_manager.handle_mouse_down(session_id, button).await;
            Ok(None)
        }
        SignalingMessage::MouseUp { button } => {
            info!("Received MouseUp: button={}", button);
            let _ = adapter.input_manager.handle_mouse_up(session_id, button).await;
            Ok(None)
        }
        SignalingMessage::MouseScroll { delta_y } => {
            info!("Received MouseScroll: delta_y={}", delta_y);
            let _ = adapter.input_manager.handle_mouse_scroll(session_id, delta_y).await;
            Ok(None)
        }
        SignalingMessage::KeyDown { key, .. } => {
            info!("Received KeyDown: key={}", key);
            let _ = adapter.input_manager.handle_key_down(session_id, &key).await;
            Ok(None)
        }
        SignalingMessage::KeyUp { key, .. } => {
            info!("Received KeyUp: key={}", key);
            let _ = adapter.input_manager.handle_key_up(session_id, &key).await;
            Ok(None)
        }
        _ => Ok(None),
    }
}

/// Send H.264 video from FFmpeg stdout to WebRTC track
async fn stream_vp8_to_track(
    mut ffmpeg_stdout: ChildStdout,
    track: Arc<TrackLocalStaticSample>,
    cancel_token: CancellationToken,
) -> Result<()> {
    info!("Starting VP8 stream from FFmpeg to WebRTC track");
    
    let mut frame_count = 0u64;
    
    // Read and skip IVF file header (32 bytes)
    let mut header = vec![0u8; 32];
    match ffmpeg_stdout.read_exact(&mut header).await {
        Ok(_) => {},
        Err(e) => {
            error!("Failed to read IVF header: {}", e);
            return Err(e.into());
        }
    }
    
    // VP8 samples for duration calculation (90kHz / 30fps = 3000)
    let _samples_per_frame = 3000u32;
    
    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                info!("Video stream cancelled after {} frames", frame_count);
                break;
            }
            read_result = async {
                // Read IVF frame header (12 bytes: 4 size + 8 timestamp)
                let mut frame_header = vec![0u8; 12];
                match ffmpeg_stdout.read_exact(&mut frame_header).await {
                    Ok(_) => {},
                    Err(e) => return Err(e),
                }
                
                // Parse frame size (little-endian u32)
                let frame_size = u32::from_le_bytes([
                    frame_header[0], frame_header[1], frame_header[2], frame_header[3]
                ]) as usize;
                
                // Read frame data
                let mut frame_data = vec![0u8; frame_size];
                ffmpeg_stdout.read_exact(&mut frame_data).await?;
                
                Ok::<Vec<u8>, std::io::Error>(frame_data)
            } => {
                match read_result {
                    Ok(frame_data) => {
                        // Send raw VP8 frame data (webrtc library handles RTP packetization and VP8 payload descriptor)
                        if let Err(e) = track.write_sample(&webrtc::media::Sample {
                            data: frame_data.into(),
                            duration: std::time::Duration::from_millis(33), // ~30fps
                            ..Default::default()
                        }).await {
                            warn!("Failed to send VP8 sample: {}", e);
                        } else {
                            frame_count += 1;
                            
                            if frame_count % 30 == 0 {
                                debug!("Streamed {} VP8 frames", frame_count);
                            }
                        }
                    }
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::UnexpectedEof {
                            info!("VP8 stream ended normally after {} frames", frame_count);
                        } else {
                            error!("Error reading VP8 frame: {}", e);
                        }
                        break;
                    }
                }
            }
        }
    }
    
    info!("VP8 stream ended after {} frames", frame_count);
    Ok(())
}

use anyhow::Result;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt, stream::{SplitSink, SplitStream}};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use tokio::io::AsyncReadExt;
use tokio::process::ChildStdout;
use bytes::BufMut;
use webrtc::{
    api::{
        media_engine::MediaEngine,
        APIBuilder,
    },
    ice_transport::ice_server::RTCIceServer,
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
use crate::infrastructure::driven::input::xdotool::XdotoolInputManager;

/// Signaling message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum SignalingMessage {
    RequestOffer,
    Offer { sdp: String },
    Answer { sdp: String },
    IceCandidate { candidate: String },
    MouseMove { x: i32, y: i32 },
    MouseDown { button: u8 },
    MouseUp { button: u8 },
    KeyDown { key: String, code: String },
    KeyUp { key: String, code: String },
    Error { message: String },
}

/// WebRTC session manager (adapter for WebRTC)
pub struct WebRTCAdapter {
    peers: Arc<RwLock<HashMap<String, Arc<RTCPeerConnection>>>>,
    tracks: Arc<RwLock<HashMap<String, Arc<TrackLocalStaticSample>>>>,
    cancel_tokens: Arc<RwLock<HashMap<String, CancellationToken>>>,
    ffmpeg_handles: Arc<RwLock<HashMap<String, ChildStdout>>>,
    input_manager: Arc<XdotoolInputManager>,
}

impl WebRTCAdapter {
    pub fn new() -> Self {
        Self {
            peers: Arc::new(RwLock::new(HashMap::new())),
            tracks: Arc::new(RwLock::new(HashMap::new())),
            cancel_tokens: Arc::new(RwLock::new(HashMap::new())),
            ffmpeg_handles: Arc::new(RwLock::new(HashMap::new())),
            input_manager: Arc::new(XdotoolInputManager::new()),
        }
    }
    
    pub async fn register_input_session(&self, session_id: String, display: String) {
        self.input_manager.register_session(session_id, display).await;
    }

    async fn create_peer_connection(&self, session_id: &str) -> Result<(Arc<RTCPeerConnection>, Arc<TrackLocalStaticSample>)> {
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
        
        // Create API with media engine and default interceptors
        let api = APIBuilder::new()
            .with_media_engine(media_engine)
            .build();
        
        // Configure ICE servers
        let config = RTCConfiguration {
            ice_servers: vec![RTCIceServer {
                urls: vec!["stun:stun.l.google.com:19302".to_owned()],
                ..Default::default()
            }],
            ..Default::default()
        };
        
        // Create peer connection
        let peer_connection = Arc::new(api.new_peer_connection(config).await?);
        
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
        
        // Start frame sender task with cancellation token
        let cancel_token = CancellationToken::new();
        let track_clone = Arc::clone(&video_track);
        let token_clone = cancel_token.clone();
        let session_id_string = session_id.to_string();
        let ffmpeg_handles_clone = Arc::clone(&self.ffmpeg_handles);
        
        tokio::spawn(async move {
            // Wait for FFmpeg stdout to be available
            let ffmpeg_stdout = loop {
                let mut handles = ffmpeg_handles_clone.write().await;
                if let Some(stdout) = handles.remove(&session_id_string) {
                    break stdout;
                }
                drop(handles);
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            };
            
            info!("Starting VP8 stream for session: {}", session_id_string);
            if let Err(e) = stream_vp8_to_track(ffmpeg_stdout, track_clone, token_clone).await {
                error!("VP8 streaming error: {}", e);
            }
        });
        
        // Store cancel token
        let mut tokens = self.cancel_tokens.write().await;
        tokens.insert(session_id.to_string(), cancel_token.clone());
        drop(tokens);
        
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

    async fn handle_request_offer(&self, session_id: &str) -> Result<String> {
        info!("Creating WebRTC offer for session: {}", session_id);
        
        // Create peer connection
        let (peer_connection, video_track) = self.create_peer_connection(session_id).await?;
        
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

    async fn handle_ice_candidate(&self, session_id: &str, candidate: String) -> Result<()> {
        debug!("Received ICE candidate from client for session: {}", session_id);
        
        let peers = self.peers.read().await;
        if let Some(pc) = peers.get(session_id) {
            use webrtc::ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit};
            let ice_candidate = RTCIceCandidateInit {
                candidate,
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
    pub async fn set_ffmpeg_stream(&self, session_id: String, ffmpeg_stdout: ChildStdout) {
        let mut handles = self.ffmpeg_handles.write().await;
        handles.insert(session_id, ffmpeg_stdout);
    }

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
        
        let mut peers = self.peers.write().await;
        let mut tracks = self.tracks.write().await;
        
        if let Some(pc) = peers.remove(session_id) {
            pc.close().await?;
            info!("Closed peer connection for session: {}", session_id);
        }
        tracks.remove(session_id);
        
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

pub async fn handle_socket_internal(socket: WebSocket, adapter: Arc<WebRTCAdapter>) {
    let session_id = Uuid::new_v4().to_string();
    handle_socket(socket, adapter, session_id).await
}

async fn handle_socket(socket: WebSocket, adapter: Arc<WebRTCAdapter>, session_id: String) {
    let (mut sender, mut receiver): (SplitSink<WebSocket, Message>, SplitStream<WebSocket>) = socket.split();
    
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
                            ).await;
                            
                            match response {
                                Ok(Some(msg)) => {
                                    if let Ok(json) = serde_json::to_string(&msg) {
                                        let _ = sender.send(Message::Text(json)).await;
                                    }
                                }
                                Ok(None) => {}
                                Err(e) => {
                                    error!("Error handling signaling message: {}", e);
                                    let error_msg = SignalingMessage::Error {
                                        message: e.to_string(),
                                    };
                                    if let Ok(json) = serde_json::to_string(&error_msg) {
                                        let _ = sender.send(Message::Text(json)).await;
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
) -> Result<Option<SignalingMessage>> {
    match message {
        SignalingMessage::RequestOffer => {
            let sdp = adapter.handle_request_offer(session_id).await?;
            Ok(Some(SignalingMessage::Offer { sdp }))
        }
        SignalingMessage::Answer { sdp } => {
            adapter.handle_answer(session_id, sdp).await?;
            Ok(None)
        }
        SignalingMessage::IceCandidate { candidate } => {
            adapter.handle_ice_candidate(session_id, candidate).await?;
            Ok(None)
        }
        SignalingMessage::MouseMove { x, y } => {
            let _ = adapter.input_manager.handle_mouse_move(session_id, x, y).await;
            Ok(None)
        }
        SignalingMessage::MouseDown { button } => {
            let _ = adapter.input_manager.handle_mouse_down(session_id, button).await;
            Ok(None)
        }
        SignalingMessage::MouseUp { button } => {
            let _ = adapter.input_manager.handle_mouse_up(session_id, button).await;
            Ok(None)
        }
        SignalingMessage::KeyDown { key, .. } => {
            let _ = adapter.input_manager.handle_key_down(session_id, &key).await;
            Ok(None)
        }
        SignalingMessage::KeyUp { key, .. } => {
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
    let samples_per_frame = 3000u32;
    
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

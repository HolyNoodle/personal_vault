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
    track::track_local::{track_local_static_rtp::TrackLocalStaticRTP, TrackLocal, TrackLocalWriter},
};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use std::collections::HashMap;

/// Signaling message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum SignalingMessage {
    RequestOffer,
    Offer { sdp: String },
    Answer { sdp: String },
    IceCandidate { candidate: String },
    Error { message: String },
}

/// WebRTC session manager (adapter for WebRTC)
pub struct WebRTCAdapter {
    peers: Arc<RwLock<HashMap<String, Arc<RTCPeerConnection>>>>,
    tracks: Arc<RwLock<HashMap<String, Arc<TrackLocalStaticRTP>>>>,
    cancel_tokens: Arc<RwLock<HashMap<String, CancellationToken>>>,
}

impl WebRTCAdapter {
    pub fn new() -> Self {
        Self {
            peers: Arc::new(RwLock::new(HashMap::new())),
            tracks: Arc::new(RwLock::new(HashMap::new())),
            cancel_tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn create_peer_connection(&self, session_id: &str) -> Result<(Arc<RTCPeerConnection>, Arc<TrackLocalStaticRTP>)> {
        // Create media engine
        let mut media_engine = MediaEngine::default();
        
        // Register H.264 codec
        media_engine.register_codec(
            webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecParameters {
                capability: RTCRtpCodecCapability {
                    mime_type: "video/H264".to_owned(),
                    clock_rate: 90000,
                    channels: 0,
                    sdp_fmtp_line: "level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=42e01f".to_owned(),
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
        let video_track = Arc::new(TrackLocalStaticRTP::new(
            RTCRtpCodecCapability {
                mime_type: "video/H264".to_owned(),
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
        tokio::spawn(async move {
            if let Err(e) = send_test_pattern(track_clone, token_clone).await {
                error!("Frame sender error: {}", e);
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

    pub async fn cleanup(&self, session_id: &str) -> Result<()> {
        info!("Cleaning up WebRTC resources for session: {}", session_id);
        
        // Cancel the frame sender task first
        let mut tokens = self.cancel_tokens.write().await;
        if let Some(token) = tokens.remove(session_id) {
            token.cancel();
            info!("Cancelled frame sender for session: {}", session_id);
        }
        drop(tokens); // Release lock before cleanup
        
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
    State(adapter): State<Arc<WebRTCAdapter>>,
) -> impl axum::response::IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, adapter))
}

pub async fn handle_socket_internal(socket: WebSocket, adapter: Arc<WebRTCAdapter>) {
    handle_socket(socket, adapter).await
}

async fn handle_socket(socket: WebSocket, adapter: Arc<WebRTCAdapter>) {
    let (mut sender, mut receiver): (SplitSink<WebSocket, Message>, SplitStream<WebSocket>) = socket.split();
    let session_id = Uuid::new_v4().to_string();
    
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
        _ => Ok(None),
    }
}

/// Send test pattern frames to the video track
async fn send_test_pattern(track: Arc<TrackLocalStaticRTP>, cancel_token: CancellationToken) -> Result<()> {
    use tokio::time::{interval, Duration};
    
    let mut ticker = interval(Duration::from_millis(33)); // ~30 FPS
    let mut sequence_number: u16 = 0;
    let mut timestamp: u32 = 0;
    let ssrc: u32 = rand::random();
    
    info!("Starting test pattern video stream");
    
    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                info!("Video stream cancelled");
                break;
            }
            _ = ticker.tick() => {}
        }
        
        // Generate a simple colored frame pattern (changes color over time)
        let frame_count = (timestamp / 3000) % 3; // Change color every ~100 frames
        let color_byte = match frame_count {
            0 => 0x80, // Gray
            1 => 0xA0, // Lighter
            _ => 0x60, // Darker
        };
        
        // Create a minimal H.264 NAL unit (I-frame header + fake data)
        let mut nal_data = vec![
            0x00, 0x00, 0x00, 0x01, // NAL start code
            0x65, // NAL unit type: IDR slice
        ];
        
        // Add some payload data (pattern based on timestamp)
        for i in 0..100 {
            nal_data.push(((color_byte as usize + i) % 256) as u8);
        }
        
        // Create RTP packet manually
        let mut rtp_data = vec![];
        
        // RTP Header (12 bytes minimum)
        rtp_data.push(0x80); // V=2, P=0, X=0, CC=0
        rtp_data.push(0xE0); // M=1, PT=96 (H.264)
        rtp_data.extend_from_slice(&sequence_number.to_be_bytes());
        rtp_data.extend_from_slice(&timestamp.to_be_bytes());
        rtp_data.extend_from_slice(&ssrc.to_be_bytes());
        
        // Payload
        rtp_data.extend_from_slice(&nal_data);
        
        // Write RTP packet to track
        if let Err(e) = track.write(&rtp_data).await {
            warn!("Failed to write RTP packet: {}", e);
            break;
        }
        
        sequence_number = sequence_number.wrapping_add(1);
        timestamp = timestamp.wrapping_add(3000); // 90kHz clock / 30 fps
        
        if sequence_number % 300 == 0 {
            debug!("Sent {} frames", sequence_number);
        }
    }
    
    Ok(())
}

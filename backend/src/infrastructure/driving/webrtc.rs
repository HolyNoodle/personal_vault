use crate::infrastructure::driven::sandbox::native_runtime::NativeAppManager;
use crate::infrastructure::driven::sandbox::gstreamer::feed_frames_to_appsrc;
use crate::infrastructure::driven::sandbox::GStreamerManager;
use anyhow::Result;
use axum::extract::{
    ws::{Message, WebSocket},
    State, WebSocketUpgrade,
};
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use webrtc::{
    api::{media_engine::MediaEngine, APIBuilder},
    ice_transport::{
        ice_credential_type::RTCIceCredentialType, ice_server::RTCIceServer,
    },
    peer_connection::{
        configuration::RTCConfiguration,
        peer_connection_state::RTCPeerConnectionState,
        sdp::session_description::RTCSessionDescription, RTCPeerConnection,
    },
    rtp_transceiver::rtp_codec::RTCRtpCodecCapability,
    track::track_local::{track_local_static_sample::TrackLocalStaticSample, TrackLocal},
};

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

/// WebRTC session manager
pub struct WebRTCAdapter {
    peers: Arc<RwLock<HashMap<String, Arc<RTCPeerConnection>>>>,
    tracks: Arc<RwLock<HashMap<String, Arc<TrackLocalStaticSample>>>>,
    cancel_tokens: Arc<RwLock<HashMap<String, CancellationToken>>>,
    native_manager: Arc<NativeAppManager>,
}

impl WebRTCAdapter {
    pub fn new(native_manager: Arc<NativeAppManager>) -> Self {
        Self {
            peers: Arc::new(RwLock::new(HashMap::new())),
            tracks: Arc::new(RwLock::new(HashMap::new())),
            cancel_tokens: Arc::new(RwLock::new(HashMap::new())),
            native_manager,
        }
    }

    async fn create_peer_connection(
        &self,
        session_id: &str,
        ws_sender: Arc<tokio::sync::Mutex<SplitSink<WebSocket, Message>>>,
        gstreamer: Arc<GStreamerManager>,
        config: &crate::domain::aggregates::VideoConfig,
    ) -> Result<(Arc<RTCPeerConnection>, Arc<TrackLocalStaticSample>)> {
        let mut media_engine = MediaEngine::default();

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

        let api = APIBuilder::new().with_media_engine(media_engine).build();

        let turn_server =
            std::env::var("TURN_SERVER").unwrap_or_else(|_| "turn:localhost:3478".to_string());
        let turn_username =
            std::env::var("TURN_USERNAME").unwrap_or_else(|_| "sandbox".to_string());
        let turn_credential =
            std::env::var("TURN_CREDENTIAL").unwrap_or_else(|_| "dev_turn_secret".to_string());

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

        let peer_connection = Arc::new(api.new_peer_connection(rtc_config).await?);

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

        peer_connection
            .add_track(Arc::clone(&video_track) as Arc<dyn TrackLocal + Send + Sync>)
            .await?;

        // Set up cancel token for this session
        let cancel_token = CancellationToken::new();
        let token_clone = cancel_token.clone();

        // Launch WASM app and get frame channel
        let width = config.width;
        let height = config.height;
        let framerate = config.framerate;

        let frame_rx = self
            .native_manager
            .launch_app(session_id, "file-explorer", width, height, framerate)
            .await?;

        // Start GStreamer appsrc pipeline
        let (pipeline, vp8_rx) =
            gstreamer.start_appsrc_pipeline(session_id, width, height, framerate)?;

        // Spawn task to feed RGBA frames from WASM → GStreamer appsrc
        let feed_token = cancel_token.clone();
        let feed_session = session_id.to_string();
        let feed_pipeline = pipeline.clone();
        tokio::spawn(async move {
            feed_frames_to_appsrc(feed_pipeline, frame_rx, feed_token, framerate, feed_session)
                .await;
        });

        // Spawn task to read VP8 frames from GStreamer → WebRTC track
        let track_clone = Arc::clone(&video_track);
        let session_id_string = session_id.to_string();
        tokio::spawn(async move {
            let mut frame_count = 0u64;
            while let Ok(frame_data) = vp8_rx.recv() {
                if token_clone.is_cancelled() {
                    info!(
                        "[session {}] VP8 stream cancelled after {} frames",
                        session_id_string, frame_count
                    );
                    break;
                }
                if let Err(e) = track_clone
                    .write_sample(&webrtc::media::Sample {
                        data: frame_data.into(),
                        duration: std::time::Duration::from_millis(1000 / framerate.max(1) as u64),
                        ..Default::default()
                    })
                    .await
                {
                    warn!("Failed to send VP8 sample: {}", e);
                } else {
                    frame_count += 1;
                    if frame_count % 30 == 0 {
                        debug!("Streamed {} VP8 frames", frame_count);
                    }
                }
            }
            info!(
                "[session {}] VP8 stream ended after {} frames",
                session_id_string, frame_count
            );
        });

        // Store cancel token
        let mut tokens = self.cancel_tokens.write().await;
        tokens.insert(session_id.to_string(), cancel_token.clone());
        drop(tokens);

        // ICE candidate handler
        let ws_sender_clone = Arc::clone(&ws_sender);
        peer_connection.on_ice_candidate(Box::new(
            move |candidate: Option<webrtc::ice_transport::ice_candidate::RTCIceCandidate>| {
                let sender = Arc::clone(&ws_sender_clone);
                Box::pin(async move {
                    if let Some(candidate) = candidate {
                        match candidate.to_json() {
                            Ok(json_candidate) => {
                                let msg = SignalingMessage::IceCandidate {
                                    candidate: json_candidate.candidate,
                                    sdp_mid: json_candidate.sdp_mid,
                                    sdp_mline_index: json_candidate
                                        .sdp_mline_index
                                        .map(|v| v as u16),
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
            },
        ));

        // Connection state handler
        let session_id_clone = session_id.to_string();
        let cancel_token_clone = cancel_token.clone();
        peer_connection.on_peer_connection_state_change(Box::new(
            move |state: RTCPeerConnectionState| {
                let session = session_id_clone.clone();
                let token = cancel_token_clone.clone();
                Box::pin(async move {
                    info!("Peer connection state changed: {}", state);
                    match state {
                        RTCPeerConnectionState::Failed
                        | RTCPeerConnectionState::Disconnected
                        | RTCPeerConnectionState::Closed => {
                            warn!(
                                "Connection {} failed/disconnected/closed, stopping streams",
                                session
                            );
                            token.cancel();
                        }
                        _ => {}
                    }
                })
            },
        ));

        Ok((peer_connection, video_track))
    }

    async fn handle_request_offer(
        &self,
        session_id: &str,
        ws_sender: Arc<tokio::sync::Mutex<SplitSink<WebSocket, Message>>>,
        gstreamer: Arc<GStreamerManager>,
        config: &crate::domain::aggregates::VideoConfig,
    ) -> Result<String> {
        info!("Creating WebRTC offer for session: {}", session_id);

        let (peer_connection, video_track) = self
            .create_peer_connection(session_id, ws_sender, gstreamer, config)
            .await?;

        let offer = peer_connection.create_offer(None).await?;
        let offer_sdp = offer.sdp.clone();
        peer_connection.set_local_description(offer).await?;

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

    async fn handle_ice_candidate(
        &self,
        session_id: &str,
        candidate: String,
        sdp_mid: Option<String>,
        sdp_mline_index: Option<u16>,
    ) -> Result<()> {
        info!(
            "Received ICE candidate from client for session: {}",
            session_id
        );

        let peers = self.peers.read().await;
        if let Some(pc) = peers.get(session_id) {
            use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
            let ice_candidate = RTCIceCandidateInit {
                candidate,
                sdp_mid,
                sdp_mline_index,
                ..Default::default()
            };
            pc.add_ice_candidate(ice_candidate).await?;
            info!("Added ICE candidate for session: {}", session_id);
        } else {
            return Err(anyhow::anyhow!("Peer connection not found"));
        }

        Ok(())
    }

    pub async fn cleanup(&self, session_id: &str) -> Result<()> {
        info!(
            "Cleaning up WebRTC resources for session: {}",
            session_id
        );

        // Cancel streaming tasks
        let mut tokens = self.cancel_tokens.write().await;
        if let Some(token) = tokens.remove(session_id) {
            token.cancel();
            info!("Cancelled streams for session: {}", session_id);
        }
        drop(tokens);

        // Cleanup native session
        let _ = self.native_manager.cleanup_session(session_id).await;

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
    let session_id = params
        .get("session")
        .cloned()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    ws.on_upgrade(move |socket| handle_socket(socket, adapter, session_id))
}

async fn handle_socket(socket: WebSocket, adapter: Arc<WebRTCAdapter>, session_id: String) {
    let (sender, mut receiver): (SplitSink<WebSocket, Message>, SplitStream<WebSocket>) =
        socket.split();
    let sender = Arc::new(tokio::sync::Mutex::new(sender));

    let gstreamer = Arc::new(
        crate::infrastructure::driven::sandbox::GStreamerManager::new()
            .expect("Failed to init GStreamer"),
    );
    let config = crate::domain::aggregates::VideoConfig::default();

    info!(
        "WebSocket connection established for session: {}",
        session_id
    );

    loop {
        match receiver.next().await {
            Some(Ok(msg)) => match msg {
                Message::Text(text) => {
                    info!("Received message: {}", text);
                    match serde_json::from_str::<SignalingMessage>(&text) {
                        Ok(message) => {
                            let response = handle_signaling_message(
                                message,
                                &session_id,
                                &adapter,
                                Arc::clone(&sender),
                                Arc::clone(&gstreamer),
                                config.clone(),
                            )
                            .await;
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

    info!(
        "WebSocket handler ending, cleaning up session: {}",
        session_id
    );
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
        SignalingMessage::RequestOffer => {
            let sdp = adapter
                .handle_request_offer(session_id, ws_sender, gstreamer, &config)
                .await?;
            Ok(Some(SignalingMessage::Offer { sdp }))
        }
        SignalingMessage::Answer { sdp } => {
            adapter.handle_answer(session_id, sdp).await?;
            Ok(None)
        }
        SignalingMessage::IceCandidate {
            candidate,
            sdp_mid,
            sdp_mline_index,
        } => {
            adapter
                .handle_ice_candidate(session_id, candidate, sdp_mid, sdp_mline_index)
                .await?;
            Ok(None)
        }
        SignalingMessage::MouseMove { x, y } => {
            info!("Received MouseMove: x={}, y={}", x, y);
            adapter
                .native_manager
                .handle_pointer_event(session_id, x as f32, y as f32, false)
                .await;
            Ok(None)
        }
        SignalingMessage::MouseDown { button } => {
            info!("Received MouseDown: button={}", button);
            adapter
                .native_manager
                .handle_mouse_button(session_id, button, true)
                .await;
            Ok(None)
        }
        SignalingMessage::MouseUp { button } => {
            info!("Received MouseUp: button={}", button);
            adapter
                .native_manager
                .handle_mouse_button(session_id, button, false)
                .await;
            Ok(None)
        }
        SignalingMessage::MouseScroll { delta_y } => {
            info!("Received MouseScroll: delta_y={}", delta_y);
            // TODO: Forward scroll events to WASM app
            Ok(None)
        }
        SignalingMessage::KeyDown { key, code } => {
            info!("Received KeyDown: key={}", key);
            adapter
                .native_manager
                .handle_keyboard(session_id, key, code, true)
                .await;
            Ok(None)
        }
        SignalingMessage::KeyUp { key, code } => {
            info!("Received KeyUp: key={}", key);
            adapter
                .native_manager
                .handle_keyboard(session_id, key, code, false)
                .await;
            Ok(None)
        }
        SignalingMessage::Resize { width, height } => {
            info!("Received Resize: width={}, height={}", width, height);
            // If you need to update the running session, call a method here, e.g.:
            // adapter.wasm_manager.set_resolution(session_id, width, height).await;
            // If you need to update framerate, add it to the message and handle accordingly.
            Ok(None)
        }
        _ => Ok(None),
    }
}

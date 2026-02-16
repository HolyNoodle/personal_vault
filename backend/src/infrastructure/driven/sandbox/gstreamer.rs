use anyhow::{Context, Result};
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app::{AppSink, AppSrc};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

pub struct GStreamerManager {}

impl GStreamerManager {
    pub fn new() -> Result<Self> {
        gst::init().context("Failed to initialize GStreamer")?;
        Ok(Self {})
    }

    /// Start a pipeline that accepts raw RGBA frames via appsrc and outputs VP8 via appsink.
    /// Returns a std::sync::mpsc::Receiver<Vec<u8>> for VP8 encoded frames.
    ///
    /// The caller must push RGBA frames into the returned pipeline's appsrc element.
    pub fn start_appsrc_pipeline(
        &self,
        session_id: &str,
        width: u16,
        height: u16,
        framerate: u8,
    ) -> Result<(gst::Pipeline, std::sync::mpsc::Receiver<Vec<u8>>)> {
        info!(
            "Starting GStreamer appsrc pipeline for session {} ({}x{}@{}fps)",
            session_id, width, height, framerate
        );

        let pipeline = gst::Pipeline::default();

        // appsrc: accepts raw RGBA frames pushed from the WASM render loop
        let appsrc = gst::ElementFactory::make("appsrc")
            .name("appsrc")
            .property("is-live", true)
            .property("format", gst::Format::Time)
            .property(
                "caps",
                &gst::Caps::builder("video/x-raw")
                    .field("format", "RGBA")
                    .field("width", width as i32)
                    .field("height", height as i32)
                    .field("framerate", gst::Fraction::new(framerate as i32, 1))
                    .build(),
            )
            .build()
            .context("Failed to create appsrc")?;

        let videoconvert = gst::ElementFactory::make("videoconvert")
            .build()
            .context("Failed to create videoconvert")?;

        let vp8enc = gst::ElementFactory::make("vp8enc")
            .property("deadline", 1i64)
            .property("cpu-used", 8i32)
            .property("target-bitrate", 1_000_000i32)
            .build()
            .context("Failed to create vp8enc")?;

        let appsink = gst::ElementFactory::make("appsink")
            .name("appsink")
            .property("sync", false)
            .property("emit-signals", true)
            .property_from_str("max-buffers", "100")
            .property("drop", true)
            .build()
            .context("Failed to create appsink")?;

        pipeline.add_many([&appsrc, &videoconvert, &vp8enc, &appsink])?;

        appsrc
            .link(&videoconvert)
            .context("Failed to link appsrc -> videoconvert")?;
        videoconvert
            .link(&vp8enc)
            .context("Failed to link videoconvert -> vp8enc")?;
        vp8enc
            .link(&appsink)
            .context("Failed to link vp8enc -> appsink")?;

        // Set up appsink callback to forward VP8 frames
        let appsink_el = appsink
            .downcast::<AppSink>()
            .map_err(|_| anyhow::anyhow!("Failed to downcast to AppSink"))?;

        let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
        let tx = Arc::new(tx);

        let tx_cb = Arc::clone(&tx);
        appsink_el.set_callbacks(
            gstreamer_app::AppSinkCallbacks::builder()
                .new_sample(move |sink| {
                    let sample = match sink.pull_sample() {
                        Ok(s) => s,
                        Err(_) => return Err(gst::FlowError::Eos),
                    };
                    if let Some(buffer) = sample.buffer() {
                        if let Ok(map) = buffer.map_readable() {
                            let data = map.as_slice().to_vec();
                            let _ = tx_cb.send(data);
                        }
                    }
                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
        );

        pipeline.set_state(gst::State::Playing)?;

        // Monitor bus for errors in a background thread
        let pipeline_clone = pipeline.clone();
        let session_id_owned = session_id.to_string();
        std::thread::spawn(move || {
            let bus = pipeline_clone.bus().unwrap();
            for msg in bus.iter_timed(gst::ClockTime::NONE) {
                use gst::MessageView;
                match msg.view() {
                    MessageView::Error(err) => {
                        error!(
                            "[session {}] GStreamer error: {} (debug: {:?})",
                            session_id_owned,
                            err.error(),
                            err.debug()
                        );
                        let _ = pipeline_clone.set_state(gst::State::Null);
                        break;
                    }
                    MessageView::Eos(..) => {
                        info!("[session {}] GStreamer EOS", session_id_owned);
                        let _ = pipeline_clone.set_state(gst::State::Null);
                        break;
                    }
                    _ => (),
                }
            }
        });

        Ok((pipeline, rx))
    }

    pub fn stop_pipeline(&self, pipeline: &gst::Pipeline) -> Result<()> {
        pipeline.set_state(gst::State::Null)?;
        Ok(())
    }
}

/// Push RGBA frames from a tokio mpsc channel into a GStreamer appsrc element.
/// This bridges the async WASM render loop to the GStreamer pipeline.
pub async fn feed_frames_to_appsrc(
    pipeline: gst::Pipeline,
    mut frame_rx: mpsc::Receiver<Vec<u8>>,
    cancel_token: tokio_util::sync::CancellationToken,
    framerate: u8,
    session_id: String,
) {
    let appsrc_el = match pipeline.by_name("appsrc") {
        Some(el) => el,
        None => {
            error!("[session {}] appsrc element not found in pipeline", session_id);
            return;
        }
    };

    let appsrc = match appsrc_el.downcast::<AppSrc>() {
        Ok(src) => src,
        Err(_) => {
            error!("[session {}] Failed to downcast to AppSrc", session_id);
            return;
        }
    };

    let frame_duration = gst::ClockTime::from_nseconds(1_000_000_000 / framerate.max(1) as u64);
    let mut pts = gst::ClockTime::ZERO;
    let mut frame_count = 0u64;

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                info!("[session {}] Frame feeder cancelled after {} frames", session_id, frame_count);
                let _ = appsrc.end_of_stream();
                break;
            }
            frame = frame_rx.recv() => {
                match frame {
                    Some(data) => {
                        let mut buffer = gst::Buffer::from_slice(data);
                        {
                            let buf_ref = buffer.get_mut().unwrap();
                            buf_ref.set_pts(pts);
                            buf_ref.set_duration(frame_duration);
                        }
                        pts += frame_duration;

                        if let Err(e) = appsrc.push_buffer(buffer) {
                            warn!("[session {}] Failed to push buffer to appsrc: {:?}", session_id, e);
                            break;
                        }

                        frame_count += 1;
                        if frame_count % 30 == 0 {
                            debug!("[session {}] Fed {} frames to GStreamer", session_id, frame_count);
                        }
                    }
                    None => {
                        info!("[session {}] Frame channel closed after {} frames", session_id, frame_count);
                        let _ = appsrc.end_of_stream();
                        break;
                    }
                }
            }
        }
    }
}

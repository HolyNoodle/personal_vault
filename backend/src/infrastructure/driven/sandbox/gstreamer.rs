use tokio_util::sync::CancellationToken;
/// Push RGBA frames from a tokio mpsc channel into a GStreamer appsrc element.
/// Cette fonction lit les frames RGBA depuis un channel et les pousse dans l'appsrc du pipeline.
pub async fn feed_frames_to_appsrc(
    pipeline: gst::Pipeline,
    mut frame_rx: tokio::sync::mpsc::Receiver<Vec<u8>>,
    cancel_token: CancellationToken,
    framerate: u8,
    session_id: String,
) {
    use gstreamer_app::AppSrc;
    use tracing::{info, warn};

    let appsrc_el = match pipeline.by_name("appsrc") {
        Some(el) => el,
        None => {
            warn!("[session {}] appsrc element not found in pipeline", session_id);
            return;
        }
    };
    let appsrc = match appsrc_el.downcast::<AppSrc>() {
        Ok(src) => src,
        Err(_) => {
            warn!("[session {}] Failed to downcast to AppSrc", session_id);
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
                        // Log first 32 bytes (8 pixels RGBA) of the buffer as received from the source (shared memory/channel)
                        if frame_count < 10 {
                            let mut px_str = String::new();
                            for i in 0..8.min(data.len()/4) {
                                let base = i*4;
                                px_str.push_str(&format!("[R:{} G:{} B:{} A:{}] ", data[base], data[base+1], data[base+2], data[base+3]));
                            }
                            debug!("[session {}] RGBA SOURCE buffer (first 8 px): {} (len={})", session_id, px_str, data.len());
                        }
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
use anyhow::{Context, Result};
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app::AppSink;
use std::sync::Arc;
use tracing::{debug, error, info};

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
        info!("Starting GStreamer appsrc pipeline for session {} ({}x{}@{}fps)", session_id, width, height, framerate);
        let pipeline = gst::Pipeline::default();

        // appsrc: accepts raw RGBA frames pushed from the WASM render loop
        info!("[session {}] Creating appsrc", session_id);
        let appsrc = gst::ElementFactory::make("appsrc")
            .name("appsrc")
            .property("is-live", true)
            .property("format", gst::Format::Time)
            .property("do-timestamp", true)
            .build()
            .context("Failed to create appsrc")?;

        // Explicitly set caps with stride and pixel-aspect-ratio
        let stride = width as i32 * 4;
        let caps = gst::Caps::builder("video/x-raw")
            .field("format", "RGBA")
            .field("width", width as i32)
            .field("height", height as i32)
            .field("framerate", gst::Fraction::new(framerate as i32, 1))
            .field("stride", stride)
            .field("pixel-aspect-ratio", gst::Fraction::new(1, 1))
            .build();
        appsrc.set_property("caps", &caps);

        info!("[session {}] Creating videoconvert", session_id);
        let videoconvert = gst::ElementFactory::make("videoconvert")
            .build()
            .context("Failed to create videoconvert")?;

        info!("[session {}] Creating capsfilter (I420)", session_id);
        let capsfilter = gst::ElementFactory::make("capsfilter")
            .property(
                "caps",
                &gst::Caps::builder("video/x-raw")
                    .field("format", "I420")
                    .field("width", width as i32)
                    .field("height", height as i32)
                    .field("framerate", gst::Fraction::new(framerate as i32, 1))
                    .field("pixel-aspect-ratio", gst::Fraction::new(1, 1))
                    .build(),
            )
            .build()
            .context("Failed to create capsfilter for I420")?;

        info!("[session {}] Creating vp8enc", session_id);
        let vp8enc = gst::ElementFactory::make("vp8enc")
            .property("deadline", 1i64)
            .property("cpu-used", 8i32)
            .property("target-bitrate", 1_000_000i32)
            .build()
            .context("Failed to create vp8enc")?;

        info!("[session {}] Creating appsink", session_id);
        let appsink = gst::ElementFactory::make("appsink")
            .name("appsink")
            .property("sync", false)
            .property("emit-signals", true)
            .property_from_str("max-buffers", "100")
            .property("drop", true)
            .build()
            .context("Failed to create appsink")?;

        info!("[session {}] Adding elements to pipeline", session_id);
        pipeline.add_many([&appsrc, &videoconvert, &capsfilter, &vp8enc, &appsink])?;

        info!("[session {}] Linking appsrc -> videoconvert", session_id);
        appsrc.link(&videoconvert).context("Failed to link appsrc -> videoconvert")?;
        info!("[session {}] Linking videoconvert -> capsfilter", session_id);
        videoconvert.link(&capsfilter).context("Failed to link videoconvert -> capsfilter")?;
        info!("[session {}] Linking capsfilter -> vp8enc", session_id);
        capsfilter.link(&vp8enc).context("Failed to link capsfilter -> vp8enc")?;
        info!("[session {}] Linking vp8enc -> appsink", session_id);
        vp8enc.link(&appsink).context("Failed to link vp8enc -> appsink")?;

        // Add a pad probe to the capsfilter's src pad to log I420 data
        {
            let session_id_owned = session_id.to_string();
            let capsfilter_src_pad = capsfilter.static_pad("src").expect("capsfilter has no src pad");
            capsfilter_src_pad.add_probe(gst::PadProbeType::BUFFER, move |_pad, info| {
                if let Some(gst::PadProbeData::Buffer(ref buffer)) = info.data {
                    if let Ok(map) = buffer.map_readable() {
                        let data: &[u8] = map.as_slice();
                        let mut px_str = String::new();
                        for i in 0..8.min(data.len()) {
                            px_str.push_str(&format!("{} ", data[i]));
                        }
                        debug!("[session {}] I420 after capsfilter (first 8 bytes): {}", session_id_owned, px_str);
                    }
                }
                gst::PadProbeReturn::Ok
            });
        }

        info!("[session {}] Pipeline created and linked", session_id);

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
                            let data = map.as_slice();
                            // Log first 8 bytes at appsink (VP8 output or I420 if before encoder)
                            let mut px_str = String::new();
                            for i in 0..8.min(data.len()) {
                                px_str.push_str(&format!("{} ", data[i]));
                            }
                            debug!("[session] Appsink buffer first 8 bytes: {}", px_str);
                            let _ = tx_cb.send(data.to_vec());
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
}

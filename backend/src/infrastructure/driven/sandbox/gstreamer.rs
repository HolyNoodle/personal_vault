use anyhow::{Context, Result};
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app::AppSink;
use std::sync::Arc;
use tracing::{error, info};

pub struct GStreamerManager {}

impl GStreamerManager {
    pub fn new() -> Result<Self> {
        gst::init().context("Failed to initialize GStreamer")?;
        Ok(Self {})
    }

    /// Start a pipeline that captures from an Xvfb display via ximagesrc and outputs VP8 via
    /// appsink. Returns a std::sync::mpsc::Receiver<Vec<u8>> for VP8 encoded frames.
    pub fn start_ximagesrc_pipeline(
        &self,
        session_id: &str,
        display_str: &str,
        framerate: u8,
    ) -> Result<(gst::Pipeline, std::sync::mpsc::Receiver<Vec<u8>>)> {
        info!(
            "Starting GStreamer ximagesrc pipeline for session {:?} on display {:?} @{:?}fps",
            session_id, display_str, framerate
        );

        let ximagesrc = gst::ElementFactory::make("ximagesrc")
            .property_from_str("display-name", display_str)
            .property("use-damage", false)
            .build()
            .context("Failed to create ximagesrc")?;

        let videoconvert = gst::ElementFactory::make("videoconvert")
            .build()
            .context("Failed to create videoconvert")?;

        let capsfilter = gst::ElementFactory::make("capsfilter")
            .property(
                "caps",
                &gst::Caps::builder("video/x-raw")
                    .field("format", "I420")
                    .field("framerate", gst::Fraction::new(framerate as i32, 1))
                    .build(),
            )
            .build()
            .context("Failed to create capsfilter")?;

        let vp8enc = gst::ElementFactory::make("vp8enc")
            .property("deadline", 1i64)
            .property("cpu-used", 8i32)
            .property("target-bitrate", 1_000_000i32)
            .build()
            .context("Failed to create vp8enc")?;

        let appsink = gst::ElementFactory::make("appsink")
            .name("sink")
            .property("sync", false)
            .property("emit-signals", true)
            .property_from_str("max-buffers", "100")
            .property("drop", true)
            .build()
            .context("Failed to create appsink")?;

        let pipeline = gst::Pipeline::default();
        pipeline.add_many([&ximagesrc, &videoconvert, &capsfilter, &vp8enc, &appsink])?;
        ximagesrc.link(&videoconvert).context("Failed to link ximagesrc -> videoconvert")?;
        videoconvert.link(&capsfilter).context("Failed to link videoconvert -> capsfilter")?;
        capsfilter.link(&vp8enc).context("Failed to link capsfilter -> vp8enc")?;
        vp8enc.link(&appsink).context("Failed to link vp8enc -> appsink")?;

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
                            let _ = tx_cb.send(map.as_slice().to_vec());
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

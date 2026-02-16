// ...existing code...
// GStreamer-based video capture and encoding for X11 display
// Replaces ffmpeg.rs for screen capture and streaming

use anyhow::{Context, Result};
use std::sync::{Arc, mpsc};
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app::AppSink;
use tracing::info;

pub struct GStreamerManager {
    // Add fields as needed for pipeline management
}

impl GStreamerManager {
    pub fn new() -> Result<Self> {
        gst::init().context("Failed to initialize GStreamer")?;
        Ok(Self {})
    }

    pub fn start_pipeline(
        &self,
        session_id: &str,
        display_str: &str,
        width: u16,
        height: u16,
        framerate: u8,
    ) -> Result<gst::Pipeline> {
        info!(
            "Starting GStreamer pipeline for session {} on display {} ({}x{}@{}fps)",
            session_id, display_str, width, height, framerate
        );

        // Set DISPLAY environment variable for this thread/process
        std::env::set_var("DISPLAY", display_str);

        // Manually build the pipeline
        let pipeline = gst::Pipeline::default();

        let ximagesrc = gst::ElementFactory::make("ximagesrc")
            .property("display-name", display_str)
            .property("use-damage", false as bool)
            .build()
            .context("Failed to create ximagesrc")?;

        let capsfilter = gst::ElementFactory::make("capsfilter")
            .property(
                "caps",
                &gst::Caps::builder("video/x-raw")
                    .field("width", width as i32)
                    .field("height", height as i32)
                    .field("framerate", gst::Fraction::new(framerate as i32, 1))
                    .build(),
            )
            .build()
            .context("Failed to create capsfilter")?;

        let videoconvert = gst::ElementFactory::make("videoconvert")
            .build()
            .context("Failed to create videoconvert")?;

        let vp8enc = gst::ElementFactory::make("vp8enc")
            .property("deadline", 1i64)
            .property("cpu-used", 8i32)
            .build()
            .context("Failed to create vp8enc")?;

        let vp8_capsfilter = gst::ElementFactory::make("capsfilter")
            .property(
                "caps",
                &gst::Caps::builder("video/x-vp8").build(),
            )
            .build()
            .context("Failed to create vp8_capsfilter")?;

        let appsink = gst::ElementFactory::make("appsink")
            .property("name", "appsink")
            .property("sync", false)
            .property("emit-signals", true)
            .property_from_str("max-buffers", "100")
            .property("drop", true)
            .build()
            .context("Failed to create appsink")?;

        pipeline.add_many([
            &ximagesrc,
            &capsfilter,
            &videoconvert,
            &vp8enc,
            &vp8_capsfilter,
            &appsink,
        ])?;

        // Link elements step-by-step and check errors
        ximagesrc.link(&capsfilter).context("Failed to link ximagesrc -> capsfilter")?;
        capsfilter.link(&videoconvert).context("Failed to link capsfilter -> videoconvert")?;
        videoconvert.link(&vp8enc).context("Failed to link videoconvert -> vp8enc")?;
        vp8enc.link(&vp8_capsfilter).context("Failed to link vp8enc -> vp8_capsfilter")?;
        vp8_capsfilter.link(&appsink).context("Failed to link vp8_capsfilter -> appsink")?;

        Ok(pipeline)
    }

    /// Start the pipeline and return a receiver for IVF VP8 frames
    pub fn start_vp8_ivf_stream(
        &self,
        session_id: &str,
        display_str: &str,
        width: u16,
        height: u16,
        framerate: u8,
    ) -> Result<mpsc::Receiver<Vec<u8>>> {
        let pipeline = self.start_pipeline(session_id, display_str, width, height, framerate)?;
        let appsink = pipeline
            .by_name("appsink")
            .ok_or_else(|| anyhow::anyhow!("appsink not found in pipeline"))?
            .downcast::<AppSink>()
            .map_err(|_| anyhow::anyhow!("Failed to downcast to AppSink"))?;

        let (tx, rx): (std::sync::mpsc::Sender<Vec<u8>>, std::sync::mpsc::Receiver<Vec<u8>>) = std::sync::mpsc::channel();
        let tx = Arc::new(tx);

        // Set up the appsink callback to send data to the channel
        let tx_cb = Arc::clone(&tx);
        appsink.set_callbacks(
            gstreamer_app::AppSinkCallbacks::builder()
                .new_sample(move |appsink| {
                    let sample = match appsink.pull_sample() {
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

        // Optionally, handle EOS and errors on the bus in a background thread
        let pipeline_clone = pipeline.clone();
        std::thread::spawn(move || {
            let bus = pipeline_clone.bus().unwrap();
            for msg in bus.iter_timed(gst::ClockTime::NONE) {
                use gst::MessageView;
                match msg.view() {
                    MessageView::Eos(..) | MessageView::Error(_) => {
                        let _ = pipeline_clone.set_state(gst::State::Null);
                        break;
                    }
                    _ => (),
                }
            }
        });

        Ok(rx)
    }

    pub fn stop_pipeline(&self, pipeline: gst::Pipeline) -> Result<()> {
        pipeline.set_state(gst::State::Null)?;
        Ok(())
    }
}

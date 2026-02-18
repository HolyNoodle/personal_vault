mod app;

use eframe::egui;

fn main() -> eframe::Result {
    let width = std::env::var("SANDBOX_WIDTH")
        .ok()
        .and_then(|w| w.parse::<f32>().ok())
        .unwrap_or(800.0);
    let height = std::env::var("SANDBOX_HEIGHT")
        .ok()
        .and_then(|h| h.parse::<f32>().ok())
        .unwrap_or(600.0);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("File Explorer")
            .with_inner_size([width, height]),
        ..Default::default()
    };
    eframe::run_native(
        "File Explorer",
        options,
        Box::new(|_cc| Ok(Box::new(app::FileExplorerApp::default()))),
    )
}

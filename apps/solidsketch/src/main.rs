//! SolidSketch for the desktop. The browser build is `web.rs`; both run the same app (`lib.rs`).

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 720.0]).with_title("SolidSketch"),
        ..Default::default()
    };
    eframe::run_native("SolidSketch", options, Box::new(|_cc| Ok(Box::new(solidsketch_app::SketchApp::new()))))
}

#[cfg(target_arch = "wasm32")]
fn main() {}

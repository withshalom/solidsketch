//! SolidSketch in the browser: the same app as the desktop, through eframe's web runner (WebGPU
//! where the browser has it, WebGL2 otherwise). Build with `trunk build --release` in this folder;
//! the site lands in `dist/web` at the repository root.

#[cfg(target_arch = "wasm32")]
mod web {
    use wasm_bindgen::JsCast as _;

    const CANVAS_ID: &str = "solidsketch_canvas";
    const LOADING_ID: &str = "solidsketch_loading";

    pub fn start() {
        eframe::WebLogger::init(log::LevelFilter::Info).ok();
        wasm_bindgen_futures::spawn_local(async {
            let Some(document) = web_sys::window().and_then(|w| w.document()) else { return };
            let Some(canvas) = document.get_element_by_id(CANVAS_ID).and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok()) else {
                log::error!("missing <canvas id=\"{CANVAS_ID}\">");
                return;
            };
            let started = eframe::WebRunner::new()
                .start(canvas, eframe::WebOptions::default(), Box::new(|_cc| Ok(Box::new(solidsketch_app::SketchApp::new()))))
                .await;
            if let Some(loading) = document.get_element_by_id(LOADING_ID) {
                loading.remove();
            }
            if let Err(e) = started {
                log::error!("SolidSketch did not start: {e:?}");
            }
        });
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    web::start();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("solidsketch-web only runs in the browser: build it with `trunk build --release` in apps/solidsketch");
}

//! The SolidSketch app: a toolbar, a viewport and a status bar over the SolidCraft engine.
//!
//! All of the behaviour that decides what the engine is asked to do lives in `solidsketch-tools`
//! (and is tested there without a screen). This file only turns egui's mouse and keyboard input
//! into that crate's calls, runs the commands it returns on a SolidCraft `Session`, and shows the
//! result: the engine's scene drawn by the shared CPU rasterizer into one texture. A CPU picture
//! is slower than a GPU one, but it runs the same everywhere, desktop and browser, which is what a
//! first testable version needs. The GPU viewport replaces it later.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use egui::{Align2, Color32, FontId, Key, PointerButton, Pos2, Rect, Sense, Stroke, TextureHandle, TextureOptions};
use solidcore_record::Vec3;
use solidcraft_engine::Session;
use solidcraft_viewport::{Camera, render};
use solidsketch_tools::{Command, Modeler, P2, Preview, Tool, ToolError, view};

/// The longest side, in pixels, the picture is drawn at. The rasterizer runs on the CPU, so a big
/// window is drawn smaller and stretched.
const MAX_PICTURE: f32 = 1000.0;
const ACCENT: Color32 = Color32::from_rgb(255, 140, 0);

/// Tools that do something today. The rest of the toolbar is there, greyed, so the shape of the
/// finished app is visible.
fn works(tool: Tool) -> bool {
    matches!(tool, Tool::Line | Tool::Rectangle | Tool::Circle | Tool::PushPull | Tool::Orbit)
}

pub struct SketchApp {
    session: Session,
    camera: Camera,
    modeler: Modeler,
    tool: Tool,
    texture: Option<TextureHandle>,
    /// The picture is out of date: the model, the camera or the window size changed.
    stale: bool,
    picture_size: (usize, usize),
    scene_radius: f64,
    /// What the last drawing event wants shown while the user decides.
    preview: Preview,
    readout: String,
    status: String,
    status_is_error: bool,
    /// Where the pointer was when a pull began (screen y), to measure the drag.
    pull_origin_y: f32,
}

impl Default for SketchApp {
    fn default() -> Self {
        Self::new()
    }
}

impl SketchApp {
    pub fn new() -> Self {
        Self {
            session: Session::default(),
            camera: Camera::default(),
            modeler: Modeler::new(),
            tool: Tool::Rectangle,
            texture: None,
            stale: true,
            picture_size: (0, 0),
            scene_radius: 100.0,
            preview: Preview::None,
            readout: String::new(),
            status: "Rectangle: click two corners on the ground. Then Push/Pull (P): drag the shape up.".to_string(),
            status_is_error: false,
            pull_origin_y: 0.0,
        }
    }

    fn say(&mut self, text: impl Into<String>) {
        self.status = text.into();
        self.status_is_error = false;
    }

    fn fail(&mut self, text: impl Into<String>) {
        self.status = text.into();
        self.status_is_error = true;
    }

    fn pick(&mut self, tool: Tool) {
        if !works(tool) {
            return;
        }
        self.tool = tool;
        self.modeler.set_tool(tool);
        self.preview = Preview::None;
        self.readout.clear();
        self.say(tool.hint());
    }

    /// Runs commands on the engine, in order. The first one the engine refuses stops the rest and
    /// the modeler forgets what it thought the engine held, so the next shape starts clean.
    fn run(&mut self, commands: &[Command]) -> bool {
        for c in commands {
            let params = match serde_json::from_str::<serde_json::Value>(&c.params) {
                Ok(v) => v,
                Err(e) => {
                    self.fail(format!("{}: the tool made unreadable parameters ({e})", c.id));
                    self.modeler.forget();
                    return false;
                }
            };
            if let Err(e) = self.session.execute(c.id, &params) {
                self.fail(format!("{}: {e}", c.id));
                self.modeler.forget();
                self.stale = true;
                return false;
            }
        }
        if !commands.is_empty() {
            self.stale = true;
        }
        true
    }

    fn undo(&mut self) {
        match self.session.execute("edit.undo", &serde_json::json!({})) {
            Ok(_) => {
                self.say("Undone.");
                self.stale = true;
            }
            Err(e) => self.fail(format!("Nothing to undo ({e})")),
        }
        // What the engine holds changed under the modeler: start its picture of it clean.
        self.modeler.forget();
        self.preview = Preview::None;
    }

    fn new_model(&mut self) {
        self.session = Session::default();
        self.modeler = Modeler::new();
        self.modeler.set_tool(self.tool);
        self.preview = Preview::None;
        self.readout.clear();
        self.stale = true;
        self.say("New model.");
    }

    fn explain(error: &ToolError) -> &'static str {
        match error {
            ToolError::NotFinite => "That point is not a number.",
            ToolError::TooFewPoints => "A shape needs more points.",
            ToolError::ZeroSize => "That has no size. Drag a little, or type a size.",
            ToolError::NoFace => "Nothing to pull yet. Draw a shape first.",
            ToolError::BadValue => "Type a number, like 40, or width,height like 40,30.",
            ToolError::NoDirection => "Move the pointer to say which way, then type the length.",
        }
    }

    /// The point on the ground under a screen position, in model units.
    fn ground_at(&self, pos: Pos2, rect: Rect, (pw, ph): (usize, usize)) -> Option<P2> {
        let (x, y) = Self::picture_xy(pos, rect, (pw, ph));
        let (o, d) = self.camera.ray(x, y, pw as f64, ph as f64);
        view::ground_hit([o.x, o.y, o.z], [d.x, d.y, d.z])
    }

    /// A screen position as pixels of the picture.
    fn picture_xy(pos: Pos2, rect: Rect, (pw, ph): (usize, usize)) -> (f64, f64) {
        let fx = (pos.x - rect.min.x) / rect.width().max(1.0);
        let fy = (pos.y - rect.min.y) / rect.height().max(1.0);
        (f64::from(fx) * pw as f64, f64::from(fy) * ph as f64)
    }

    /// A model point on the ground, as a screen position.
    fn screen_of(&self, p: P2, rect: Rect, (pw, ph): (usize, usize)) -> Option<Pos2> {
        let (x, y, _) = self.camera.to_screen(Vec3::new(p.x, p.y, 0.0), pw as f64, ph as f64, self.scene_radius)?;
        Some(Pos2::new(rect.min.x + (x / pw as f64) as f32 * rect.width(), rect.min.y + (y / ph as f64) as f32 * rect.height()))
    }

    fn redraw_picture(&mut self, ctx: &egui::Context, (pw, ph): (usize, usize)) {
        match solidcraft_engine::view::scene(&self.session, &self.camera) {
            Ok(scene) => {
                self.scene_radius = scene.radius.max(1.0);
                let canvas = render(&scene, &self.camera, pw, ph);
                let image = egui::ColorImage::from_rgba_unmultiplied([canvas.w, canvas.h], &canvas.rgba);
                match &mut self.texture {
                    Some(t) => t.set(image, TextureOptions::LINEAR),
                    None => self.texture = Some(ctx.load_texture("solidsketch_view", image, TextureOptions::LINEAR)),
                }
            }
            Err(e) => self.fail(format!("Could not draw the model: {e}")),
        }
        self.stale = false;
    }

    /// Keyboard input: a letter picks a tool, anything else is typed into the value box, Enter
    /// finishes, Escape cancels, Ctrl+Z undoes.
    fn keys(&mut self, ui: &egui::Ui) {
        let events = ui.input(|i| i.events.clone());
        for event in events {
            match event {
                egui::Event::Text(text) => {
                    for c in text.chars() {
                        match Tool::from_key(c) {
                            Some(tool) => self.pick(tool),
                            None => {
                                let out = self.modeler.type_char(c);
                                self.readout = out.readout;
                            }
                        }
                    }
                }
                egui::Event::Key { key, pressed: true, modifiers, .. } => match key {
                    Key::Z if modifiers.command => self.undo(),
                    Key::Enter => self.enter(),
                    Key::Escape => {
                        let out = self.modeler.escape();
                        self.preview = out.preview;
                        self.readout = out.readout;
                    }
                    Key::Backspace => {
                        let out = self.modeler.backspace();
                        self.readout = out.readout;
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }

    fn enter(&mut self) {
        if self.modeler.is_pulling() {
            self.finish_pull();
            return;
        }
        let out = self.modeler.enter();
        self.apply(out);
    }

    /// Takes what a drawing event produced: runs its commands and shows its preview and errors.
    fn apply(&mut self, out: solidsketch_tools::Output) {
        self.preview = out.preview;
        self.readout = out.readout;
        if let Some(e) = &out.error {
            self.fail(Self::explain(e));
        }
        if out.commands.is_empty() {
            return;
        }
        if self.run(&out.commands) {
            if self.modeler.can_pull() {
                self.say("Shape drawn. Press P, then drag it up (or type a height and press Enter).");
            } else {
                self.say("Done.");
            }
        }
    }

    fn finish_pull(&mut self) {
        match self.modeler.release_pull() {
            Ok(commands) => {
                if self.run(&commands) {
                    self.say("Pulled. Pull again to change it, or draw another shape.");
                }
                self.readout.clear();
            }
            Err(e) => {
                // The gesture stays alive so a typed height can fix it; a plain release cancels it.
                self.fail(Self::explain(&e));
                if e == ToolError::ZeroSize && self.readout.is_empty() {
                    self.modeler.cancel_pull();
                }
            }
        }
    }

    /// Pointer input over the viewport.
    fn pointer(&mut self, ui: &egui::Ui, response: &egui::Response, rect: Rect, size: (usize, usize)) {
        let (_, ph) = size;
        let shift = ui.input(|i| i.modifiers.shift);

        // Looking around works with every tool: middle drag orbits (shift pans), the wheel zooms.
        if response.dragged_by(PointerButton::Middle) || (self.tool == Tool::Orbit && response.dragged_by(PointerButton::Primary)) {
            let d = response.drag_delta();
            if shift {
                self.camera.pan(f64::from(d.x), f64::from(d.y), rect.height() as f64);
            } else {
                self.camera.orbit(f64::from(d.x), f64::from(d.y));
            }
            self.stale = true;
        }
        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                self.camera.zoom_at((-f64::from(scroll) * 0.002).exp(), None);
                self.stale = true;
            }
        }

        match self.tool {
            Tool::Line | Tool::Rectangle | Tool::Circle => {
                if let Some(pos) = response.hover_pos() {
                    if let Some(p) = self.ground_at(pos, rect, size) {
                        let out = self.modeler.hover(p);
                        self.preview = out.preview;
                        if !out.readout.is_empty() || self.readout.is_empty() {
                            self.readout = out.readout;
                        }
                    }
                    if response.clicked_by(PointerButton::Primary)
                        && let Some(p) = self.ground_at(pos, rect, size)
                    {
                        let out = self.modeler.click(p);
                        self.apply(out);
                    }
                }
            }
            Tool::PushPull => {
                if response.drag_started_by(PointerButton::Primary) {
                    if self.modeler.begin_pull() {
                        self.pull_origin_y = response.interact_pointer_pos().map_or(0.0, |p| p.y);
                        self.say("Drag up to pull out, down to push in. Or type a height and press Enter.");
                    } else {
                        self.fail(Self::explain(&ToolError::NoFace));
                    }
                }
                if response.dragged_by(PointerButton::Primary)
                    && self.modeler.is_pulling()
                    && let Some(pos) = response.interact_pointer_pos()
                {
                    // Screen points to picture pixels, then to model units along Z.
                    let dy = f64::from(pos.y - self.pull_origin_y) * ph as f64 / f64::from(rect.height().max(1.0));
                    let per_pixel = 2.0 * self.camera.half_height() / ph as f64;
                    self.modeler.drag_pull(view::pull_distance(dy, per_pixel, self.camera.pitch));
                    self.readout = self.modeler.pull_readout();
                }
                if response.drag_stopped() && self.modeler.is_pulling() {
                    self.finish_pull();
                }
            }
            _ => {}
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for tool in Tool::ALL {
                let selected = self.tool == tool;
                let label = if tool.key() == ' ' { tool.label().to_string() } else { format!("{} ({})", tool.label(), tool.key()) };
                let button = egui::Button::new(label).fill(if selected { ACCENT.gamma_multiply(0.35) } else { Color32::TRANSPARENT });
                let mut response = ui.add_enabled(works(tool), button);
                if !works(tool) {
                    response = response.on_disabled_hover_text("Coming soon");
                }
                if response.clicked() {
                    self.pick(tool);
                }
            }
            ui.separator();
            if ui.button("Undo (Ctrl+Z)").clicked() {
                self.undo();
            }
            if ui.button("New").clicked() {
                self.new_model();
            }
        });
    }

    fn overlay(&self, painter: &egui::Painter, rect: Rect, size: (usize, usize)) {
        let stroke = Stroke::new(2.0, ACCENT);
        let line = |points: &[P2], closed: bool| {
            let screen: Vec<Pos2> = points.iter().filter_map(|p| self.screen_of(*p, rect, size)).collect();
            for pair in screen.windows(2) {
                painter.line_segment([pair[0], pair[1]], stroke);
            }
            if closed && screen.len() > 2 {
                painter.line_segment([screen[screen.len() - 1], screen[0]], stroke);
            }
        };
        match &self.preview {
            Preview::None => {}
            Preview::Chain(points) => line(points, false),
            Preview::Rect { a, b } => line(&[*a, P2::new(b.x, a.y), *b, P2::new(a.x, b.y)], true),
            Preview::Circle { center, radius } => {
                let ring: Vec<P2> = (0..=48)
                    .map(|i| {
                        let t = f64::from(i) / 48.0 * std::f64::consts::TAU;
                        P2::new(center.x + radius * t.cos(), center.y + radius * t.sin())
                    })
                    .collect();
                line(&ring, false);
            }
        }
    }

    fn status_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let color = if self.status_is_error { Color32::from_rgb(255, 120, 100) } else { ui.visuals().text_color() };
            ui.colored_label(color, &self.status);
            if !self.readout.is_empty() {
                ui.separator();
                ui.strong(format!("Size: {}", self.readout));
            }
        });
    }
}

impl eframe::App for SketchApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.keys(ui);
        self.toolbar(ui);

        let available = ui.available_size();
        let canvas_size = egui::vec2(available.x.max(64.0), (available.y - 30.0).max(64.0));
        let (rect, response) = ui.allocate_exact_size(canvas_size, Sense::click_and_drag());

        // The picture is drawn at most `MAX_PICTURE` on its longest side.
        let ppp = ui.ctx().pixels_per_point();
        let scale = (MAX_PICTURE / rect.width().max(rect.height())).min(ppp);
        let size = (((rect.width() * scale) as usize).max(1), ((rect.height() * scale) as usize).max(1));
        if size != self.picture_size {
            self.picture_size = size;
            self.stale = true;
        }

        self.pointer(ui, &response, rect, size);
        if self.stale {
            self.redraw_picture(ui.ctx(), size);
        }

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::from_rgb(31, 41, 51));
        if let Some(texture) = &self.texture {
            painter.image(texture.id(), rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        }
        self.overlay(&painter, rect, size);
        if !self.readout.is_empty()
            && let Some(pos) = response.hover_pos()
        {
            painter.text(pos + egui::vec2(16.0, 16.0), Align2::LEFT_TOP, &self.readout, FontId::proportional(15.0), ACCENT);
        }

        self.status_bar(ui);
    }
}

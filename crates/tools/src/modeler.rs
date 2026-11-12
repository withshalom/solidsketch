//! The whole draw-and-pull loop, without a screen: the drawing tools and Push/Pull together, plus
//! what the host would otherwise have to remember between gestures (is a sketch open, which shape
//! is waiting to be pulled, where the last block's far face is).
//!
//! The host feeds it pointer events as model-space points, runs the commands it returns on the
//! SolidCraft session in order, and draws the previews. Pulling takes the shape just drawn if there
//! is one, otherwise the far face of the last block. Picking any face on screen is the engine's
//! `view.pick` and comes later; this is the loop that makes a first block testable end to end.

use crate::command::{Command, FaceRef, ToolError};
use crate::draw::{Drawing, Event, Output};
use crate::geom::P2;
use crate::pushpull::PushPull;
use crate::tool::Tool;

/// Where the drawing tools draw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DrawPlane {
    /// The ground: shapes pull up into new bodies.
    Ground,
    /// A face of a body, named by a point on it: shapes pull out as bosses or in as pockets.
    Face([f64; 3]),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pulling {
    Profile,
    FaceProfile,
    FarFace,
}

/// The face Push/Pull took last: a point on it, and which way its outside points along Z.
#[derive(Clone, Copy, Debug, PartialEq)]
struct FarFace {
    at: [f64; 3],
    /// `1.0` when the outside points up, `-1.0` when it points down.
    outward: f64,
}

/// The drawing tools and Push/Pull working together.
#[derive(Clone, Debug)]
pub struct Modeler {
    drawing: Drawing,
    pull: PushPull,
    pulling: Option<Pulling>,
    /// A sketch is open in the engine (something was drawn and not yet pulled).
    sketch_open: bool,
    /// A point inside the closed shape waiting to be pulled.
    profile: Option<P2>,
    far_face: Option<FarFace>,
    plane: DrawPlane,
    /// The plane the open sketch was made on.
    sketch_plane: DrawPlane,
}

impl Default for Modeler {
    fn default() -> Self {
        Self::new()
    }
}

impl Modeler {
    pub fn new() -> Self {
        Self {
            drawing: Drawing::new(Tool::Rectangle),
            pull: PushPull::new(),
            pulling: None,
            sketch_open: false,
            profile: None,
            far_face: None,
            plane: DrawPlane::Ground,
            sketch_plane: DrawPlane::Ground,
        }
    }

    pub fn tool(&self) -> Tool {
        self.drawing.tool()
    }

    /// Picks a tool; anything half drawn or half pulled is dropped.
    pub fn set_tool(&mut self, tool: Tool) {
        self.cancel_pull();
        self.drawing.set_tool(tool);
    }

    /// Where the next sketch is drawn. Takes effect when a new sketch opens: a sketch already
    /// open stays on its own plane.
    pub fn set_plane(&mut self, plane: DrawPlane) {
        self.plane = plane;
    }

    /// Forget what the engine holds, after an undo or a command that failed: the next shape opens
    /// a fresh sketch and nothing is waiting to be pulled.
    pub fn forget(&mut self) {
        self.cancel_pull();
        self.drawing.set_tool(self.drawing.tool());
        self.sketch_open = false;
        self.profile = None;
        self.far_face = None;
    }

    pub fn hover(&mut self, p: P2) -> Output {
        self.draw(Event::Hover(p))
    }

    pub fn click(&mut self, p: P2) -> Output {
        self.draw(Event::Click(p))
    }

    /// A typed character goes to whichever gesture is under way.
    pub fn type_char(&mut self, c: char) -> Output {
        if self.pull.is_active() {
            self.pull.push_char(c);
            return self.pull_output();
        }
        self.draw(Event::Char(c))
    }

    pub fn backspace(&mut self) -> Output {
        if self.pull.is_active() {
            self.pull.backspace();
            return self.pull_output();
        }
        self.draw(Event::Backspace)
    }

    /// Enter finishes a typed size. While pulling, the host calls [`Modeler::release_pull`] instead.
    pub fn enter(&mut self) -> Output {
        self.draw(Event::Enter)
    }

    pub fn escape(&mut self) -> Output {
        if self.pull.is_active() {
            self.cancel_pull();
            return self.pull_output();
        }
        self.draw(Event::Escape)
    }

    fn draw(&mut self, event: Event) -> Output {
        let mut out = self.drawing.handle(event);
        if !out.commands.is_empty() {
            let mut commands = Vec::with_capacity(out.commands.len() + 1);
            if !self.sketch_open {
                let opened = match self.plane {
                    DrawPlane::Ground => Ok(Command::create_sketch_ground()),
                    DrawPlane::Face(at) => Command::create_sketch_on_face(&FaceRef::Point(at)),
                };
                match opened {
                    Ok(command) => {
                        commands.push(command);
                        self.sketch_open = true;
                        self.sketch_plane = self.plane;
                    }
                    // A face that cannot be named (not a real point) draws nothing.
                    Err(e) => {
                        out.commands.clear();
                        out.error = Some(e);
                        return out;
                    }
                }
            }
            commands.append(&mut out.commands);
            out.commands = commands;
            // An open polyline is not a profile: nothing to pull until a closed shape is drawn.
            self.profile = self.drawing.last_inside();
        }
        out
    }

    // ---- pulling ----

    /// Something is there to pull: the shape just drawn, else the last block's far face.
    pub fn can_pull(&self) -> bool {
        (self.sketch_open && self.profile.is_some()) || self.far_face.is_some()
    }

    /// The user pressed to pull. Returns `false` when there is nothing to pull yet.
    pub fn begin_pull(&mut self) -> bool {
        if self.sketch_open && self.profile.is_some() {
            match self.sketch_plane {
                DrawPlane::Ground => {
                    self.pull.begin_profile();
                    self.pulling = Some(Pulling::Profile);
                }
                DrawPlane::Face(_) => {
                    self.pull.begin_face_profile();
                    self.pulling = Some(Pulling::FaceProfile);
                }
            }
            true
        } else if let Some(face) = self.far_face {
            self.pull.begin(FaceRef::Point(face.at));
            self.pulling = Some(Pulling::FarFace);
            true
        } else {
            false
        }
    }

    pub fn is_pulling(&self) -> bool {
        self.pull.is_active()
    }

    /// The pointer dragged: `distance` is the pull along Z, up positive. (A face whose outside
    /// points down is pulled out by dragging down: that is worked out here.)
    pub fn drag_pull(&mut self, distance: f64) {
        let outward = match (self.pulling, self.far_face) {
            (Some(Pulling::FarFace), Some(face)) => face.outward,
            _ => 1.0,
        };
        self.pull.drag(distance * outward);
    }

    /// The text for the value box while pulling.
    pub fn pull_readout(&self) -> String {
        self.pull.readout()
    }

    pub fn cancel_pull(&mut self) {
        self.pull.cancel();
        self.pulling = None;
    }

    /// The user let go. On success: the commands to run, and the model moves on (a profile is now
    /// a block; a pulled face has moved). On a slip the gesture stays alive so a typed distance
    /// can fix it.
    pub fn release_pull(&mut self) -> Result<Vec<Command>, ToolError> {
        let Some(target) = self.pulling else { return Err(ToolError::NoFace) };
        let distance = self.pull.distance()?;
        let commands = self.pull.commit()?;
        match target {
            Pulling::Profile => {
                if let Some(p) = self.profile.take() {
                    // Pulled up, the far face is the top; pulled down, the bottom.
                    self.far_face = Some(FarFace { at: [p.x, p.y, distance], outward: if distance < 0.0 { -1.0 } else { 1.0 } });
                }
                self.sketch_open = false;
            }
            Pulling::FaceProfile => {
                if let (Some(p), DrawPlane::Face(base)) = (self.profile.take(), self.sketch_plane) {
                    // A boss ends on a new top; a pocket on a floor, whose outside points up.
                    self.far_face = Some(FarFace { at: [p.x, p.y, base[2] + distance], outward: 1.0 });
                }
                self.sketch_open = false;
            }
            Pulling::FarFace => {
                if let Some(face) = &mut self.far_face {
                    // The face moves along its outside by `distance`.
                    face.at[2] += distance * face.outward;
                }
            }
        }
        self.pulling = None;
        Ok(commands)
    }

    fn pull_output(&self) -> Output {
        Output { preview: crate::draw::Preview::None, readout: self.pull.readout(), commands: Vec::new(), error: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> P2 {
        P2::new(x, y)
    }

    fn ids(out: &Output) -> Vec<&'static str> {
        out.commands.iter().map(|c| c.id).collect()
    }

    fn draw_rect(m: &mut Modeler, a: P2, b: P2) -> Vec<&'static str> {
        m.click(a);
        let out = m.click(b);
        ids(&out)
    }

    #[test]
    fn the_first_shape_opens_a_sketch_and_the_next_does_not() {
        let mut m = Modeler::new();
        assert_eq!(draw_rect(&mut m, p(0.0, 0.0), p(10.0, 10.0)), ["sketch.create", "sketch.rectangle.two_point"]);
        assert_eq!(draw_rect(&mut m, p(20.0, 0.0), p(30.0, 10.0)), ["sketch.rectangle.two_point"]);
    }

    #[test]
    fn nothing_to_pull_at_the_start() {
        let mut m = Modeler::new();
        assert!(!m.can_pull());
        assert!(!m.begin_pull());
        assert_eq!(m.release_pull(), Err(ToolError::NoFace));
    }

    #[test]
    fn a_drawn_rectangle_pulls_into_a_block() {
        let mut m = Modeler::new();
        draw_rect(&mut m, p(0.0, 0.0), p(10.0, 10.0));
        assert!(m.begin_pull());
        m.drag_pull(5.0);
        let cmds = m.release_pull().unwrap();
        assert_eq!(cmds.iter().map(|c| c.id).collect::<Vec<_>>(), ["sketch.finish", "solid.extrude"]);
        assert!(!m.is_pulling());
    }

    #[test]
    fn the_block_can_then_be_pulled_again_from_its_far_face() {
        let mut m = Modeler::new();
        draw_rect(&mut m, p(0.0, 0.0), p(10.0, 10.0));
        m.begin_pull();
        m.drag_pull(5.0);
        m.release_pull().unwrap();
        assert!(m.begin_pull());
        m.drag_pull(3.0);
        let cmds = m.release_pull().unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].id, "solid.offset_face");
        assert_eq!(cmds[0].params, "{\"faces\":[[5,5,5]],\"distance\":3}");
        // The face moved up: the next pull names the new height.
        m.begin_pull();
        m.drag_pull(-2.0);
        assert_eq!(m.release_pull().unwrap()[0].params, "{\"faces\":[[5,5,8]],\"distance\":-2}");
    }

    #[test]
    fn a_new_shape_after_a_block_opens_a_new_sketch_and_is_pulled_first() {
        let mut m = Modeler::new();
        draw_rect(&mut m, p(0.0, 0.0), p(10.0, 10.0));
        m.begin_pull();
        m.drag_pull(5.0);
        m.release_pull().unwrap();
        assert_eq!(draw_rect(&mut m, p(20.0, 0.0), p(30.0, 10.0)), ["sketch.create", "sketch.rectangle.two_point"]);
        m.begin_pull();
        m.drag_pull(2.0);
        assert_eq!(m.release_pull().unwrap()[0].id, "sketch.finish");
    }

    #[test]
    fn a_circle_pulls_from_its_centre() {
        let mut m = Modeler::new();
        m.set_tool(Tool::Circle);
        m.click(p(4.0, 6.0));
        m.click(p(7.0, 6.0));
        m.begin_pull();
        m.drag_pull(2.0);
        m.release_pull().unwrap();
        m.begin_pull();
        m.drag_pull(1.0);
        assert_eq!(m.release_pull().unwrap()[0].params, "{\"faces\":[[4,6,2]],\"distance\":1}");
    }

    #[test]
    fn a_closed_line_shape_can_be_pulled_but_an_open_one_cannot() {
        let mut m = Modeler::new();
        m.set_tool(Tool::Line);
        for q in [p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0)] {
            m.click(q);
        }
        m.enter(); // finishes the open chain
        assert!(!m.can_pull());

        m.set_tool(Tool::Line);
        for q in [p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(0.0, 0.0)] {
            m.click(q);
        }
        assert!(m.can_pull());
    }

    #[test]
    fn a_typed_distance_beats_the_drag_and_a_slip_keeps_the_gesture() {
        let mut m = Modeler::new();
        draw_rect(&mut m, p(0.0, 0.0), p(10.0, 10.0));
        m.begin_pull();
        assert_eq!(m.release_pull(), Err(ToolError::ZeroSize));
        assert!(m.is_pulling());
        m.type_char('1');
        m.type_char('2');
        assert_eq!(m.pull_readout(), "12");
        m.drag_pull(99.0);
        assert_eq!(m.release_pull().unwrap()[1].params, "{\"distance\":12,\"operation\":\"new\"}");
    }

    #[test]
    fn escape_drops_a_pull_but_keeps_the_shape() {
        let mut m = Modeler::new();
        draw_rect(&mut m, p(0.0, 0.0), p(10.0, 10.0));
        m.begin_pull();
        m.escape();
        assert!(!m.is_pulling());
        assert!(m.can_pull());
    }

    #[test]
    fn switching_tools_drops_a_pull_in_flight() {
        let mut m = Modeler::new();
        draw_rect(&mut m, p(0.0, 0.0), p(10.0, 10.0));
        m.begin_pull();
        m.set_tool(Tool::Circle);
        assert!(!m.is_pulling());
        assert_eq!(m.release_pull(), Err(ToolError::NoFace));
    }

    #[test]
    fn forgetting_starts_clean() {
        let mut m = Modeler::new();
        draw_rect(&mut m, p(0.0, 0.0), p(10.0, 10.0));
        m.forget();
        assert!(!m.can_pull());
        assert_eq!(draw_rect(&mut m, p(0.0, 0.0), p(5.0, 5.0)), ["sketch.create", "sketch.rectangle.two_point"]);
    }

    #[test]
    fn typed_sizes_work_through_the_modeler() {
        let mut m = Modeler::new();
        m.click(p(0.0, 0.0));
        m.hover(p(5.0, 5.0));
        for c in "10,20".chars() {
            m.type_char(c);
        }
        let out = m.enter();
        assert_eq!(ids(&out), ["sketch.create", "sketch.rectangle.two_point"]);
        // Centre of a 10 by 20 rectangle from the origin.
        m.begin_pull();
        m.drag_pull(1.0);
        m.release_pull().unwrap();
        m.begin_pull();
        m.drag_pull(1.0);
        assert_eq!(m.release_pull().unwrap()[0].params, "{\"faces\":[[5,10,1]],\"distance\":1}");
    }

    #[test]
    fn hostile_input_makes_no_commands() {
        let mut m = Modeler::new();
        let out = m.click(p(f64::NAN, 0.0));
        assert!(out.commands.is_empty());
        assert!(!m.can_pull());
        m.begin_pull();
        m.drag_pull(f64::NAN);
        assert_eq!(m.release_pull(), Err(ToolError::NoFace));
    }

    #[test]
    fn every_command_the_modeler_makes_is_on_the_engine_list() {
        use crate::command::ENGINE_COMMANDS;
        let mut m = Modeler::new();
        let mut all = draw_rect(&mut m, p(0.0, 0.0), p(10.0, 10.0));
        m.begin_pull();
        m.drag_pull(4.0);
        all.extend(m.release_pull().unwrap().iter().map(|c| c.id));
        m.begin_pull();
        m.drag_pull(1.0);
        all.extend(m.release_pull().unwrap().iter().map(|c| c.id));
        for id in all {
            assert!(ENGINE_COMMANDS.contains(&id), "{id} is not listed");
        }
    }

    #[test]
    fn pulling_a_block_down_then_further_down_sends_a_positive_offset() {
        let mut m = Modeler::new();
        draw_rect(&mut m, p(0.0, 0.0), p(10.0, 10.0));
        m.begin_pull();
        m.drag_pull(-5.0);
        let cmds = m.release_pull().unwrap();
        assert_eq!(cmds[1].params, "{\"distance\":-5,\"operation\":\"new\"}");
        // The block hangs below the ground; its bottom face looks down. Dragging down pulls it out.
        m.begin_pull();
        m.drag_pull(-2.0);
        assert_eq!(m.release_pull().unwrap()[0].params, "{\"faces\":[[5,5,-5]],\"distance\":2}");
        m.begin_pull();
        m.drag_pull(1.0);
        assert_eq!(m.release_pull().unwrap()[0].params, "{\"faces\":[[5,5,-7]],\"distance\":-1}");
    }

    #[test]
    fn drawing_on_a_face_opens_a_face_sketch_and_pulls_a_boss() {
        let mut m = Modeler::new();
        m.set_plane(DrawPlane::Face([5.0, 5.0, 20.0]));
        let out = {
            m.click(p(10.0, 10.0));
            m.click(p(30.0, 20.0))
        };
        assert_eq!(out.commands[0].params, "{\"plane\":{\"face\":[5,5,20]}}");
        m.begin_pull();
        m.drag_pull(5.0);
        let cmds = m.release_pull().unwrap();
        assert_eq!(cmds[1].params, "{\"distance\":5,\"operation\":\"join\"}");
        // The boss's top is at 25, centred on the rectangle.
        m.begin_pull();
        m.drag_pull(1.0);
        assert_eq!(m.release_pull().unwrap()[0].params, "{\"faces\":[[20,15,25]],\"distance\":1}");
    }

    #[test]
    fn pushing_a_face_profile_in_cuts_a_pocket_whose_floor_can_be_pulled() {
        let mut m = Modeler::new();
        m.set_plane(DrawPlane::Face([5.0, 5.0, 20.0]));
        draw_rect(&mut m, p(10.0, 10.0), p(30.0, 20.0));
        m.begin_pull();
        m.drag_pull(-3.0);
        let cmds = m.release_pull().unwrap();
        assert_eq!(cmds[1].params, "{\"distance\":3,\"direction\":\"negative\",\"operation\":\"cut\"}");
        m.begin_pull();
        m.drag_pull(-1.0);
        assert_eq!(m.release_pull().unwrap()[0].params, "{\"faces\":[[20,15,17]],\"distance\":-1}");
    }

    #[test]
    fn an_open_sketch_keeps_its_plane_when_the_plane_changes() {
        let mut m = Modeler::new();
        draw_rect(&mut m, p(0.0, 0.0), p(10.0, 10.0));
        m.set_plane(DrawPlane::Face([1.0, 1.0, 1.0]));
        m.begin_pull();
        m.drag_pull(2.0);
        assert_eq!(m.release_pull().unwrap()[1].params, "{\"distance\":2,\"operation\":\"new\"}");
    }

    #[test]
    fn a_face_that_is_not_a_real_point_draws_nothing() {
        let mut m = Modeler::new();
        m.set_plane(DrawPlane::Face([f64::NAN, 0.0, 0.0]));
        m.click(p(0.0, 0.0));
        let out = m.click(p(5.0, 5.0));
        assert!(out.commands.is_empty());
        assert_eq!(out.error, Some(ToolError::NotFinite));
        assert!(!m.can_pull());
    }
}

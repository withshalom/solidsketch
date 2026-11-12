//! The drawing tools on a flat plane: Line, Rectangle and Circle.
//!
//! The host turns the mouse into [`Event`]s (already snapped to whatever the viewport infers:
//! endpoints, midpoints, axes) and draws the [`Preview`] it gets back. A finished shape comes out
//! as engine [`Command`]s. Nothing here knows about the screen or the kernel.

use crate::command::{Command, ToolError};
use crate::geom::P2;
use crate::tool::Tool;
use crate::vcb::{ValueBox, fmt_len};

/// A click this near the first point of a line chain closes it.
pub const CLOSE_DISTANCE: f64 = 1e-6;

/// What the user did.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// The pointer moved to this point.
    Hover(P2),
    /// The pointer was clicked at this point.
    Click(P2),
    /// A key was typed (goes to the value box).
    Char(char),
    Backspace,
    Enter,
    Escape,
}

/// The shape to draw while the user is still deciding.
#[derive(Clone, Debug, PartialEq)]
pub enum Preview {
    None,
    /// A chain of segments, ending at the pointer.
    Chain(Vec<P2>),
    Rect {
        a: P2,
        b: P2,
    },
    Circle {
        center: P2,
        radius: f64,
    },
}

/// What the host should do after an event.
#[derive(Clone, Debug, PartialEq)]
pub struct Output {
    pub preview: Preview,
    /// The text for the value box next to the pointer: what was typed, or the live size.
    pub readout: String,
    /// Commands to run on the engine, in order. Usually none or one.
    pub commands: Vec<Command>,
    /// Set when the event could not be used, so the host can say so in the status bar.
    pub error: Option<ToolError>,
}

/// One drawing tool's state between events.
#[derive(Clone, Debug)]
pub struct Drawing {
    tool: Tool,
    points: Vec<P2>,
    cursor: Option<P2>,
    vcb: ValueBox,
    inside: Option<P2>,
}

fn midpoint(a: P2, b: P2) -> P2 {
    P2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0)
}

fn mean(points: &[P2]) -> Option<P2> {
    if points.is_empty() {
        return None;
    }
    let n = points.len() as f64;
    Some(P2::new(points.iter().map(|p| p.x).sum::<f64>() / n, points.iter().map(|p| p.y).sum::<f64>() / n))
}

impl Drawing {
    pub fn new(tool: Tool) -> Self {
        Self { tool, points: Vec::new(), cursor: None, vcb: ValueBox::new(), inside: None }
    }

    pub fn tool(&self) -> Tool {
        self.tool
    }

    /// Switches tool and drops anything half drawn.
    pub fn set_tool(&mut self, tool: Tool) {
        *self = Self::new(tool);
    }

    /// A shape has been started and not finished.
    pub fn is_drawing(&self) -> bool {
        !self.points.is_empty()
    }

    /// A point inside the last closed shape this tool finished (a rectangle's centre, a circle's
    /// centre, a polygon's vertex average), or `None` if the last thing finished was not a closed
    /// shape. The host uses it to say which profile Push/Pull should take.
    pub fn last_inside(&self) -> Option<P2> {
        self.inside
    }

    pub fn handle(&mut self, event: Event) -> Output {
        let mut commands = Vec::new();
        let mut error = None;
        if matches!(self.tool, Tool::Line | Tool::Rectangle | Tool::Circle) {
            match event {
                Event::Hover(p) => {
                    if p.is_finite() {
                        self.cursor = Some(p);
                    }
                }
                Event::Click(p) if p.is_finite() => {
                    self.cursor = Some(p);
                    self.click(p, &mut commands, &mut error);
                }
                Event::Click(_) => error = Some(ToolError::NotFinite),
                Event::Char(c) => {
                    self.vcb.push(c);
                }
                Event::Backspace => self.vcb.backspace(),
                Event::Enter => self.enter(&mut commands, &mut error),
                Event::Escape => self.cancel(),
            }
        }
        Output { preview: self.preview(), readout: self.readout(), commands, error }
    }

    fn cancel(&mut self) {
        self.points.clear();
        self.vcb.clear();
    }

    fn finish(&mut self, made: Result<Command, ToolError>, inside: Option<P2>, commands: &mut Vec<Command>, error: &mut Option<ToolError>) {
        match made {
            Ok(c) => {
                commands.push(c);
                self.inside = inside;
                self.cancel();
            }
            // A slip (a flat rectangle, say) leaves the shape open so the next click can fix it.
            Err(e) => *error = Some(e),
        }
    }

    fn click(&mut self, p: P2, commands: &mut Vec<Command>, error: &mut Option<ToolError>) {
        self.vcb.clear();
        let Some(&anchor) = self.points.first() else {
            self.points.push(p);
            return;
        };
        match self.tool {
            Tool::Line => {
                if self.points.len() >= 3 && p.dist(anchor) <= CLOSE_DISTANCE {
                    let made = Command::line(&self.points, true);
                    let inside = mean(&self.points);
                    self.finish(made, inside, commands, error);
                } else if self.points.last().is_none_or(|last| p.dist(*last) > CLOSE_DISTANCE) {
                    self.points.push(p);
                }
            }
            Tool::Rectangle => {
                let made = Command::rectangle_two_point(anchor, p);
                self.finish(made, Some(midpoint(anchor, p)), commands, error);
            }
            Tool::Circle => {
                let made = Command::circle_center(anchor, anchor.dist(p));
                self.finish(made, Some(anchor), commands, error);
            }
            _ => {}
        }
    }

    fn enter(&mut self, commands: &mut Vec<Command>, error: &mut Option<ToolError>) {
        if self.vcb.is_empty() {
            if self.tool == Tool::Line {
                if self.points.len() >= 2 {
                    let made = Command::line(&self.points, false);
                    self.finish(made, None, commands, error);
                } else {
                    self.cancel();
                }
            }
            return;
        }
        let Some(values) = self.vcb.values() else {
            self.vcb.clear();
            *error = Some(ToolError::BadValue);
            return;
        };
        let (Some(&anchor), Some(&first)) = (self.points.first(), values.first()) else {
            // Typing before the first click has nothing to size.
            self.vcb.clear();
            return;
        };
        match self.tool {
            Tool::Line => {
                let from = self.points.last().copied().unwrap_or(anchor);
                let next = self.cursor.and_then(|c| from.along(c, first));
                match next {
                    Some(p) => {
                        self.points.push(p);
                        self.vcb.clear();
                    }
                    None => {
                        self.vcb.clear();
                        *error = Some(ToolError::NoDirection);
                    }
                }
            }
            Tool::Rectangle => {
                // One number is a square; the pointer's side of the corner says which way it grows.
                let (w, h) = (first, values.get(1).copied().unwrap_or(first));
                let (sx, sy) = match self.cursor {
                    Some(c) => (if c.x < anchor.x { -1.0 } else { 1.0 }, if c.y < anchor.y { -1.0 } else { 1.0 }),
                    None => (1.0, 1.0),
                };
                let other = P2::new(anchor.x + sx * w, anchor.y + sy * h);
                let made = Command::rectangle_two_point(anchor, other);
                self.vcb.clear();
                self.finish(made, Some(midpoint(anchor, other)), commands, error);
            }
            Tool::Circle => {
                let made = Command::circle_center(anchor, first);
                self.vcb.clear();
                self.finish(made, Some(anchor), commands, error);
            }
            _ => {}
        }
    }

    fn preview(&self) -> Preview {
        let Some(&anchor) = self.points.first() else { return Preview::None };
        let cursor = self.cursor;
        match self.tool {
            Tool::Line => {
                let mut chain = self.points.clone();
                chain.extend(cursor);
                Preview::Chain(chain)
            }
            Tool::Rectangle => cursor.map_or(Preview::None, |b| Preview::Rect { a: anchor, b }),
            Tool::Circle => cursor.map_or(Preview::None, |c| Preview::Circle { center: anchor, radius: anchor.dist(c) }),
            _ => Preview::None,
        }
    }

    fn readout(&self) -> String {
        if !self.vcb.as_str().is_empty() {
            return self.vcb.as_str().to_string();
        }
        let (Some(&anchor), Some(cursor)) = (self.points.first(), self.cursor) else { return String::new() };
        match self.tool {
            Tool::Line => fmt_len(self.points.last().copied().unwrap_or(anchor).dist(cursor)),
            Tool::Rectangle => format!("{}, {}", fmt_len((cursor.x - anchor.x).abs()), fmt_len((cursor.y - anchor.y).abs())),
            Tool::Circle => fmt_len(anchor.dist(cursor)),
            _ => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::ENGINE_COMMANDS;

    fn p(x: f64, y: f64) -> P2 {
        P2::new(x, y)
    }

    fn run(d: &mut Drawing, events: &[Event]) -> Vec<Command> {
        events.iter().flat_map(|e| d.handle(*e).commands).collect()
    }

    fn typed(d: &mut Drawing, s: &str) {
        for c in s.chars() {
            d.handle(Event::Char(c));
        }
    }

    #[test]
    fn two_clicks_make_a_rectangle() {
        let mut d = Drawing::new(Tool::Rectangle);
        let cmds = run(&mut d, &[Event::Click(p(0.0, 0.0)), Event::Hover(p(5.0, 5.0)), Event::Click(p(10.0, 20.0))]);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].id, "sketch.rectangle.two_point");
        assert_eq!(cmds[0].params, "{\"p0\":[0,0],\"p1\":[10,20]}");
        assert!(!d.is_drawing());
    }

    #[test]
    fn typed_size_beats_the_pointer_but_keeps_its_side() {
        let mut d = Drawing::new(Tool::Rectangle);
        d.handle(Event::Click(p(0.0, 0.0)));
        d.handle(Event::Hover(p(-3.0, 4.0)));
        typed(&mut d, "4,3");
        let out = d.handle(Event::Enter);
        assert_eq!(out.commands[0].params, "{\"p0\":[0,0],\"p1\":[-4,3]}");
    }

    #[test]
    fn one_typed_number_is_a_square() {
        let mut d = Drawing::new(Tool::Rectangle);
        d.handle(Event::Click(p(1.0, 1.0)));
        typed(&mut d, "6");
        let out = d.handle(Event::Enter);
        assert_eq!(out.commands[0].params, "{\"p0\":[1,1],\"p1\":[7,7]}");
    }

    #[test]
    fn a_flat_rectangle_is_refused_and_can_be_fixed() {
        let mut d = Drawing::new(Tool::Rectangle);
        d.handle(Event::Click(p(0.0, 0.0)));
        let out = d.handle(Event::Click(p(5.0, 0.0)));
        assert_eq!(out.error, Some(ToolError::ZeroSize));
        assert!(out.commands.is_empty());
        assert!(d.is_drawing());
        let out = d.handle(Event::Click(p(5.0, 5.0)));
        assert_eq!(out.commands.len(), 1);
    }

    #[test]
    fn circle_takes_its_radius_from_the_second_click() {
        let mut d = Drawing::new(Tool::Circle);
        let cmds = run(&mut d, &[Event::Click(p(2.0, 2.0)), Event::Click(p(5.0, 6.0))]);
        assert_eq!(cmds[0].params, "{\"center\":[2,2],\"radius\":5}");
    }

    #[test]
    fn circle_radius_can_be_typed() {
        let mut d = Drawing::new(Tool::Circle);
        d.handle(Event::Click(p(0.0, 0.0)));
        typed(&mut d, "12.5");
        assert_eq!(d.handle(Event::Hover(p(1.0, 0.0))).readout, "12.5");
        let out = d.handle(Event::Enter);
        assert_eq!(out.commands[0].params, "{\"center\":[0,0],\"radius\":12.5}");
    }

    #[test]
    fn clicking_the_first_point_closes_a_line() {
        let mut d = Drawing::new(Tool::Line);
        let cmds = run(&mut d, &[Event::Click(p(0.0, 0.0)), Event::Click(p(10.0, 0.0)), Event::Click(p(10.0, 10.0)), Event::Click(p(0.0, 0.0))]);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].params, "{\"points\":[[0,0],[10,0],[10,10]],\"closed\":true}");
        assert!(!d.is_drawing());
    }

    #[test]
    fn enter_ends_an_open_line() {
        let mut d = Drawing::new(Tool::Line);
        let cmds = run(&mut d, &[Event::Click(p(0.0, 0.0)), Event::Click(p(10.0, 0.0)), Event::Enter]);
        assert_eq!(cmds[0].params, "{\"points\":[[0,0],[10,0]],\"closed\":false}");
    }

    #[test]
    fn enter_with_one_point_just_drops_it() {
        let mut d = Drawing::new(Tool::Line);
        let cmds = run(&mut d, &[Event::Click(p(0.0, 0.0)), Event::Enter]);
        assert!(cmds.is_empty());
        assert!(!d.is_drawing());
    }

    #[test]
    fn a_typed_length_goes_towards_the_pointer() {
        let mut d = Drawing::new(Tool::Line);
        d.handle(Event::Click(p(0.0, 0.0)));
        d.handle(Event::Hover(p(0.0, 3.0)));
        typed(&mut d, "25");
        d.handle(Event::Enter);
        let out = d.handle(Event::Enter);
        assert_eq!(out.commands[0].params, "{\"points\":[[0,0],[0,25]],\"closed\":false}");
    }

    #[test]
    fn a_typed_length_needs_a_direction() {
        let mut d = Drawing::new(Tool::Line);
        d.handle(Event::Click(p(0.0, 0.0)));
        typed(&mut d, "25");
        assert_eq!(d.handle(Event::Enter).error, Some(ToolError::NoDirection));
    }

    #[test]
    fn nonsense_in_the_box_is_reported_and_cleared() {
        let mut d = Drawing::new(Tool::Circle);
        d.handle(Event::Click(p(0.0, 0.0)));
        typed(&mut d, "1.2.3");
        let out = d.handle(Event::Enter);
        assert_eq!(out.error, Some(ToolError::BadValue));
        // The typed text is gone, so the readout is the live radius again.
        assert_eq!(d.handle(Event::Hover(p(3.0, 4.0))).readout, "5");
    }

    #[test]
    fn escape_throws_the_shape_away() {
        let mut d = Drawing::new(Tool::Rectangle);
        d.handle(Event::Click(p(0.0, 0.0)));
        d.handle(Event::Hover(p(4.0, 4.0)));
        typed(&mut d, "7");
        let out = d.handle(Event::Escape);
        assert_eq!(out.preview, Preview::None);
        assert_eq!(out.readout, "");
        assert!(!d.is_drawing());
    }

    #[test]
    fn the_readout_follows_the_pointer() {
        let mut d = Drawing::new(Tool::Rectangle);
        d.handle(Event::Click(p(0.0, 0.0)));
        assert_eq!(d.handle(Event::Hover(p(4.0, -2.5))).readout, "4, 2.5");
        let mut l = Drawing::new(Tool::Line);
        l.handle(Event::Click(p(0.0, 0.0)));
        assert_eq!(l.handle(Event::Hover(p(3.0, 4.0))).readout, "5");
    }

    #[test]
    fn the_preview_chain_ends_at_the_pointer() {
        let mut d = Drawing::new(Tool::Line);
        d.handle(Event::Click(p(0.0, 0.0)));
        let out = d.handle(Event::Hover(p(1.0, 1.0)));
        assert_eq!(out.preview, Preview::Chain(vec![p(0.0, 0.0), p(1.0, 1.0)]));
    }

    #[test]
    fn switching_tools_drops_half_drawn_shapes() {
        let mut d = Drawing::new(Tool::Line);
        d.handle(Event::Click(p(0.0, 0.0)));
        d.set_tool(Tool::Circle);
        assert!(!d.is_drawing());
        assert_eq!(d.tool(), Tool::Circle);
    }

    #[test]
    fn other_tools_are_left_alone() {
        let mut d = Drawing::new(Tool::Orbit);
        let out = d.handle(Event::Click(p(1.0, 1.0)));
        assert_eq!(out.preview, Preview::None);
        assert!(out.commands.is_empty());
        assert!(!d.is_drawing());
    }

    #[test]
    fn hostile_events_never_panic_or_make_commands() {
        let bad = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e308, -1e308];
        for tool in [Tool::Line, Tool::Rectangle, Tool::Circle] {
            let mut d = Drawing::new(tool);
            for &x in &bad {
                for &y in &bad {
                    d.handle(Event::Hover(p(x, y)));
                    d.handle(Event::Click(p(x, y)));
                    d.handle(Event::Enter);
                }
            }
            // Finite but huge points are legal input; whatever comes out must still be finite JSON.
            for out in [d.handle(Event::Click(p(0.0, 0.0))), d.handle(Event::Click(p(1e308, 1e308)))] {
                for c in out.commands {
                    assert!(!c.params.contains("inf") && !c.params.contains("NaN"), "{}", c.params);
                }
            }
        }
    }

    #[test]
    fn every_command_emitted_is_on_the_engine_list() {
        let mut d = Drawing::new(Tool::Line);
        let mut ids = run(&mut d, &[Event::Click(p(0.0, 0.0)), Event::Click(p(1.0, 0.0)), Event::Enter]);
        d.set_tool(Tool::Rectangle);
        ids.extend(run(&mut d, &[Event::Click(p(0.0, 0.0)), Event::Click(p(1.0, 1.0))]));
        d.set_tool(Tool::Circle);
        ids.extend(run(&mut d, &[Event::Click(p(0.0, 0.0)), Event::Click(p(1.0, 0.0))]));
        assert_eq!(ids.len(), 3);
        for c in ids {
            assert!(ENGINE_COMMANDS.contains(&c.id), "{} is not listed", c.id);
        }
    }
}

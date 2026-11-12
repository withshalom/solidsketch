//! Push/Pull: grab a face and pull it out or push it in, or pull a flat drawing up into a solid.
//!
//! The host picks what is under the pointer and measures how far the pointer has moved along its
//! normal. This type holds the gesture between those calls, takes a typed distance, and makes the
//! engine commands when the user lets go:
//!
//! - a face of a solid becomes `solid.offset_face`;
//! - a drawn profile (a closed shape that is still a sketch) becomes `sketch.finish` then
//!   `solid.extrude`, so pulling a rectangle off the ground gives a block in one gesture.

use crate::command::{Command, FaceRef, ToolError};
use crate::vcb::{ValueBox, fmt_len};

/// What the gesture has hold of.
#[derive(Clone, Debug, PartialEq)]
enum Target {
    Face(FaceRef),
    /// The profile of the sketch that is still open.
    Profile,
}

/// A Push/Pull gesture in progress.
#[derive(Clone, Debug, Default)]
pub struct PushPull {
    target: Option<Target>,
    dragged: f64,
    vcb: ValueBox,
}

impl PushPull {
    pub fn new() -> Self {
        Self::default()
    }

    /// The user pressed on `face`.
    pub fn begin(&mut self, face: FaceRef) {
        self.start(Target::Face(face));
    }

    /// The user pressed on the profile of the sketch just drawn (a flat shape, not yet a solid).
    pub fn begin_profile(&mut self) {
        self.start(Target::Profile);
    }

    fn start(&mut self, target: Target) {
        self.target = Some(target);
        self.dragged = 0.0;
        self.vcb.clear();
    }

    /// The pointer moved: `distance` is how far along the normal, positive out of the body.
    /// A value that is not a number is ignored.
    pub fn drag(&mut self, distance: f64) {
        if distance.is_finite() {
            self.dragged = distance;
        }
    }

    pub fn push_char(&mut self, c: char) {
        self.vcb.push(c);
    }

    pub fn backspace(&mut self) {
        self.vcb.backspace();
    }

    pub fn is_active(&self) -> bool {
        self.target.is_some()
    }

    /// The distance now: what was typed, or else how far the pointer has dragged.
    pub fn distance(&self) -> Result<f64, ToolError> {
        if self.vcb.is_empty() {
            return Ok(self.dragged);
        }
        self.vcb.values().and_then(|v| v.first().copied()).ok_or(ToolError::BadValue)
    }

    /// The text for the value box next to the pointer.
    pub fn readout(&self) -> String {
        if !self.vcb.as_str().is_empty() {
            return self.vcb.as_str().to_string();
        }
        if self.is_active() { fmt_len(self.dragged) } else { String::new() }
    }

    pub fn cancel(&mut self) {
        self.target = None;
        self.dragged = 0.0;
        self.vcb.clear();
    }

    /// The user let go (or pressed Enter): the commands to run, in order. On a slip (nothing
    /// picked, a zero distance, a bad number) the gesture stays alive so the user can fix it; on
    /// success it ends.
    pub fn commit(&mut self) -> Result<Vec<Command>, ToolError> {
        let target = self.target.clone().ok_or(ToolError::NoFace)?;
        let distance = self.distance()?;
        let commands = match target {
            Target::Face(face) => vec![Command::offset_face(&[face], distance)?],
            Target::Profile => vec![Command::finish_sketch(), Command::extrude_new(distance)?],
        };
        self.cancel();
        Ok(commands)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn top() -> FaceRef {
        FaceRef::Point([0.0, 0.0, 10.0])
    }

    /// The one command a face pull makes.
    fn single(made: Result<Vec<Command>, ToolError>) -> Command {
        let mut v = made.unwrap();
        assert_eq!(v.len(), 1);
        v.remove(0)
    }

    #[test]
    fn pulling_a_face_out_makes_an_offset() {
        let mut pp = PushPull::new();
        pp.begin(top());
        pp.drag(2.0);
        pp.drag(7.5);
        let c = single(pp.commit());
        assert_eq!(c.id, "solid.offset_face");
        assert_eq!(c.params, "{\"faces\":[[0,0,10]],\"distance\":7.5}");
        assert!(!pp.is_active());
    }

    #[test]
    fn pushing_in_is_a_negative_distance() {
        let mut pp = PushPull::new();
        pp.begin(top());
        pp.drag(-3.0);
        assert_eq!(single(pp.commit()).params, "{\"faces\":[[0,0,10]],\"distance\":-3}");
    }

    #[test]
    fn a_typed_distance_beats_the_drag() {
        let mut pp = PushPull::new();
        pp.begin(top());
        pp.drag(2.0);
        for c in "40".chars() {
            pp.push_char(c);
        }
        assert_eq!(pp.readout(), "40");
        assert_eq!(single(pp.commit()).params, "{\"faces\":[[0,0,10]],\"distance\":40}");
    }

    #[test]
    fn the_readout_shows_the_drag() {
        let mut pp = PushPull::new();
        assert_eq!(pp.readout(), "");
        pp.begin(top());
        pp.drag(3.456);
        assert_eq!(pp.readout(), "3.46");
    }

    #[test]
    fn letting_go_without_moving_changes_nothing_and_keeps_the_gesture() {
        let mut pp = PushPull::new();
        pp.begin(top());
        assert_eq!(pp.commit(), Err(ToolError::ZeroSize));
        assert!(pp.is_active());
    }

    #[test]
    fn no_face_no_command() {
        let mut pp = PushPull::new();
        pp.drag(5.0);
        assert_eq!(pp.commit(), Err(ToolError::NoFace));
    }

    #[test]
    fn a_bad_typed_number_is_reported() {
        let mut pp = PushPull::new();
        pp.begin(top());
        for c in "1.2.3".chars() {
            pp.push_char(c);
        }
        assert_eq!(pp.commit(), Err(ToolError::BadValue));
        assert!(pp.is_active());
    }

    #[test]
    fn non_finite_drags_are_ignored() {
        let mut pp = PushPull::new();
        pp.begin(top());
        pp.drag(4.0);
        pp.drag(f64::NAN);
        pp.drag(f64::INFINITY);
        assert_eq!(pp.distance(), Ok(4.0));
    }

    #[test]
    fn cancel_forgets_everything() {
        let mut pp = PushPull::new();
        pp.begin(top());
        pp.drag(4.0);
        pp.push_char('9');
        pp.cancel();
        assert!(!pp.is_active());
        assert_eq!(pp.readout(), "");
    }

    #[test]
    fn pulling_a_profile_finishes_the_sketch_then_extrudes() {
        let mut pp = PushPull::new();
        pp.begin_profile();
        pp.drag(12.0);
        let v = pp.commit().unwrap();
        assert_eq!(v.iter().map(|c| c.id).collect::<Vec<_>>(), ["sketch.finish", "solid.extrude"]);
        assert_eq!(v[0].params, "{}");
        assert_eq!(v[1].params, "{\"distance\":12,\"operation\":\"new\"}");
        assert!(!pp.is_active());
    }

    #[test]
    fn a_typed_distance_beats_the_drag_on_a_profile_too() {
        let mut pp = PushPull::new();
        pp.begin_profile();
        pp.drag(2.0);
        for c in "30".chars() {
            pp.push_char(c);
        }
        assert_eq!(pp.commit().unwrap()[1].params, "{\"distance\":30,\"operation\":\"new\"}");
    }

    #[test]
    fn a_profile_pulled_down_extrudes_the_other_way() {
        let mut pp = PushPull::new();
        pp.begin_profile();
        pp.drag(-5.0);
        assert_eq!(pp.commit().unwrap()[1].params, "{\"distance\":-5,\"operation\":\"new\"}");
    }

    #[test]
    fn a_profile_with_no_pull_makes_no_commands_and_keeps_the_gesture() {
        let mut pp = PushPull::new();
        pp.begin_profile();
        assert_eq!(pp.commit(), Err(ToolError::ZeroSize));
        assert!(pp.is_active());
    }

    #[test]
    fn a_bad_typed_number_on_a_profile_is_reported_and_finishes_nothing() {
        let mut pp = PushPull::new();
        pp.begin_profile();
        for c in "1..2".chars() {
            pp.push_char(c);
        }
        assert_eq!(pp.commit(), Err(ToolError::BadValue));
        assert!(pp.is_active());
    }

    #[test]
    fn letters_cannot_be_typed_so_no_exponent_can_overflow_a_profile_pull() {
        let mut pp = PushPull::new();
        pp.begin_profile();
        for c in "1e999".chars() {
            pp.push_char(c);
        }
        // The value box drops the `e`, leaving 1999: a plain, finite distance.
        assert_eq!(pp.commit().unwrap()[1].params, "{\"distance\":1999,\"operation\":\"new\"}");
    }

    #[test]
    fn non_finite_drags_never_reach_a_profile_extrude() {
        let mut pp = PushPull::new();
        pp.begin_profile();
        pp.drag(f64::NAN);
        pp.drag(f64::NEG_INFINITY);
        assert_eq!(pp.commit(), Err(ToolError::ZeroSize));
    }

    #[test]
    fn starting_a_new_gesture_replaces_the_old_target() {
        let mut pp = PushPull::new();
        pp.begin_profile();
        pp.begin(top());
        pp.drag(1.0);
        assert_eq!(single(pp.commit()).id, "solid.offset_face");
    }

    #[test]
    fn the_readout_shows_a_profile_drag() {
        let mut pp = PushPull::new();
        pp.begin_profile();
        pp.drag(3.456);
        assert_eq!(pp.readout(), "3.46");
    }

    #[test]
    fn cancel_forgets_a_profile() {
        let mut pp = PushPull::new();
        pp.begin_profile();
        pp.drag(4.0);
        pp.cancel();
        assert!(!pp.is_active());
        assert_eq!(pp.commit(), Err(ToolError::NoFace));
    }
}

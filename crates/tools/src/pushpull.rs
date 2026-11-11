//! Push/Pull: grab a face and pull it out or push it in.
//!
//! The host picks the face under the pointer and measures how far the pointer has moved along the
//! face's normal. This type holds the gesture between those calls, takes a typed distance, and
//! makes the engine command (`solid.offset_face`) when the user lets go.

use crate::command::{Command, FaceRef, ToolError};
use crate::vcb::{ValueBox, fmt_len};

/// A Push/Pull gesture in progress.
#[derive(Clone, Debug, Default)]
pub struct PushPull {
    face: Option<FaceRef>,
    dragged: f64,
    vcb: ValueBox,
}

impl PushPull {
    pub fn new() -> Self {
        Self::default()
    }

    /// The user pressed on `face`.
    pub fn begin(&mut self, face: FaceRef) {
        self.face = Some(face);
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
        self.face.is_some()
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
        self.face = None;
        self.dragged = 0.0;
        self.vcb.clear();
    }

    /// The user let go (or pressed Enter). On a slip (no face, a zero distance, a bad number) the
    /// gesture stays alive so the user can fix it; on success it ends.
    pub fn commit(&mut self) -> Result<Command, ToolError> {
        let face = self.face.clone().ok_or(ToolError::NoFace)?;
        let command = Command::offset_face(&[face], self.distance()?)?;
        self.cancel();
        Ok(command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn top() -> FaceRef {
        FaceRef::Point([0.0, 0.0, 10.0])
    }

    #[test]
    fn pulling_a_face_out_makes_an_offset() {
        let mut pp = PushPull::new();
        pp.begin(top());
        pp.drag(2.0);
        pp.drag(7.5);
        let c = pp.commit().unwrap();
        assert_eq!(c.id, "solid.offset_face");
        assert_eq!(c.params, "{\"faces\":[[0,0,10]],\"distance\":7.5}");
        assert!(!pp.is_active());
    }

    #[test]
    fn pushing_in_is_a_negative_distance() {
        let mut pp = PushPull::new();
        pp.begin(top());
        pp.drag(-3.0);
        assert_eq!(pp.commit().unwrap().params, "{\"faces\":[[0,0,10]],\"distance\":-3}");
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
        assert_eq!(pp.commit().unwrap().params, "{\"faces\":[[0,0,10]],\"distance\":40}");
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
}

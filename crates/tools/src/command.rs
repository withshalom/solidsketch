//! The engine commands SolidSketch's tools emit.
//!
//! A [`Command`] is a SolidCraft command id plus its parameters as JSON text. The host parses the
//! text and runs `Session::execute(id, &params)`; nothing here depends on the engine, so the ids
//! are checked against the engine's source by `scripts/check-command-ids.sh`.

use crate::geom::P2;

/// Every engine command the tools emit. `scripts/check-command-ids.sh` reads this list and checks
/// each id is registered in the SolidCraft engine, so keep one id per line between the markers.
// BEGIN ENGINE COMMANDS
pub const ENGINE_COMMANDS: &[&str] = &["sketch.line", "sketch.rectangle.two_point", "sketch.circle.center", "solid.extrude", "solid.offset_face"];
// END ENGINE COMMANDS

/// Why a tool could not build a command. Input is never trusted: these are values, not panics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolError {
    /// A coordinate or size is NaN or infinite.
    NotFinite,
    /// A shape needs more points than it was given.
    TooFewPoints,
    /// A size or distance is zero (or too small to be anything but a slip).
    ZeroSize,
    /// No face is picked.
    NoFace,
    /// What was typed in the value box is not a number (or not the right count of numbers).
    BadValue,
    /// A typed length needs a direction, and the pointer is still on the last point.
    NoDirection,
}

/// A SolidCraft command, ready for the engine.
#[derive(Clone, Debug, PartialEq)]
pub struct Command {
    pub id: &'static str,
    /// The parameters, as a JSON object.
    pub params: String,
}

/// How a face is named to the engine: a point on it, or its stable name.
#[derive(Clone, Debug, PartialEq)]
pub enum FaceRef {
    Point([f64; 3]),
    Name(String),
}

/// Smallest size or distance a tool accepts, in model units.
pub const MIN_SIZE: f64 = 1e-9;

fn num(v: f64) -> Result<String, ToolError> {
    if !v.is_finite() {
        return Err(ToolError::NotFinite);
    }
    // `-0` is valid JSON but reads as a slip in a saved design.
    Ok(if v == 0.0 { "0".to_string() } else { format!("{v}") })
}

fn point(p: P2) -> Result<String, ToolError> {
    Ok(format!("[{},{}]", num(p.x)?, num(p.y)?))
}

fn text(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

impl FaceRef {
    fn json(&self) -> Result<String, ToolError> {
        match self {
            FaceRef::Point([x, y, z]) => Ok(format!("[{},{},{}]", num(*x)?, num(*y)?, num(*z)?)),
            FaceRef::Name(n) => Ok(text(n)),
        }
    }
}

impl Command {
    /// A chain of line segments through `points`; `closed` joins the last back to the first.
    pub fn line(points: &[P2], closed: bool) -> Result<Command, ToolError> {
        if points.len() < 2 || (closed && points.len() < 3) {
            return Err(ToolError::TooFewPoints);
        }
        let pts = points.iter().map(|p| point(*p)).collect::<Result<Vec<_>, _>>()?;
        Ok(Command { id: "sketch.line", params: format!("{{\"points\":[{}],\"closed\":{closed}}}", pts.join(",")) })
    }

    /// An axis-aligned rectangle from two opposite corners.
    pub fn rectangle_two_point(p0: P2, p1: P2) -> Result<Command, ToolError> {
        let (a, b) = (point(p0)?, point(p1)?);
        if (p1.x - p0.x).abs() < MIN_SIZE || (p1.y - p0.y).abs() < MIN_SIZE {
            return Err(ToolError::ZeroSize);
        }
        Ok(Command { id: "sketch.rectangle.two_point", params: format!("{{\"p0\":{a},\"p1\":{b}}}") })
    }

    /// A circle from its centre and radius.
    pub fn circle_center(center: P2, radius: f64) -> Result<Command, ToolError> {
        let c = point(center)?;
        let r = num(radius)?;
        if radius < MIN_SIZE {
            return Err(ToolError::ZeroSize);
        }
        Ok(Command { id: "sketch.circle.center", params: format!("{{\"center\":{c},\"radius\":{r}}}") })
    }

    /// Extrudes the open sketch profile by `distance` into a new body.
    pub fn extrude_new(distance: f64) -> Result<Command, ToolError> {
        let d = num(distance)?;
        if distance.abs() < MIN_SIZE {
            return Err(ToolError::ZeroSize);
        }
        Ok(Command { id: "solid.extrude", params: format!("{{\"distance\":{d},\"operation\":\"new\"}}") })
    }

    /// Push/Pull: moves the picked faces along their normal by `distance` (negative pushes in).
    pub fn offset_face(faces: &[FaceRef], distance: f64) -> Result<Command, ToolError> {
        if faces.is_empty() {
            return Err(ToolError::NoFace);
        }
        let list = faces.iter().map(FaceRef::json).collect::<Result<Vec<_>, _>>()?;
        let d = num(distance)?;
        if distance.abs() < MIN_SIZE {
            return Err(ToolError::ZeroSize);
        }
        Ok(Command { id: "solid.offset_face", params: format!("{{\"faces\":[{}],\"distance\":{d}}}", list.join(",")) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_lists_its_points_and_whether_it_closes() {
        let c = Command::line(&[P2::new(0.0, 0.0), P2::new(40.0, 0.0), P2::new(40.0, 30.0)], true);
        assert_eq!(c, Ok(Command { id: "sketch.line", params: "{\"points\":[[0,0],[40,0],[40,30]],\"closed\":true}".into() }));
    }

    #[test]
    fn a_closed_line_needs_three_points() {
        assert_eq!(Command::line(&[P2::new(0.0, 0.0), P2::new(1.0, 0.0)], true), Err(ToolError::TooFewPoints));
        assert_eq!(Command::line(&[P2::new(0.0, 0.0)], false), Err(ToolError::TooFewPoints));
    }

    #[test]
    fn rectangle_matches_the_engines_two_point_form() {
        let c = Command::rectangle_two_point(P2::new(0.0, 0.0), P2::new(10.0, 20.5));
        assert_eq!(c, Ok(Command { id: "sketch.rectangle.two_point", params: "{\"p0\":[0,0],\"p1\":[10,20.5]}".into() }));
    }

    #[test]
    fn flat_rectangles_are_refused() {
        assert_eq!(Command::rectangle_two_point(P2::new(0.0, 0.0), P2::new(10.0, 0.0)), Err(ToolError::ZeroSize));
    }

    #[test]
    fn circle_needs_a_real_radius() {
        assert!(Command::circle_center(P2::new(1.0, 2.0), 3.0).is_ok());
        assert_eq!(Command::circle_center(P2::new(1.0, 2.0), 0.0), Err(ToolError::ZeroSize));
        assert_eq!(Command::circle_center(P2::new(1.0, 2.0), -1.0), Err(ToolError::ZeroSize));
        assert_eq!(Command::circle_center(P2::new(1.0, 2.0), f64::NAN), Err(ToolError::NotFinite));
    }

    #[test]
    fn push_pull_names_faces_by_point_or_name() {
        let c = Command::offset_face(&[FaceRef::Point([1.0, 2.0, 3.0]), FaceRef::Name("Top".into())], -4.5).unwrap();
        assert_eq!(c.id, "solid.offset_face");
        assert_eq!(c.params, "{\"faces\":[[1,2,3],\"Top\"],\"distance\":-4.5}");
    }

    #[test]
    fn face_names_are_escaped() {
        let c = Command::offset_face(&[FaceRef::Name("a\"b\\c\nd".into())], 1.0).unwrap();
        assert_eq!(c.params, "{\"faces\":[\"a\\\"b\\\\c\\nd\"],\"distance\":1}");
    }

    #[test]
    fn push_pull_needs_a_face_and_a_distance() {
        assert_eq!(Command::offset_face(&[], 1.0), Err(ToolError::NoFace));
        assert_eq!(Command::offset_face(&[FaceRef::Name("Top".into())], 0.0), Err(ToolError::ZeroSize));
    }

    #[test]
    fn negative_zero_is_written_as_zero() {
        let c = Command::line(&[P2::new(-0.0, 0.0), P2::new(1.0, 1.0)], false).unwrap();
        assert_eq!(c.params, "{\"points\":[[0,0],[1,1]],\"closed\":false}");
    }

    #[test]
    fn non_finite_input_never_reaches_json() {
        assert_eq!(Command::extrude_new(f64::INFINITY), Err(ToolError::NotFinite));
        assert_eq!(Command::line(&[P2::new(f64::NAN, 0.0), P2::new(1.0, 1.0)], false), Err(ToolError::NotFinite));
    }
}

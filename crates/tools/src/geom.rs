//! Points on the drawing plane (a face, or the ground plane), in model units.

/// A point on the drawing plane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct P2 {
    pub x: f64,
    pub y: f64,
}

impl P2 {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Both coordinates are real numbers (no NaN, no infinity).
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }

    pub fn dist(self, other: P2) -> f64 {
        (other.x - self.x).hypot(other.y - self.y)
    }

    /// The point `len` away from `self`, towards `toward`. `None` when the two coincide, since
    /// there is then no direction to go in.
    pub fn along(self, toward: P2, len: f64) -> Option<P2> {
        let d = self.dist(toward);
        if !(d.is_finite() && d > 0.0 && len.is_finite()) {
            return None;
        }
        Some(P2::new(self.x + (toward.x - self.x) / d * len, self.y + (toward.y - self.y) / d * len))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn along_walks_the_asked_distance() {
        let p = P2::new(0.0, 0.0).along(P2::new(3.0, 4.0), 10.0);
        assert_eq!(p, Some(P2::new(6.0, 8.0)));
    }

    #[test]
    fn along_has_no_direction_from_a_point_to_itself() {
        assert_eq!(P2::new(1.0, 1.0).along(P2::new(1.0, 1.0), 5.0), None);
    }

    #[test]
    fn non_finite_points_are_spotted() {
        assert!(!P2::new(f64::NAN, 0.0).is_finite());
        assert!(!P2::new(0.0, f64::INFINITY).is_finite());
        assert!(P2::new(-2.0, 7.5).is_finite());
    }
}

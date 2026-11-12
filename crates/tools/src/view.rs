//! Pointer maths the host needs and the engine does not: where a viewing ray meets the ground, and
//! how far a vertical pointer drag pulls.

use crate::geom::P2;

/// Farther than this from the origin is not a place a person pointed at: a grazing ray, not a hit.
const MAX_REACH: f64 = 1e9;

/// Where a ray from `origin` along `dir` meets the ground plane (z = 0), or `None` when it never
/// does: parallel to the ground, pointing away from it, grazing it so far off that it is no place a
/// person pointed at, or not made of real numbers.
pub fn ground_hit(origin: [f64; 3], dir: [f64; 3]) -> Option<P2> {
    if !origin.iter().chain(dir.iter()).all(|v| v.is_finite()) || dir[2].abs() < 1e-9 {
        return None;
    }
    let t = -origin[2] / dir[2];
    if t < 0.0 {
        return None;
    }
    let p = P2::new(origin[0] + dir[0] * t, origin[1] + dir[1] * t);
    (p.is_finite() && p.x.abs() <= MAX_REACH && p.y.abs() <= MAX_REACH).then_some(p)
}

/// The pull, in model units along Z, for a pointer that moved `dy_pixels` down the screen (up the
/// screen pulls up). `units_per_pixel` is how much model one screen pixel spans; `pitch` is the
/// camera's elevation in radians, which foreshortens vertical distances on screen. Returns 0 for
/// anything that is not a real number.
pub fn pull_distance(dy_pixels: f64, units_per_pixel: f64, pitch: f64) -> f64 {
    if !(dy_pixels.is_finite() && units_per_pixel.is_finite() && pitch.is_finite()) {
        return 0.0;
    }
    // Looking straight down (or up), vertical movement has no screen length: cap the stretch.
    let along_screen = pitch.cos().abs().max(0.05);
    -dy_pixels * units_per_pixel / along_screen
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ray_down_to_the_ground_lands_under_it() {
        let hit = ground_hit([3.0, 4.0, 10.0], [0.0, 0.0, -1.0]).unwrap();
        assert_eq!((hit.x, hit.y), (3.0, 4.0));
    }

    #[test]
    fn a_slanted_ray_lands_along_its_slope() {
        let hit = ground_hit([0.0, 0.0, 10.0], [1.0, 2.0, -1.0]).unwrap();
        assert_eq!((hit.x, hit.y), (10.0, 20.0));
    }

    #[test]
    fn a_ray_that_never_meets_the_ground_gives_nothing() {
        assert_eq!(ground_hit([0.0, 0.0, 10.0], [1.0, 0.0, 0.0]), None); // parallel
        assert_eq!(ground_hit([0.0, 0.0, 10.0], [0.0, 0.0, 1.0]), None); // pointing away
        assert_eq!(ground_hit([f64::NAN, 0.0, 10.0], [0.0, 0.0, -1.0]), None);
        assert_eq!(ground_hit([0.0, 0.0, 10.0], [0.0, f64::INFINITY, -1.0]), None);
        assert_eq!(ground_hit([0.0, 0.0, 1e300], [1.0, 0.0, -1e-8]), None); // grazes it a light-year off
    }

    #[test]
    fn dragging_up_the_screen_pulls_up() {
        assert!(pull_distance(-10.0, 1.0, 0.0) > 0.0);
        assert!(pull_distance(10.0, 1.0, 0.0) < 0.0);
        assert_eq!(pull_distance(-10.0, 0.5, 0.0), 5.0);
    }

    #[test]
    fn a_tilted_camera_stretches_the_pull() {
        let flat = pull_distance(-10.0, 1.0, 0.0);
        let tilted = pull_distance(-10.0, 1.0, 60f64.to_radians());
        assert!((tilted - 2.0 * flat).abs() < 1e-9);
    }

    #[test]
    fn looking_straight_down_does_not_blow_up() {
        let d = pull_distance(-10.0, 1.0, std::f64::consts::FRAC_PI_2);
        assert!(d.is_finite() && d <= 200.0 + 1e-9);
        assert_eq!(pull_distance(f64::NAN, 1.0, 0.0), 0.0);
        assert_eq!(pull_distance(1.0, f64::INFINITY, 0.0), 0.0);
        assert_eq!(pull_distance(1.0, 1.0, f64::NAN), 0.0);
    }
}

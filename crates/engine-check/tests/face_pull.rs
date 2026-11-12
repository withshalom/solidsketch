//! The riskiest idea in SolidSketch, as a test: draw a shape *inside* a face, split the face by it,
//! then push or pull the new patch.
//!
//! SolidCraft's own tests prove two neighbours of this: splitting a face with lines that run edge to
//! edge, and offsetting one of the halves. Neither covers an island, a patch that sits inside the
//! face with no edge touching the boundary. That is what "draw a rectangle on a face, then pull it"
//! needs, so if it fails here the Phase 3 gesture needs a different engine path.
//!
//! The sketch is on the XY plane and the engine projects its curves onto the picked face, the same
//! way the engine's `split_face_tests.rs` does it.

use serde_json::{Value, json};
use solidcraft_engine::Session;
use solidsketch_tools::{Command, P2};

fn plain(s: &mut Session, id: &str, params: Value) -> Value {
    s.execute(id, &params).unwrap_or_else(|e| panic!("{id} {params}: {e}"))
}

fn run(s: &mut Session, c: &Command) -> Value {
    let params: Value = serde_json::from_str(&c.params).expect("a tool's params are JSON");
    s.execute(c.id, &params).unwrap_or_else(|e| panic!("{} {}: {e}", c.id, c.params))
}

/// Merged face count (what a user sees), raw kernel face count, and volume of body `b`.
///
/// The kernel count is higher: it counts every piece the kernel keeps, such as the sub-faces of a
/// face with a hole in it. The merged count is the one that matches how a person would count.
fn faces_volume(s: &mut Session, b: &str) -> (u64, u64, f64) {
    let m = plain(s, "inspect.measure", json!({ "bodies": [b] }))["bodies"][0].clone();
    let n = |v: &Value| v.as_u64().unwrap_or(0);
    (n(&m["faces"]), n(&m["kernel"]["faces"]), m["volume_mm3"].as_f64().unwrap_or(0.0))
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-6 * b.abs().max(1.0)
}

/// A 40 x 30 x 20 box called `B`, with the sketch `Patch` drawn by `draw` and finished.
fn box_with_patch(draw: impl FnOnce(&mut Session)) -> Session {
    let mut s = Session::default();
    plain(&mut s, "solid.box", json!({"length": 40, "width": 30, "height": 20, "body_name": "B"}));
    plain(&mut s, "sketch.create", json!({"plane": "XY", "name": "Patch"}));
    draw(&mut s);
    plain(&mut s, "sketch.finish", json!({}));
    s
}

const TOP: [f64; 3] = [5.0, 5.0, 20.0]; // on the top face, outside the patch
const PATCH: [f64; 3] = [20.0, 15.0, 20.0]; // on the top face, inside the patch

fn split_top(s: &mut Session) -> (u64, u64, f64) {
    plain(s, "solid.split_face", json!({"faces": [TOP], "body": "B", "sketch": "Patch"}));
    faces_volume(s, "B")
}

#[test]
fn a_rectangle_patch_splits_the_top_face_in_two() {
    let mut s = box_with_patch(|s| {
        run(s, &Command::rectangle_two_point(P2::new(10.0, 10.0), P2::new(30.0, 20.0)).unwrap());
    });
    let (faces, kernel, volume) = split_top(&mut s);
    assert_eq!(faces, 7, "six box faces, the top now in two pieces (kernel count {kernel})");
    assert!(close(volume, 24000.0), "{volume}");
}

#[test]
fn pulling_a_rectangle_patch_out_grows_four_side_walls() {
    let mut s = box_with_patch(|s| {
        run(s, &Command::rectangle_two_point(P2::new(10.0, 10.0), P2::new(30.0, 20.0)).unwrap());
    });
    split_top(&mut s);
    plain(&mut s, "solid.offset_face", json!({"faces": [PATCH], "distance": 5}));
    let (faces, kernel, volume) = faces_volume(&mut s, "B");
    assert!(close(volume, 24000.0 + 20.0 * 10.0 * 5.0), "a 20 x 10 block 5 high on top: {volume}");
    assert_eq!(faces, 7 + 4, "the raised patch gains four walls (kernel count {kernel})");
}

#[test]
fn pushing_a_rectangle_patch_in_cuts_a_pocket() {
    let mut s = box_with_patch(|s| {
        run(s, &Command::rectangle_two_point(P2::new(10.0, 10.0), P2::new(30.0, 20.0)).unwrap());
    });
    split_top(&mut s);
    plain(&mut s, "solid.offset_face", json!({"faces": [PATCH], "distance": -3}));
    let (faces, kernel, volume) = faces_volume(&mut s, "B");
    assert!(close(volume, 24000.0 - 20.0 * 10.0 * 3.0), "a 20 x 10 pocket 3 deep: {volume}");
    assert_eq!(faces, 7 + 4, "the pocket gains four walls (kernel count {kernel})");
}

#[test]
fn pulling_a_circle_patch_out_grows_a_cylinder() {
    let mut s = box_with_patch(|s| {
        run(s, &Command::circle_center(P2::new(20.0, 15.0), 5.0).unwrap());
    });
    let (faces, _, volume) = split_top(&mut s);
    assert!(faces >= 7, "the circle splits the top: {faces}");
    assert!(close(volume, 24000.0), "{volume}");
    plain(&mut s, "solid.offset_face", json!({"faces": [PATCH], "distance": 4}));
    let (_, _, volume) = faces_volume(&mut s, "B");
    let expect = 24000.0 + std::f64::consts::PI * 25.0 * 4.0;
    assert!((volume - expect).abs() < 1e-3 * expect, "a 5 mm radius, 4 mm tall cylinder on top: {volume} vs {expect}");
}

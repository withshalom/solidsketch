//! The tools' commands, run on the real SolidCraft engine.
//!
//! `crates/tools` only builds command text; these tests are the proof that the engine accepts
//! that text. They follow the engine's own sequence: create a sketch, draw, finish, then extrude
//! or push/pull.

use serde_json::{Value, json};
use solidcraft_engine::Session;
use solidsketch_tools::{Command, FaceRef, P2, PushPull};

fn run(s: &mut Session, c: &Command) -> Value {
    let params: Value = serde_json::from_str(&c.params).expect("a tool's params are JSON");
    s.execute(c.id, &params).unwrap_or_else(|e| panic!("{} {}: {e}", c.id, c.params))
}

fn plain(s: &mut Session, id: &str, params: Value) -> Value {
    s.execute(id, &params).unwrap_or_else(|e| panic!("{id} {params}: {e}"))
}

#[test]
fn a_rectangle_pulled_up_and_then_push_pulled() {
    let mut s = Session::default();
    plain(&mut s, "sketch.create", json!({"plane": "XY"}));
    run(&mut s, &Command::rectangle_two_point(P2::new(0.0, 0.0), P2::new(10.0, 10.0)).unwrap());
    plain(&mut s, "sketch.finish", json!({}));
    run(&mut s, &Command::extrude_new(5.0).unwrap());
    // The block's top face is at z = 5: pull it up by 3.
    run(&mut s, &Command::offset_face(&[FaceRef::Point([5.0, 5.0, 5.0])], 3.0).unwrap());
}

#[test]
fn a_circle_becomes_a_cylinder() {
    let mut s = Session::default();
    plain(&mut s, "sketch.create", json!({"plane": "XY"}));
    run(&mut s, &Command::circle_center(P2::new(0.0, 0.0), 4.0).unwrap());
    plain(&mut s, "sketch.finish", json!({}));
    run(&mut s, &Command::extrude_new(6.0).unwrap());
}

#[test]
fn a_closed_line_chain_becomes_a_prism() {
    let mut s = Session::default();
    plain(&mut s, "sketch.create", json!({"plane": "XY"}));
    let triangle = [P2::new(0.0, 0.0), P2::new(10.0, 0.0), P2::new(5.0, 8.0)];
    run(&mut s, &Command::line(&triangle, true).unwrap());
    plain(&mut s, "sketch.finish", json!({}));
    run(&mut s, &Command::extrude_new(4.0).unwrap());
}

#[test]
fn pulling_a_drawn_profile_makes_a_block_in_one_gesture() {
    let mut s = Session::default();
    plain(&mut s, "sketch.create", json!({"plane": "XY"}));
    run(&mut s, &Command::rectangle_two_point(P2::new(0.0, 0.0), P2::new(10.0, 10.0)).unwrap());
    // Push/Pull on the flat drawing: the tool finishes the sketch, then extrudes it.
    let mut pp = PushPull::new();
    pp.begin_profile();
    pp.drag(5.0);
    for c in pp.commit().unwrap() {
        run(&mut s, &c);
    }
    // The block's top face is at z = 5: pull it up by 3, as a second Push/Pull.
    let mut pp = PushPull::new();
    pp.begin(FaceRef::Point([5.0, 5.0, 5.0]));
    pp.drag(3.0);
    for c in pp.commit().unwrap() {
        run(&mut s, &c);
    }
}

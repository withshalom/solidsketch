//! SolidSketch's tools: pointer events in, SolidCraft engine commands out.
//!
//! This crate is the part of SolidSketch that has no screen. The host (the app) turns mouse and
//! keyboard input into [`draw::Event`]s or Push/Pull calls, draws the previews it gets back, and
//! runs the returned [`Command`]s on a SolidCraft `Session`. Keeping the tools free of any UI
//! toolkit and of the kernel means every behaviour here is a plain test, and the same logic runs
//! on the desktop and in the browser.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod command;
pub mod draw;
pub mod geom;
pub mod modeler;
pub mod pushpull;
pub mod tool;
pub mod vcb;
pub mod view;

pub use command::{Command, ENGINE_COMMANDS, FaceRef, ToolError};
pub use draw::{Drawing, Event, Output, Preview};
pub use geom::P2;
pub use modeler::{DrawPlane, Modeler};
pub use pushpull::PushPull;
pub use tool::Tool;
pub use vcb::ValueBox;

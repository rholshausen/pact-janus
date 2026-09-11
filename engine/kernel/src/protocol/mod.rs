//! The engine protocol (design 2.1, plan task 4.1): frame dispatch and the `consumer-session/*`
//! session lifecycle (`create`/`add-interaction`/`finalise`). Errors are values (spec §10) and
//! sessions are the only resource (spec §7.1) throughout.
//!
//! Not yet implemented — later tasks, not silently dropped: `consumer-session/variants`,
//! `start-transport`, `serve-variant` (4.2/4.3), `verification/*`, `upgrade/*`, `events/*`,
//! `engine/shutdown`. Until variants/transport exist, every added interaction is honestly
//! `not-exercised` at `finalise`.

mod consumer_session;
mod engine;
mod frame;
mod hello;
mod session;

pub use engine::Engine;
pub use frame::{EngineError, RequestFrame, ResponseFrame};

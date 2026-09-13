//! The engine protocol (design 2.1, plan tasks 4.1/4.3/4.4): frame dispatch, the
//! `consumer-session/*` session lifecycle (`create`/`add-interaction`/`variants`/`serve-variant`/
//! `finalise`), and `finalise`'s contract writing (contract-file spec §2.2's honesty rule).
//! Errors are values (spec §10) and sessions are the only resource (spec §7.1) throughout.
//!
//! Not yet implemented — later tasks, not silently dropped: `start-transport` and the
//! passive/emissive exchange loop it would drive (plan task 4.5), `verification/*`, `upgrade/*`,
//! `events/*`, `engine/shutdown`. Until a transport is bound to a session and actually exercises a
//! variant, every added interaction is honestly `not-exercised` at `finalise`, and `contract` is
//! never present.

mod consumer_session;
mod engine;
mod frame;
mod hello;
mod session;

pub use engine::Engine;
pub use frame::{EngineError, RequestFrame, ResponseFrame};

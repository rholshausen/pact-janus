//! The engine protocol (design 2.1, plan tasks 4.1/4.3/4.4/4.5): frame dispatch, the
//! `consumer-session/*` session lifecycle (`create`/`add-interaction`/`variants`/`serve-variant`/
//! `start-transport`/`finalise`), the live passive exchange `start-transport` binds
//! ([`exchange`]), and `finalise`'s contract writing (contract-file spec §2.2's honesty rule).
//! Errors are values (spec §10) and sessions are the only resource (spec §7.1) throughout.
//!
//! Not yet implemented — later tasks, not silently dropped: `verification/*`, `upgrade/*`,
//! `events/*`, `engine/shutdown`. Emissive (message) interactions and routing one inbound request
//! across several concurrently armed interactions (variant-semantics spec §4.1) are also not
//! here — `start-transport` only drives passive HTTP today, one armed exchange at a time per
//! transport instance, which is what a sequential `serve-variant` loop actually needs.

mod consumer_session;
mod engine;
mod exchange;
mod frame;
mod hello;
mod session;

pub use engine::Engine;
pub use frame::{EngineError, RequestFrame, ResponseFrame};

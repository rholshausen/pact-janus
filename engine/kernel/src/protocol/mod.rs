//! The engine protocol (design 2.1, plan tasks 4.1/4.3/4.4/4.5): frame dispatch, the
//! `consumer-session/*` session lifecycle (`create`/`add-interaction`/`variants`/`serve-variant`/
//! `start-transport`/`finalise`), the live passive exchange `start-transport` binds
//! ([`exchange`]), and `finalise`'s contract writing (contract-file spec §2.2's honesty rule).
//! Errors are values (spec §10) and sessions are the only resource (spec §7.1) throughout.
//!
//! Also here: the verification run ([`verification`], plan task 5.1) — the same engine driven the
//! other way, replaying a contract's recorded variants at a provider — and the event streams it
//! reports on ([`events`], spec §9), which is the delivery model every reporting operation shares.
//!
//! Also here: `verification/explain` ([`explain`], plan task 5.5) — one interaction compiled to a
//! plan, from whichever of the three documents a plan is ever compiled from — and `upgrade/pact`,
//! the session-less v1–v4 conversion ([`crate::upgrade`]).
//!
//! Not yet implemented — later tasks, not silently dropped: emissive (message) interactions and routing one inbound request
//! across several concurrently armed interactions (variant-semantics spec §4.1) are also not
//! here — `start-transport` only drives passive HTTP today, one armed exchange at a time per
//! transport instance, which is what a sequential `serve-variant` loop actually needs.

mod consumer_session;
mod engine;
mod events;
mod exchange;
mod explain;
mod frame;
mod hello;
mod session;
mod verification;
mod wire;

pub use engine::Engine;
pub use events::Event;
pub use frame::{EngineError, RequestFrame, ResponseFrame};

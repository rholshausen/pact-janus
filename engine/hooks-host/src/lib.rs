//! The **host side** of hooks (lifecycle-hooks spec §7–§8, plan task 5.3): the configuration
//! loader, and the two implementations that need something the engine does not have.
//!
//! The split is [ADR 0014](../../../Documentation/decisions/0014-hooks-are-resolved-configuration-not-callbacks.md)'s,
//! and it is the reason this crate exists rather than a module in the kernel:
//!
//! - **The loader is where the file system is** ([`loader`]). Between the file a project writes and
//!   the document the engine receives there is exactly one transformation — interpolate `${VAR}`,
//!   inline script sources, resolve relative paths, validate — and doing it here means a run does
//!   not depend on the directory it was started in, the WASM-embedded engine that can read no files
//!   runs the same hooks as the native one, and the document that determined a run's behaviour is
//!   one document that can be attached to a failure report.
//! - **An implementation is an embedding capability** ([`exec`], [`http`], spec §8.5). Spawning a
//!   process and opening a socket are things a native host can do and a kernel that must build for
//!   `wasm32-wasip2` cannot, so they are registered into the engine the way transports are. An
//!   engine that was handed neither refuses a configuration naming them, by name, before the run
//!   starts — never by running anyway.
//!
//! What is deliberately *not* here: anything about what a hook means. Points, ordering, change
//! permission, failure policy and the report are the kernel's (`pact_janus_kernel::hooks`), because
//! they must be identical in every embedding and for every implementation.

pub mod exec;
pub mod http;
pub mod loader;

pub use exec::ExecHooks;
pub use http::HttpHooks;
pub use loader::{LoadError, load, load_document, resolve};

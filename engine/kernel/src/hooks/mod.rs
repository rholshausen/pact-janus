//! The hook system (lifecycle-hooks spec, design 2.7; plan task 5.3): the named places where a
//! run stops and calls out to something only this project knows — sign this request, make this
//! fixture exist, put the provider in this state.
//!
//! What the kernel owns here is the *system*, not the implementations: the point vocabulary and
//! each point's scope, mutable set and default policy ([`points`]); the resolved configuration
//! document and everything it can be wrong about ([`config`]); the context handed out and the
//! result taken back, including which changes are permitted ([`invoke`]); and the report that
//! answers "which hook rewrote this header" by path and never by value ([`report`]).
//!
//! **The implementations live where their capabilities live** (spec §8.5: they are an embedding
//! capability, exactly as component loaders are — ADR 0013). `exec` needs to spawn a process and
//! `http` needs an HTTP client, neither of which belongs in a kernel that builds for every embedding, so
//! both arrive as [`invoke::HookInvoker`]s the embedding registers — the same injection point
//! transports already use, for the same reason (CLAUDE.md's B3). `component` hooks are answered by
//! a registered [`crate::component::HookComponent`]. A configuration naming a kind the embedding
//! did not register fails **before the first exchange** with `hook-unavailable` naming it, because
//! a degraded run that silently skipped a signing hook is worse than no run.
//!
//! **Hooks leave no trace in the contract** (spec §7.4). Nothing in this module reaches
//! [`crate::contract`]: a contract records what the parties agreed, not how one run was performed.
//! The single exception is a variant's `state-unavailable` status, which is a finding about the
//! contract — the provider cannot produce a state the consumer declared — and not a note about a
//! hook.

pub mod config;
pub mod invoke;
pub mod points;
pub mod report;
pub mod runner;
pub mod script;

pub use config::{HookConfig, HookEntry, RunSpec, When};
pub use invoke::{HookFailure, HookInvoker, InvokeResult, Outcome};
pub use points::{Policy, Scope, point};
pub use report::{HookReport, Invocation};
pub use runner::{ConfigError, HookRunner, Occurrence, PointOutcome};
pub use script::ScriptHooks;

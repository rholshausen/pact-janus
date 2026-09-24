//! The subsumption checker (design 2.8, plan task 7.1): `admits(provider) ⊆ admits(consumer)`,
//! walked across a whole contract rather than one node at a time.
//!
//! The division of labour is the one design 2.8 §1 sets out. Shape-language spec §8 owns the
//! alphabet — the identity floor, the three comparability classes, and the rule that `unknown` is
//! never a guess — and this module never second-guesses it: [`compare`] calls a class's procedure
//! at each node and composes the results. Design 2.8 owns the composition (§3.2's two rules), the
//! finding vocabulary (§4), the report (§6) and the text rendering (§6.4), which are what this
//! module implements.
//!
//! What is deliberately **not** here:
//!
//! - **Deriving** a provider shape from types (task 7.3). Recording one from the provider's own
//!   tests is [`Recorder`] (task 7.2), which lives here because the artifact it writes is the one
//!   the walk reads; nothing else about the walk changes by provenance (§2.3).
//! - **The decision.** Design 2.8 §7's policy document lives here ([`SubsumptionPolicy`]) because
//!   its semantics are that specification's, but what a *run* does with one — combining a report
//!   with verification results into the RFC's `can-i-deploy` answer — is [`crate::compatibility`]
//!   (plan task 7.4), which design 2.8 §1 lists as out of its own scope. The checker's job still
//!   ends at the report: [`SubsumptionReport`] carries the `severity` each finding was computed
//!   with (§4.3) precisely so that dispatch never has to re-walk the tree.

mod compare;
mod phrases;
mod policy;
mod provider_shape;
mod record;
mod render;
mod report;
mod select;

pub use crate::contract::Party;
pub use compare::{Verdict, compare};
pub use policy::{Action, Exemption, InteractionRef, SubsumptionPolicy};
pub use provider_shape::{
  FORMAT as PROVIDER_SHAPE_FORMAT, ProviderInteraction, ProviderShape, StateRef, read_provider_shape,
};
pub use record::{Recorder, RecordingPolicy};
pub use render::render;
// The page task 7.4 prints is this block under a decision, so [`crate::compatibility`] composes
// the same lines rather than re-wording them (render's own module docs).
pub(crate) use render::{finding_lines, header};
pub use report::{
  CheckError, FORMAT as REPORT_FORMAT, Finding, InteractionResult, NOT_PUBLISHED, Severity, Side,
  SubsumptionReport, Summary, check,
};

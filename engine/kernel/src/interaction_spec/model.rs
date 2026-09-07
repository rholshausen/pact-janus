//! The interaction-specification document model (contract-file spec §4.1/§5.1/§6, shape-language
//! spec §3.6): what a consumer submits to `consumer-session/add-interaction` (engine protocol
//! spec §8.2), before it has been executed and recorded.
//!
//! It is the contract's interaction record (`contract::Interaction`) minus the evidence that
//! only exists after a test runs: no `selection`, and shapes typed rather than opaque — this is
//! precisely the parsing and validation plan task 3.2 exists to do, ahead of the plan compiler
//! (3.3) that consumes it.

use crate::common::{Requirement, State, Transport};
use crate::shape::ShapeNode;
use std::collections::BTreeMap;

/// One part's slots: slot name -> the shape it holds (shape-language spec §3.6). Which slots a
/// part has is the transport and content components' business, never this model's.
pub type Part = BTreeMap<String, ShapeNode>;

/// One interaction, as authored and submitted, not yet executed.
#[derive(Debug, Clone, PartialEq)]
pub struct InteractionSpec {
  pub description: String,
  pub transport: Option<Transport>,
  /// Provider states with their bindings unresolved (contract-file spec §6): resolution to a
  /// concrete `params` happens per variant, which does not exist until execution (design 2.3).
  pub states: Option<Vec<State>>,
  pub parts: BTreeMap<String, Part>,
  pub requires: Option<Vec<Requirement>>,
}

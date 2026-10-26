//! The plan compiler and interpreter: interaction specs (design 3.2) and the shapes they carry
//! (design 2.2) compiled to plans (design 2.4, `schemas/v1/plan.schema.json`, plan task 3.3), the
//! plans executed against a [`Resolver`] (plan task 3.4), and `explain`'s two text forms
//! (plan-grammar spec §3, plan task 3.6). The v1–v4 matching-rule compiler builds the same plans
//! from outside this module: it is [`crate::legacy::compile`], behind the v1–v4 boundary.

pub mod actions;
mod compile;
pub mod fragment;
mod interpret;
mod model;
mod render;
mod resolve;
mod tokens;
mod value;

pub use actions::{RESERVED_NAMESPACES, is_component_action};
pub use compile::{Assignment, compile, variant_space};
pub use interpret::{
  ActionApplier, Executed, ExecutedKind, Mismatch, NodeResult, Status, execute, execute_with,
  execute_with_content, outcome,
};
pub use model::{
  DocumentKind, GRAMMAR_VERSION, Literal, Node, NodeKind, Plan, from_json, to_json as plan_json,
};
pub use render::{
  executed as render_executed, executed_marked as render_executed_marked, pretty as render_pretty,
  pretty_marked as render_pretty_marked,
};
pub use resolve::{CapturedValues, ContentDetector, Resolver};
pub use tokens::{Token, TokenKind, tokens};
pub use value::{RuntimeValue, member_path, navigate};

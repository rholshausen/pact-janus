//! The plan compiler and interpreter: interaction specs (design 3.2) and the shapes they carry
//! (design 2.2) compiled to plans (design 2.4, `schemas/v0/plan.schema.json`, plan task 3.3), the
//! v1–v4 matching-rule compiler (plan task 3.5), the plans executed against a [`Resolver`] (plan
//! task 3.4), and `explain`'s two text forms (plan-grammar spec §3, plan task 3.6).

mod compile;
mod interpret;
mod legacy;
mod model;
mod render;
mod resolve;
mod value;

pub use compile::{Assignment, compile, variant_space};
pub use interpret::{Executed, ExecutedKind, Mismatch, NodeResult, Status, execute, outcome};
pub use legacy::{
  LegacyRequest, LegacyResponse, compile_interaction as compile_legacy_interaction,
  compile_request as compile_legacy_request, compile_response as compile_legacy_response,
};
pub use model::{DocumentKind, GRAMMAR_VERSION, Literal, Node, NodeKind, Plan};
pub use render::{executed as render_executed, pretty as render_pretty};
pub use resolve::{CapturedValues, Resolver};
pub use value::{RuntimeValue, navigate};

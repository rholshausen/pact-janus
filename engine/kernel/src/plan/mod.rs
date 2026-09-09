//! The plan compiler (plan task 3.3): interaction specs (design 3.2) and the shapes they carry
//! (design 2.2) compiled to plans (design 2.4, `schemas/v0/plan.schema.json`).
//!
//! What is deliberately **not** here: executing a plan and resolving values against a running
//! interaction (plan task 3.4), rendering the text forms of plan-grammar spec §3 (task 3.6), and
//! the v1–v4 matching-rule compiler (task 3.5).

mod compile;
mod model;

pub use compile::{Assignment, compile, variant_space};
pub use model::{DocumentKind, GRAMMAR_VERSION, Literal, Node, NodeKind, Plan};

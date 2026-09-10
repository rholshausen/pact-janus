//! The plan compiler and interpreter: interaction specs (design 3.2) and the shapes they carry
//! (design 2.2) compiled to plans (design 2.4, `schemas/v0/plan.schema.json`, plan task 3.3), and
//! the plans executed against a [`Resolver`] (plan task 3.4).
//!
//! What is deliberately **not** here: `explain`'s text rendering of a plan or an [`Executed`] one
//! (plan-grammar spec §3, task 3.6), and the v1–v4 matching-rule compiler (task 3.5).

mod compile;
mod interpret;
mod model;
mod resolve;
mod value;

pub use compile::{Assignment, compile, variant_space};
pub use interpret::{Executed, ExecutedKind, Mismatch, NodeResult, Status, execute, outcome};
pub use model::{DocumentKind, GRAMMAR_VERSION, Literal, Node, NodeKind, Plan};
pub use resolve::{CapturedValues, Resolver};
pub use value::{RuntimeValue, navigate};

//! The shape language (design 2.2): the document model, and the parser/validator that turns a
//! JSON shape document into it, enforcing well-formedness (spec §5) as it goes.
//!
//! What is deliberately **not** here: compiling a shape into a plan belongs to
//! [`crate::plan`] (plan task 3.3); matching and producing values belong to task 3.4. This
//! module's own job stops at "is this shape tree well-formed, and if not, where and why" — the
//! `interaction-invalid` quality plan task 3.2 exists to deliver — plus, in [`variant_space`],
//! answering "which dimensions does this tree contribute" (spec §6), which the plan compiler
//! needs computed by the same walk it uses.

mod model;
mod parse;
pub(crate) mod path;
pub mod variant_space;

pub use model::{CORE_OPERATORS, CoreShape, Example, ShapeKind, ShapeNode, TemporalKind, ValueKind};
pub use parse::parse;

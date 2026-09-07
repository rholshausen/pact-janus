//! The shape language (design 2.2): the document model, and the parser/validator that turns a
//! JSON shape document into it, enforcing well-formedness (spec §5) as it goes.
//!
//! What is deliberately **not** here: computing a shape's variant space (spec §6) and compiling
//! it into a plan belong to plan task 3.3; matching and producing values belong to 3.4. This
//! module's job stops at "is this shape tree well-formed, and if not, where and why" — the
//! `interaction-invalid` quality plan task 3.2 exists to deliver.

mod model;
mod parse;

pub use model::{CORE_OPERATORS, CoreShape, Example, ShapeKind, ShapeNode, TemporalKind, ValueKind};
pub use parse::parse;

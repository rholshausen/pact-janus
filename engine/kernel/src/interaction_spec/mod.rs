//! The interaction-specification document model: parser and validator (plan task 3.2).

mod error;
mod model;
mod parse;

pub use error::{InteractionSpecError, Problem};
pub use model::{InteractionSpec, Part};
pub use parse::parse;

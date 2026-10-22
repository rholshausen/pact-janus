//! **The v1–v4 compatibility layer: the one part of the kernel that knows HTTP and JSON**, and the body
//! kinds a v1–v4 pact compares by their own rules: form and XML.
//!
//! The kernel's rule is that it knows nothing about HTTP or JSON (AGENTS.md's architecture rules):
//! transports, content and matchers are components, and the plan, the interpreter, shapes,
//! variants and contracts deal in open parts, slots and documents. A v1–v4 pact is the exception
//! by specification, not by this code's choice. It is an HTTP request/response document with JSON
//! matching rules, and reading one, compiling its rules and upgrading it all mean knowing that
//! (kernel-boundary review findings 3 and 6, staged plan S1.11).
//!
//! So that exception lives here, and only here:
//!
//! - [`pact`] reads a v1–v4 pact with `pact_models`, hands its HTTP interactions to the compiler,
//!   and turns its recorded request into the parts a transport sends, generators applied
//!   (ADR 0034);
//! - [`compile`] is the v1–v4 matching-rule compiler (plan-grammar spec §4.4): the single home of
//!   v1–v4 cascading and precedence, and of the HTTP slot names `method`, `path`, `query`,
//!   `headers`, `body` and `status`;
//! - [`upgrade`] converts a v1–v4 pact into a Janus contract (contract-file spec §8);
//! - [`xml`] reads a v1–v4 XML body into the document the built-in XML component decodes one to
//!   (ADR 0039), which [`compile`] walks as today's verifier walks XML.
//!
//! The rest of the kernel calls these, and holds what they return, without naming a `pact_models`
//! type: a request kept for its generators is a [`pact::RequestTemplate`]. `tests/boundary.rs`
//! checks it: outside this module, `pact_models` is used only for its generator vocabulary
//! (`component::generator`, the built-in `pact` generator component) and its date-time format
//! check (`plan::interpret`), neither of which is HTTP or a pact document.
//!
//! A module rather than a crate: the compiler builds the kernel's plan nodes and the verification
//! path calls the compiler, so a crate would need the kernel and the kernel would need it.

pub mod compile;
pub mod pact;
pub mod upgrade;
pub mod xml;

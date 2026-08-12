//! Pact Janus engine kernel.
//!
//! The kernel owns session lifecycle, the plan compiler, the plan interpreter and the pact model.
//! It knows nothing about HTTP, JSON or any other transport/content type — those are components
//! behind the component interfaces (see the component-interface design, plan task 2.6).

/// The engine version, as reported over the engine protocol.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The engine protocol version this kernel speaks.
///
/// Placeholder until the protocol is designed (plan task 2.1) and the IDL chosen (gate G1);
/// version negotiation semantics are part of that design.
pub const PROTOCOL_VERSION: u32 = 0;

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn engine_version_matches_crate_version() {
    assert_eq!(ENGINE_VERSION, env!("CARGO_PKG_VERSION"));
  }
}

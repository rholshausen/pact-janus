//! What the kind `http` declares that is not I/O (ADR 0094): which part answers an exchange, which
//! slots tell its outcomes apart, and which members carry credentials. The transport answers from
//! here, and an embedding with no transport registers it (`Engine::register_kind`), as the engine's
//! WASM component does, so a recording and a redaction read the same declarations in both.

use pact_janus_kernel::component::{ExchangeParts, KindDeclarations, OutcomeSlots};
use pact_janus_kernel::redaction::CredentialMembers;
use std::collections::BTreeMap;

/// The kind the HTTP transport serves.
pub const KIND: &str = "http";

/// Everything below, as the kernel holds it.
pub fn declarations() -> KindDeclarations {
  KindDeclarations {
    exchange_parts: exchange_parts(),
    outcome_slots: outcome_slots(),
    credential_members: credential_members(),
  }
}

/// An exchange starts with a request and is answered with a response (component-interfaces spec
/// §5.7): the names the engine reads instead of spelling them.
pub fn exchange_parts() -> ExchangeParts {
  ExchangeParts {
    initiating: "request".to_string(),
    answering: Some("response".to_string()),
  }
}

/// The credentials HTTP itself defines (ADR 0074 decision 2): `Authorization` and
/// `Proxy-Authorization` (RFC 9110 §11.6.2, §11.7.2) and `Cookie` on a request, `Set-Cookie` on a
/// response (RFC 6265 §4). Header names are lower-case, as the transport presents them.
pub fn credential_members() -> CredentialMembers {
  let headers = |names: &[&str]| {
    BTreeMap::from([(
      "headers".to_string(),
      names.iter().map(|name| name.to_string()).collect::<Vec<_>>(),
    )])
  };
  CredentialMembers::from([
    (
      "request".to_string(),
      headers(&["authorization", "proxy-authorization", "cookie"]),
    ),
    ("response".to_string(), headers(&["set-cookie"])),
  ])
}

/// A response's status tells one answer from another (ADR 0086 decision 4): a 200 with an order
/// and a 404 with an error document are two outcomes of one operation, not one widened answer.
pub fn outcome_slots() -> OutcomeSlots {
  OutcomeSlots::from([("response".to_string(), vec!["status".to_string()])])
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;

  /// The WASM engine registers these for `http`, and the native one asks the transport: the two
  /// must be one answer.
  #[cfg(feature = "transport")]
  #[test]
  fn the_transport_declares_what_the_kind_does() {
    let transport = crate::HttpTransport::new();
    assert_eq!(KindDeclarations::of(&transport), declarations());
  }
}
